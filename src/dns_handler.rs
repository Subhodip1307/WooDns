// this page will responsible all types of dns activities
use crate::loggin::DnsLogger;
use crate::response_handler::ResponseBuilder;
use crate::storage_system::DockerStorage;
use dashmap::DashMap;
use hickory_proto::op::Message;
use hickory_proto::rr::rdata;
use hickory_proto::rr::{RData, Record};
use hickory_proto::serialize::binary::BinDecodable;
use std::{env, net::SocketAddr, sync::Arc, time::Instant};
use tokio::net::UdpSocket;
use tokio::time::{Duration, interval, timeout};

#[derive(Debug, Clone)]
pub enum RemoteReocrds {
    A(std::net::Ipv4Addr),
    Aaaa(std::net::Ipv6Addr),
    Ptr(hickory_proto::rr::Name),
}

#[derive(Debug, Clone)]
pub struct RemoteDnsCache {
    records: RemoteReocrds,
    ttl: Instant,
}

pub struct DNSManager {
    pub docker_dns: Arc<DockerStorage>,
    pub logger: Arc<DnsLogger>,
    pub remote_dns: Arc<DashMap<String, RemoteDnsCache>>,
    responce_builder: ResponseBuilder,
    src: SocketAddr,
    socket: Arc<UdpSocket>,
}

impl DNSManager {
    pub fn new(
        docker: Arc<DockerStorage>,
        log: Arc<DnsLogger>,
        remote: Arc<DashMap<String, RemoteDnsCache>>,
        source: SocketAddr,
        socket_addr: Arc<UdpSocket>,
    ) -> Self {
        Self {
            src: source,
            socket: socket_addr,
            docker_dns: docker,
            logger: log,
            remote_dns: remote,
            responce_builder: ResponseBuilder,
        }
    }

    pub async fn handle_dns_query(&self, data: Vec<u8>) -> anyhow::Result<()> {
        let request = self.responce_builder.deserialize(&data)?;
        //TODO: future: use for..loop instade of just .first()
        if let Some(query) = request.queries().first().cloned() {
            // checking if the query is .ends with docker
            if query.name().to_ascii().ends_with("docker.") {
                return self.docker_resolve(request, query).await;
            } //not ends with .docker

            if let Some(entry) = self.remote_dns.get(&query.name().to_ascii()) {
                let records = &entry.value().records;
                if query.query_type().to_string() == "A"
                    && let RemoteReocrds::A(addr) = records
                {
                    #[cfg(debug_assertions)]
                    println!("Cache used for A");
                    let record = Record::from_rdata(
                        query.name().clone(),
                        300,
                        RData::A(hickory_proto::rr::rdata::A(*addr)),
                    );
                    let response =
                        self.responce_builder
                            .success_response(request, query.clone(), record)?;
                    self.socket.send_to(&response, self.src).await?;
                    return Ok(());
                } else if query.query_type().to_string() == "AAAA"
                    && let RemoteReocrds::Aaaa(addr) = records
                {
                    #[cfg(debug_assertions)]
                    println!("Cache used for AAAA");
                    let record = Record::from_rdata(
                        query.name().clone(),
                        300,
                        RData::AAAA(hickory_proto::rr::rdata::AAAA(*addr)),
                    );
                    let response =
                        self.responce_builder
                            .success_response(request, query.clone(), record)?;
                    self.socket.send_to(&response, self.src).await?;
                    return Ok(());
                }
                //checking for ptr records
                else if query.query_type().to_string() == "PTR"
                    && let RemoteReocrds::Ptr(addr) = records.clone()
                {
                    #[cfg(debug_assertions)]
                    println!("Cache used for PTR");
                    let record = Record::from_rdata(
                        query.name().clone(),
                        300,
                        RData::PTR(hickory_proto::rr::rdata::PTR(addr)),
                    );
                    let response =
                        self.responce_builder
                            .success_response(request, query.clone(), record)?;
                    self.socket.send_to(&response, self.src).await?;
                    return Ok(());
                } //ptr record done
            } //end checking in the dashmap if data is there data will be send  need to remove duplicate codes
            match self.query_remote_dns(&data).await {
                Ok(upstream_bytes) => {
                    self.socket.send_to(&upstream_bytes, self.src).await?;
                }
                Err(err) => {
                    let resp_buffer = self.responce_builder.error_res(request.clone(), query)?;
                    self.socket.send_to(&resp_buffer, self.src).await?;
                    self.logger
                        .log(&format!(
                            "something  Went Wrong, more info:- {:?}, Quries {:?}",
                            err,
                            request.queries()
                        ))
                        .await;
                }
            } //end matche
        }
        Ok(())
    } //handel dns query

    async fn query_remote_dns(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.logger
            .log("Request not found locally, forwarding to system DNS")
            .await;
        // query the remote dns server
        let server = env::var("fallback").unwrap_or(String::from("8.8.8.8:53"));
        //sending request to fallback dns
        let upstream_socket = UdpSocket::bind("0.0.0.0:0").await?;
        upstream_socket.send_to(data, server).await?;

        let mut buf = [0u8; 512];
        let recv_result =
            timeout(Duration::from_secs(5), upstream_socket.recv_from(&mut buf)).await;
        match recv_result {
            Ok(Ok((len, _))) => {
                //return the responce after caching it
                let res: Message = Message::from_bytes(&buf[..len])?;
                if let Some(ans) = res.answers().first() {
                    match ans.data() {
                        Some(RData::A(rdata::A(ipv4_ref))) => {
                            let ip_owned = *ipv4_ref;
                            self.logger
                                .log(&format!(
                                    "A The IP of this {} is this {}",
                                    ans.name().to_ascii(),
                                    ip_owned
                                ))
                                .await;

                            if let Some(mut entry) = self.remote_dns.get_mut(&ans.name().to_ascii())
                            {
                                entry.ttl = Instant::now() + Duration::from_secs(100);
                                entry.records = RemoteReocrds::A(ip_owned);
                            } else {
                                let remote_cache = RemoteDnsCache {
                                    ttl: Instant::now() + Duration::from_secs(100),
                                    records: RemoteReocrds::A(ip_owned),
                                };
                                self.remote_dns.insert(ans.name().to_ascii(), remote_cache);
                            }
                        } //end A
                        Some(RData::AAAA(rdata::AAAA(ipv6_ref))) => {
                            let ip_owned = *ipv6_ref;
                            self.logger
                                .log(&format!(
                                    "AAAA The IP of this {} is this {}",
                                    ans.name().to_ascii(),
                                    ip_owned
                                ))
                                .await;
                            if let Some(mut entry) = self.remote_dns.get_mut(&ans.name().to_ascii())
                            {
                                entry.ttl = Instant::now() + Duration::from_secs(100);
                                entry.records = RemoteReocrds::Aaaa(ip_owned);
                            } else {
                                let remote_cache = RemoteDnsCache {
                                    ttl: Instant::now() + Duration::from_secs(100),
                                    records: RemoteReocrds::Aaaa(ip_owned),
                                };
                                self.remote_dns.insert(ans.name().to_ascii(), remote_cache);
                            }
                        } //end AAAA
                        Some(RData::PTR(rdata::PTR(ip))) => {
                            let real_ip = ip.clone();
                            self.logger
                                .log(&format!(
                                    "PTR The IP of this {} is this {}",
                                    ans.name().to_ascii(),
                                    ip
                                ))
                                .await;
                            if let Some(mut entry) = self.remote_dns.get_mut(&ans.name().to_ascii())
                            {
                                entry.ttl = Instant::now() + Duration::from_secs(100);
                                entry.records = RemoteReocrds::Ptr(real_ip);
                            } else {
                                let remote_cache = RemoteDnsCache {
                                    ttl: Instant::now() + Duration::from_secs(100),
                                    records: RemoteReocrds::Ptr(real_ip),
                                };
                                self.remote_dns.insert(ans.name().to_ascii(), remote_cache);
                            }
                        }
                        _ => {
                            println!(
                                "No Need to Store This Type of Addess {}",
                                ans.name().to_ascii()
                            );
                        }
                    }
                }
                Ok(buf[..len].to_vec())
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err(anyhow::anyhow!("DNS request timed out")),
        } //end matche
    }

    async fn docker_resolve(
        &self,
        request: Message,
        query: hickory_proto::op::query::Query,
    ) -> anyhow::Result<()> //resolve only docker records
    {
        let record_exists: bool = { self.docker_dns.contains(&query.name().to_ascii()).await };
        if !record_exists {
            let resp_buf = self.responce_builder.nxdomain(request, query.clone())?;
            self.socket.send_to(&resp_buf, self.src).await?;
            return Ok(());
        } else if query.query_type().to_string() == "A" {
            // currently docker only supports A records
            if let Some(ip_str) = self.docker_dns.get(&query.name().to_ascii()).await
                && let Ok(ip_address) = ip_str.parse::<std::net::Ipv4Addr>()
            {
                let record = Record::from_rdata(
                    query.name().clone(),
                    300,
                    RData::A(hickory_proto::rr::rdata::A(ip_address)),
                );
                let response =
                    self.responce_builder
                        .success_response(request, query.clone(), record)?;
                self.socket.send_to(&response, self.src).await?;
                self.logger
                    .log(&format!(
                        "Request received for {}, resolved to {}",
                        query.name(),
                        ip_address
                    ))
                    .await;
                return Ok(());
            } //innner if
        } //if to check record type
        // unknown types
        let resp_buf = self
            .responce_builder
            .record_not_found(request, query.clone())?;
        self.socket.send_to(&resp_buf, self.src).await?;
        self.logger
            .log(&format!(
                "Unresolved Docker Query {} & and Query Type is {}",
                query.name(),
                query.query_type()
            ))
            .await;
        Ok(())
    }
} //end of impl

pub async fn remove_cache(data: Arc<DashMap<String, RemoteDnsCache>>) {
    // this function will remove the remote dns cache
    // will use .retain in future
    let mut ticker = interval(Duration::from_secs(5));
    loop {
        let currnet = Instant::now();
        let keys_to_remove: Vec<String> = data
            .iter()
            .filter_map(|f| {
                if f.value().ttl <= currnet {
                    Some(f.key().clone())
                } else {
                    None
                }
            })
            .collect();
        for keys in keys_to_remove {
            data.remove(&keys);
        }
        ticker.tick().await;
    }
}
