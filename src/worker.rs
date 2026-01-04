use tokio::sync::mpsc::Receiver;
use std::{sync::Arc,env};
use crate::loggin::{DnsLogger, LogHandler, all_write_now};
use crate::dns_handler::{RemoteDnsCache,remove_cache};
use crate::storage_system::DockerStorage;
use crate::docker::{event_monitor, gather_docker};
use crate::access_api::access;
use tokio::sync::{Mutex};
use tokio::signal;
use std::sync::atomic::{Ordering};
use tokio::time::{Duration, interval};
use dashmap::DashMap;


#[cfg(debug_assertions)]
const MAX_MESSAGE_BATCH_SIZE: usize = 10;

#[cfg(not(debug_assertions))]
const MAX_MESSAGE_BATCH_SIZE: usize = 100;



pub struct ServiceWorker{
    docker_dns:Arc<DockerStorage>,
    remote_dns_cache:Arc<DashMap<String, RemoteDnsCache>>,
    logger:Arc<DnsLogger>,
    // log_tx:
}

impl ServiceWorker{
    pub fn new(docker_dns:&Arc<DockerStorage>,remote_dns_cache:&Arc<DashMap<String, RemoteDnsCache>>,logger:&Arc<DnsLogger>)->Self{
        Self {docker_dns: Arc::clone(&docker_dns), remote_dns_cache:Arc::clone(&remote_dns_cache), logger:Arc::clone(&logger) }
    }
    pub  async fn init(&self,rx:Arc<Mutex<Receiver<String>>>){  
        
        #[cfg(debug_assertions)]
        let log_path = String::from("./woo");
        #[cfg(not(debug_assertions))]
        let log_path = env::var("woodns_log_path").unwrap_or("/var/log".to_string());

        let file_haneler = Arc::new(LogHandler::init(&log_path).await);

        // starts with docker collection
        self.docker_collector();
        self.docker_auto_update();
        // remove cache
        self.remove_dns_cache();
        // bactch log collector 
        let log_receiver = Arc::clone(&rx);
        self.batch_process_log(&log_receiver, &file_haneler);
        self.shutdown(&log_receiver,&file_haneler);
        // api
        self.socket_api_interface();
    }

    fn shutdown(&self,log_receiver:&Arc<Mutex<Receiver<String>>>,log_handeler:&Arc<LogHandler>){
        let logmanager = Arc::clone(&log_handeler);
        let rl=Arc::clone(&log_receiver);
        tokio::spawn(async move {
            signal::ctrl_c().await.expect("failed to listen for event");
            println!("Going to shutdown, writing all logs");
            std::process::exit(0);
            // this line need to be fixed worng path 
            // let file_name:String  =format!("{}/woodns/output.log", logmanager.file_number.load(Ordering::Relaxed));
            // println!("file name {}",file_name);
            // let mut receiver_lock2 = rl.lock().await;
            // all_write_now(file_name, &mut receiver_lock2).await;
            // std::process::exit(0);
        });
    }
    fn batch_process_log(&self,log_:&Arc<Mutex<Receiver<String>>>,file_:&Arc<LogHandler>){
        let log_receiver=Arc::clone(&log_);
        let file_haneler=Arc::clone(&file_);
        let batch_log_collection=Arc::clone(&self.logger);
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
    }

    fn docker_collector(&self){
         
        let docker_collection_log = Arc::clone(&self.logger);
        let writer_hasmap = Arc::clone(&self.docker_dns);
        tokio::spawn(async move {
            let _ = gather_docker(writer_hasmap, docker_collection_log).await;
        });
    }

    fn docker_auto_update(&self){
        let update_hasmap = Arc::clone(&self.docker_dns);
        let docker_event_log = Arc::clone(&self.logger);
        tokio::spawn(async move {
            let _ = event_monitor(update_hasmap, docker_event_log).await;
        });
    }

    fn remove_dns_cache(&self)/*not docker cache*/{
        let rm_cache_data = Arc::clone(&self.remote_dns_cache);
        tokio::spawn(async move {
            let _ = remove_cache(rm_cache_data).await;
        }); //track and remove cache
    }

    fn socket_api_interface(&self){
        let remote_data = Arc::clone(&self.remote_dns_cache);
        let docker_data = Arc::clone(&self.docker_dns);
        tokio::spawn(async move {
            let _ = access(remote_data, docker_data).await;
        });
    }
    
}