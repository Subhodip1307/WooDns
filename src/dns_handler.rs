// this page will responsible all types of dns activities
use crate::loggin::DnsLogger;
use dashmap::DashMap;
use hickory_proto::op::{Message, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::{RData, Record};
use hickory_proto::serialize::binary::{BinDecodable, BinEncodable, BinEncoder};
use std::{collections::HashMap, env, net::SocketAddr, sync::Arc, time::Instant};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep, timeout};

#[derive(Debug, Clone)]
pub enum RemoteReocrds {
    A(std::net::Ipv4Addr),
    // AAAA(std::net::Ipv6Addr),
}

#[derive(Debug, Clone)]
pub struct RemoteDnsCache {
    records: RemoteReocrds,
    ttl: Instant,
}

pub struct DNSManager {
    pub docker_dns: Arc<RwLock<HashMap<String, String>>>,
    pub logger: Arc<DnsLogger>,
    pub remote_dns: Arc<DashMap<String, RemoteDnsCache>>,
}
impl DNSManager {
    pub async fn handle_dns_query(
        &self,
        data: Vec<u8>,
        src: SocketAddr,
        socket: Arc<UdpSocket>,
    ) -> anyhow::Result<()> {
        let request = Message::from_bytes(&data)?;
        let mut response = Message::new();
        response
            .set_id(request.id())
            .set_message_type(MessageType::Response)
            .set_op_code(OpCode::Query)
            .set_authoritative(true)
            .set_recursion_desired(request.recursion_desired())
            .set_recursion_available(false);

        // taking Read permission

        for query in request.queries() {
            response.add_query(query.clone());
            //start checking if the query is "A" type and if data present in local or not
            let data_presence: bool = {
                //check if the query is persent in the docker records (need to improve)
                //if query ends with it may be local docker container query
                if query.name().to_ascii().ends_with("docker.")
                    && query.query_type().to_string() == "A"
                {
                    let data_reader: tokio::sync::RwLockReadGuard<'_, HashMap<String, String>> =
                        self.docker_dns.read().await;
                    data_reader.contains_key(&query.name().to_ascii())
                } else {
                    false
                }
            }; //end checking presence
            if data_presence {
                let data_reader: tokio::sync::RwLockReadGuard<'_, HashMap<String, String>> =
                    self.docker_dns.read().await;
                if let Some(ip_str) = data_reader.get(&query.name().to_ascii()).cloned()
                    && let Ok(ip_address) = ip_str.parse::<std::net::Ipv4Addr>()
                {
                    let record = Record::from_rdata(
                        query.name().clone(),
                        300,
                        RData::A(hickory_proto::rr::rdata::A(ip_address)),
                    );
                    response.add_answer(record);
                    //sending responce after finding the data in local
                    let mut resp_buf: Vec<u8> = Vec::with_capacity(512);
                    let mut encoder: BinEncoder<'_> = BinEncoder::new(&mut resp_buf);
                    response.emit(&mut encoder)?;
                    socket.send_to(&resp_buf, src).await?;
                    self.logger
                        .log(&format!(
                            "Request received for {}, resolved to {}",
                            query.name(),
                            ip_str
                        ))
                        .await;
                    return Ok(());
                }
            } else
            /*if query not found in docker cache then */
            if let Some(entry) = self.remote_dns.get(&query.name().to_ascii())
                && query.query_type().to_string() == "A"
            {
                let RemoteReocrds::A(addr) = entry.value().records;
                let record = Record::from_rdata(
                    query.name().clone(),
                    300,
                    RData::A(hickory_proto::rr::rdata::A(addr)),
                );
                response.add_answer(record);
                //sending responce after finding the data in local
                let mut resp_buf: Vec<u8> = Vec::with_capacity(512);
                let mut encoder: BinEncoder<'_> = BinEncoder::new(&mut resp_buf);
                response.emit(&mut encoder)?;
                socket.send_to(&resp_buf, src).await?;
                self.logger
                    .log(&format!(
                        "Request received for {}, resolved to {}",
                        query.name(),
                        addr
                    ))
                    .await;
                return Ok(());
            } //end checking in the dashmap if data is there data will be send  need to remove duplicate codes
        } //end for loop

        self.logger
            .log("Request not found locally, forwarding to system DNS")
            .await;
        match self.query_remote_dns(&data).await {
            Ok(upstream_bytes) => {
                socket.send_to(&upstream_bytes, src).await?;
            }
            Err(err) => {
                response.set_response_code(ResponseCode::ServFail);
                let mut resp_buffer = Vec::with_capacity(512);
                let mut encoder = BinEncoder::new(&mut resp_buffer);
                response.emit(&mut encoder)?;
                socket.send_to(&resp_buffer, src).await?;
                self.logger
                    .log(&format!("something  Went Wrong, more info:- {:?}", err))
                    .await;
            }
        }
        // end of loop and now data is send
        Ok(())
    }

    async fn query_remote_dns(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        // query the remote dns server
        // println!("querying dns server");
        let mut server = String::from("8.8.8.8:53");
        if let Ok(dns_ip) = env::var("fallback") {
            server = dns_ip
        }
        //sending request to fallback dns
        let upstream_socket = UdpSocket::bind("0.0.0.0:0").await?;
        upstream_socket.send_to(data, server).await?;

        let mut buf = [0u8; 512];
        let recv_result =
            timeout(Duration::from_secs(2), upstream_socket.recv_from(&mut buf)).await;
        match recv_result {
            Ok(Ok((len, _))) => {
                //return the responce after caching it (only A recrods)
                let res: Message = Message::from_bytes(&buf[..len])?;
                if let Some(ans) = res.answers().first()
                    && let Some(RData::A(ip)) = ans.data()
                {
                    let remote_cache = RemoteDnsCache {
                        ttl: Instant::now() + Duration::from_secs(100),
                        records: RemoteReocrds::A(ip.to_string().parse::<std::net::Ipv4Addr>()?),
                    };
                    self.remote_dns.insert(ans.name().to_ascii(), remote_cache);
                }
                Ok(buf[..len].to_vec())
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err(anyhow::anyhow!("DNS request timed out")),
        } //end matche
    }

    // async fn send_dns_responce(&self,response,record,socket)-> anyhow::Result<()>  {
    //     response.add_answer(record);
    //     //sending responce after finding the data in local
    //     let mut resp_buf: Vec<u8> = Vec::with_capacity(512);
    //     let mut encoder: BinEncoder<'_> = BinEncoder::new(&mut resp_buf);
    //     response.emit(&mut encoder)?;
    //     socket.send_to(&resp_buf, src).await?;
    //     self.logger
    //         .log(&format!(
    //             "Request received for {}, resolved to {}",
    //             query.name(),
    //             addr
    //         ))
    //         .await;
    // }
} //end of impl

pub async fn remove_cache(data: Arc<DashMap<String, RemoteDnsCache>>) {
    // this function will remove the remote dns cache
    // will use .retain in future
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
        sleep(Duration::from_secs(5)).await;
    }
}
