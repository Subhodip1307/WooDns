use std::{env, sync::Arc};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
mod access_api;
mod handler;
use handler::{dns_handler, response_handler};
mod docker;
mod loggin;
mod socket_pool;
use dashmap::DashMap;
use dns_handler::{DNSManager, RemoteDnsCache};
use loggin::DnsLogger;
use socket_pool::SocketPool;
mod storage_system;
use storage_system::DockerStorage;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
mod worker;
use worker::ServiceWorker;



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel::<String>(4096);
    let rx = Arc::new(Mutex::new(rx));

    let logger = Arc::new(DnsLogger::new(tx).await);
    logger
        .log(&format!("Runing Version {}", env!("CARGO_PKG_VERSION")))
        .await;
    logger.log("Server starting...").await;
    let remote_dns: Arc<DashMap<String, RemoteDnsCache>> = Arc::new(DashMap::new());
    let dns_store: Arc<DockerStorage> = Arc::new(DockerStorage::new());
    // starting other services
    let workers=ServiceWorker::new(&dns_store,&remote_dns,&logger);
    workers.init(rx).await;


    let address = env::var("host").unwrap_or(String::from("127.0.0.13"));
    let socket = Arc::new(
        UdpSocket::bind(format!("{}:53", address))
            .await
            .unwrap_or_else(|_| panic!("Unable to operate on {}:53", address))
    );
    logger
        .log(&format!("DNS server listening on {address} UDP port 53"))
        .await;

    let mut buf = [0u8; 512];
    // opening socket

    // socket pooling
    let remote_sockets = Arc::new(SocketPool::init(10).await);

    {
        let all_sockets = Arc::clone(&remote_sockets);
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(60));
            ticker.tick().await;
            loop {
                all_sockets.remove_socket().await;
                ticker.tick().await;
            }
        });
    }

    loop {
        let (len, src) = match socket.recv_from(&mut buf).await {
            Ok((len, src)) => (len, src),
            Err(e) => {
                println!("error whiile receving message {}", e);
                std::process::exit(0);
            }
        };
        let data = buf[..len].to_vec();
        let socket = Arc::clone(&socket);
        let access_logger = Arc::clone(&logger);

        let dns_manager = DNSManager::new(
            Arc::clone(&dns_store),
            access_logger.clone(),
            Arc::clone(&remote_dns),
            src,
            socket,
            Arc::clone(&remote_sockets),
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
