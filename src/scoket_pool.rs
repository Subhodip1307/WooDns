use std::sync::{Arc,atomic::{AtomicU8,Ordering}};
use futures_util::lock::Mutex;
use tokio::net::UdpSocket;
use tokio::time::{Instant,Duration};

struct SocketInfo{
    scoket:UdpSocket,
    time:Instant
}

pub struct SocketPool{
    tx:mpsc::Sender<SocketInfo>,
    rx:Arc<Mutex<mpsc::Receiver<SocketInfo>>>,
    counter:AtomicU8,
}

use tokio::sync::mpsc;

impl SocketPool {
    pub async fn init()->Self{
        let (tx,rx)=mpsc::channel(50);
        for _ in 0..50{
            let socket=UdpSocket::bind("0.0.0.0:0").await.unwrap();
            let info=SocketInfo{ scoket:socket,time:Instant::now() + Duration::from_secs(60*5)};
            tx.send(info).await.unwrap();
        }
        Self{
            tx:tx,
            rx:Arc::new(Mutex::new(rx)),
            counter:AtomicU8::new(50),
        }
    }
    pub async fn open(&self)->UdpSocket{
        let mut receiver=self.rx.lock().await;
        match receiver.try_recv(){
            Ok(scoket_info)=>{ 
                scoket_info.scoket 
            },
            Err(_)=>{
                UdpSocket::bind("0.0.0.0:0").await.unwrap()//createing new socket in case there is no socket left/channel closed
            },
        }//end match
    }//end open

    pub async fn close(&self,socket:UdpSocket)->bool{
        if self.counter.load(Ordering::SeqCst) <= 50{
            let info=SocketInfo{ scoket:socket,time:Instant::now() + Duration::from_secs(60*5)};
            self.tx.send(info).await.unwrap();
            return true;
        }
        false
    }

    
}
