use std::{env, sync::Arc};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
mod dns_handler;
mod docker;
mod loggin;
mod response_handler;
mod access_api;
use access_api::access;
use dashmap::DashMap;
use dns_handler::{DNSManager, RemoteDnsCache, remove_cache};
use docker::{event_monitor, gather_docker};
use loggin::{DnsLogger, LogHandler, all_write_now, get_file_count};
mod storage_system;
use std::sync::atomic::Ordering;
use storage_system::DockerStorage;
use tokio::signal;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};


#[cfg(debug_assertions)]
const MAX_MESSAGE_BATCH_SIZE: usize = 10;

#[cfg(not(debug_assertions))]
const MAX_MESSAGE_BATCH_SIZE: usize = 100;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel::<String>(4096);
    let rx = Arc::new(Mutex::new(rx));

    #[cfg(debug_assertions)]
    let log_path = String::from("./woo");
    #[cfg(not(debug_assertions))]
    let log_path = env::var("woodns_log_path").unwrap_or(String::from("/var/log"));

    let mut file_haneler = LogHandler::init(log_path.clone()).await;
    let logger = Arc::new(DnsLogger::new(tx).await);
    logger
        .log(&format!("Runing Version {}", env!("CARGO_PKG_VERSION")))
        .await;
    logger.log("Server starting...").await;

    //shutdown handel
    {
        let batch_log_collection = Arc::clone(&logger);
        let log_receiver = Arc::clone(&rx);
        tokio::spawn(async move {
            signal::ctrl_c().await.expect("failed to listen for event");
            println!("Going to shutdown, writing all logs");
            let file_name: String = {
                let files_numbers =match get_file_count(&log_path).await{
                    Ok(count)=>count,
                    Err(err)=>{batch_log_collection.log(&format!("error while trying to get file count  {}",err)).await;1}
                };
                if files_numbers <= 1 {
                    format!("{}/woodns/output.log", log_path)
                } else {
                    format!("{}/woodns/output.log.{}", log_path, files_numbers)
                }
            };
            let mut receiver_lock = log_receiver.lock().await;
            all_write_now(file_name, &mut receiver_lock).await;
            std::process::exit(0);
        });
    }



    // log batch process
    {
        let log_receiver = Arc::clone(&rx);
        let batch_log_collection = Arc::clone(&logger);
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(5));
            loop {
                if batch_log_collection.message_count.load(Ordering::Relaxed)
                    >= MAX_MESSAGE_BATCH_SIZE
                {
                    let mut receiver_lock = log_receiver.lock().await;
                    println!("message count riched to max batch size");
                    batch_log_collection.restart_counter();
                    file_haneler.bulk_write(&mut receiver_lock).await;
                }
                ticker.tick().await;
            }
        });
    } //end block

    let remote_dns: Arc<DashMap<String, RemoteDnsCache>> = Arc::new(DashMap::new());
    let dns_store: Arc<DockerStorage> = Arc::new(DockerStorage::new());
    {
        let docker_collection_log = Arc::clone(&logger);
        let writer_hasmap = Arc::clone(&dns_store);
        let _ = gather_docker(writer_hasmap, docker_collection_log).await;
    } //collect all dockers

    // for testing
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

    {//the simple api interface
        let remote_data = Arc::clone(&remote_dns);
        let  docker_data= Arc::clone(&dns_store);
        tokio::spawn(async move {
            let _ =access(remote_data,docker_data).await;
        });


    }

    let address = env::var("host").unwrap_or(String::from("127.0.0.13"));
    let socket = Arc::new(UdpSocket::bind(format!("{}:53", address)).await.expect(&format!("Unable to operate on {}:53",address)));
    logger
        .log(&format!("DNS server listening on {address} UDP port 53"))
        .await;

    let mut buf = [0u8; 512];
    // opening socket
   

    loop {
        let (len, src) =match socket.recv_from(&mut buf).await{
            Ok((len,src))=>(len,src),
            Err(e)=>{
                println!("error whiile receving message {}",e);
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
