use std::{collections::HashMap, env, sync::Arc};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
mod dns_handler;
mod docker;
mod loggin;
mod response_handler;
use dashmap::DashMap;
use dns_handler::{DNSManager, RemoteDnsCache, remove_cache};
use docker::{event_monitor, gather_docker};
use loggin::DnsLogger;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Runing Version 3.0.1");
    let log_path = env::var("woodns_log_path").unwrap_or(String::from("/var/log"));

    let logger = Arc::new(DnsLogger::new(log_path)?);
    logger.log("Server starting...").await;

    let remote_dns: Arc<DashMap<String, RemoteDnsCache>> = Arc::new(DashMap::new());
    let dns_store = Arc::new(RwLock::new(HashMap::new()));

    {
        let docker_collection_log = Arc::clone(&logger);
        let writer_hasmap = Arc::clone(&dns_store);
        let _ = gather_docker(writer_hasmap, docker_collection_log).await;
    } //collect all dockers

    {
        let update_hasmap = Arc::clone(&dns_store);
        let docker_event_log = Arc::clone(&logger);
        tokio::spawn(async move {
            let _ = event_monitor(update_hasmap, docker_event_log).await;
        });
    } //track envents

    {
        let rm_cache_data = Arc::clone(&remote_dns);
        tokio::spawn(async move {
            let _ = remove_cache(rm_cache_data).await;
        }); //track and remove cache
    }

    let address = env::var("host").unwrap_or(String::from("127.0.0.13"));
    let socket = Arc::new(UdpSocket::bind(format!("{}:53", address)).await?);
    logger
        .log(&format!("DNS server listening on {address} UDP port 53"))
        .await;

    let mut buf = [0u8; 512];

    loop {
        let (len, src) = socket.recv_from(&mut buf).await?;
        let data = buf[..len].to_vec();
        let socket = Arc::clone(&socket);
        let access_logger = Arc::clone(&logger);

        let dns_manager = DNSManager::new(
            Arc::clone(&dns_store),
            access_logger.clone(),
            Arc::clone(&remote_dns),
            src,
            socket
        );

        tokio::spawn(async move {
            if let Err(err) = dns_manager.handle_dns_query(data).await {
                access_logger
                    .log(&format!("Error handling query from {}: {:?}", src, err))
                    .await;
            }
        });
    } //end loop
}
