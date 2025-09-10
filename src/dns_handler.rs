// this page will responsible all types of dns activities
use crate::loggin::DnsLogger;
use crate::response_handler::ResponseBuilder;
// use response_handler::ResponseBuilder;
use dashmap::DashMap;
use hickory_proto::op::{query, Message, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::rdata::{self, PTR};
use hickory_proto::rr::{RData, Record};
use hickory_proto::serialize::binary::{BinDecodable, BinEncodable, BinEncoder};
use std::{collections::HashMap, env, net::SocketAddr, sync::Arc, time::Instant};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep, timeout};

#[derive(Debug, Clone)]
pub enum RemoteReocrds {
    A(std::net::Ipv4Addr),
    AAAA(std::net::Ipv6Addr),
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
    responce_builder: ResponseBuilder,
    src:SocketAddr,
     socket: Arc<UdpSocket>,
}

impl DNSManager {
    pub fn new(
       docker_dns: Arc<RwLock<HashMap<String, String>>>, logger: Arc<DnsLogger>,remote_dns: Arc<DashMap<String, RemoteDnsCache>>
       ,src:SocketAddr, socket: Arc<UdpSocket>
    )->Self {
        Self { src:src,socket:socket,docker_dns: docker_dns, logger: logger, remote_dns: remote_dns, responce_builder: ResponseBuilder}
    }

    pub async fn handle_dns_query(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<()>{
        let request =  self.responce_builder.deserialize(&data)?;
        //TODO: future: use for..loop instade of just .first()
        if let Some(query)=request.queries().first().cloned(){
            // checking if the query is .ends with docker
            if query.name().to_ascii().ends_with("docker."){
                return self.docker_resolve(request, query).await;
            }//not ends with .docker
            
            if let Some(entry) = self.remote_dns.get(&query.name().to_ascii())
            {
            if query.query_type().to_string()=="A" &&    
             let RemoteReocrds::A(addr) = entry.value().records{
                println!("Cache used for A");
                let record = Record::from_rdata(
                    query.name().clone(),
                    300,
                    RData::A(hickory_proto::rr::rdata::A(addr)),
                );
                let response=self.responce_builder.success_response(request, query.clone(), record)?;
                self.socket.send_to(&response, self.src).await?;
                return Ok(());
             }else if query.query_type().to_string()=="AAAA" &&    
             let RemoteReocrds::AAAA(addr) = entry.value().records {
                println!("Cache used for AAAA");
                let record = Record::from_rdata(
                    query.name().clone(),
                    300,
                    RData::AAAA(hickory_proto::rr::rdata::AAAA(addr)),
                );
                let response=self.responce_builder.success_response(request, query.clone(), record)?;
                self.socket.send_to(&response, self.src).await?;
                return Ok(());
             }
            } //end checking in the dashmap if data is there data will be send  need to remove duplicate codes



            match self.query_remote_dns(&data).await {
            Ok(upstream_bytes) => {
                self.socket.send_to(&upstream_bytes, self.src).await?;
            }
            Err(err) => {
                let resp_buffer=self.responce_builder.error_res(request, query)?;
                self.socket.send_to(&resp_buffer, self.src).await?;
                self.logger
                    .log(&format!("something  Went Wrong, more info:- {:?}", err))
                    .await;
            }
        }//end matche


        }
        Ok(())
        
    }//handel dns query


    async fn query_remote_dns(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        // query the remote dns server
        let server= env::var("fallback").unwrap_or(String::from("8.8.8.8:53"));
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
                    
                {
                    match ans.data() {
                        Some(RData::A(rdata::A(ipv4_ref)))=>{
                            let ip_owned=*ipv4_ref;
                            if let Some(mut entry) = self.remote_dns.get_mut(&ans.name().to_ascii()){
                                    entry.ttl=Instant::now() + Duration::from_secs(100);
                                    entry.records = RemoteReocrds::A(ip_owned);
                            }else{
                                let remote_cache = RemoteDnsCache {
                                    ttl: Instant::now() + Duration::from_secs(100),
                                    records: RemoteReocrds::A(ip_owned),
                                };
                                self.remote_dns.insert(ans.name().to_ascii(), remote_cache);
                            }

                        }
                        Some(RData::AAAA(rdata::AAAA(ipv6_ref)))=>{
                            let ip_owned=*ipv6_ref;
                            println!("AAAA The IP of this {} is this {}",ans.name().to_ascii(),ip_owned);
                            if let Some(mut entry) = self.remote_dns.get_mut(&ans.name().to_ascii()){
                                    entry.ttl=Instant::now() + Duration::from_secs(100);
                                    entry.records = RemoteReocrds::AAAA(ip_owned);
                            }else{
                                let remote_cache = RemoteDnsCache {
                                    ttl: Instant::now() + Duration::from_secs(100),
                                    records: RemoteReocrds::AAAA(ip_owned),
                                };
                                self.remote_dns.insert(ans.name().to_ascii(), remote_cache);
                            }
                        }
                        Some(RData::PTR(ip))=>{
                            println!("PTR The IP of this {} is this {}",ans.name().to_ascii(),ip);
                        }
                        _=>{
                            println!("IDK The IP of this {} is this",ans.name().to_ascii());

                        }
                        
                    }  
                    
                    
                    // && let Some(RData::A(ip)) = 
                    // let remote_cache = RemoteDnsCache {
                    //     ttl: Instant::now() + Duration::from_secs(100),
                    //     records: RemoteReocrds::A(ip.to_string().parse::<std::net::Ipv4Addr>()?),
                    // };
                    // self.remote_dns.insert(ans.name().to_ascii(), remote_cache);



                }
                Ok(buf[..len].to_vec())
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err(anyhow::anyhow!("DNS request timed out")),
        } //end matche
    }

    

    async fn docker_resolve(&self,request:Message,query:hickory_proto::op::query::Query)
    -> anyhow::Result<()> //resolve only docker records
    {
        let record_exists:bool={
            let data_reader =self.docker_dns.read().await;
            data_reader.contains_key(&query.name().to_ascii())
        };
        if record_exists  && query.query_type().to_string() == "A" {
            // currently docker only supports A records
            let data_reader =self.docker_dns.read().await;
            if let Some(ip_str) = data_reader.get(&query.name().to_ascii()).cloned()
            && let Ok(ip_address) = ip_str.parse::<std::net::Ipv4Addr>(){
                let record = Record::from_rdata(
                    query.name().clone(),
                    300,
                    RData::A(hickory_proto::rr::rdata::A(ip_address)),
                );
                let response=self.responce_builder.success_response(request, query.clone(), record)?;
                self.socket.send_to(&response, self.src).await?;
                self.logger
                    .log(&format!(
                        "Request received for {}, resolved to {}",
                        query.name(),
                        ip_address
                    ))
                    .await;
                return Ok(());
        }//innner if
        }//if to check record type
        let resp_buf=  self.responce_builder.error_res(request, query.clone())?;
        self.socket.send_to(&resp_buf, self.src).await?;
        self.logger
                .log(&format!(
                    "Unresolved Docker Query {}",
                    query.name()
                ))
                .await;
    Ok(())
    }



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
