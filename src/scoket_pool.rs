use futures_util::lock::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicU16, Ordering},
};
use tokio::net::UdpSocket;
use tokio::time::{Duration, Instant};

struct SocketInfo {
    scoket: UdpSocket,
    time: Instant,
}

pub struct SocketPool {
    tx: mpsc::Sender<SocketInfo>,
    rx: Arc<Mutex<mpsc::Receiver<SocketInfo>>>,
    counter: AtomicU16,
    max: u16,
}

use tokio::sync::mpsc;

impl SocketPool {
    pub async fn init(count: u16) -> Self {
        let (tx, rx) = mpsc::channel(count as usize);
        for _ in 0..calculate_initial_scokets(&count) {
            let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
            let info = SocketInfo {
                scoket: socket,
                time: Instant::now() + Duration::from_secs(60 * 5),
            };
            tx.send(info).await.unwrap();
        }
        Self {
            tx: tx,
            rx: Arc::new(Mutex::new(rx)),
            counter: AtomicU16::new(calculate_initial_scokets(&count)),
            max: count,
        }
    }
    pub async fn open(&self) -> UdpSocket {
        if self.counter.load(Ordering::SeqCst) > 0 {
            //checking if there are any socket left
            let mut receiver = self.rx.lock().await;
            match receiver.try_recv() {
                Ok(scoket_info) => {
                    self.counter.fetch_sub(1, Ordering::SeqCst);
                    return scoket_info.scoket;
                }
                Err(err) => {
                    // if errors comes still will retrun the scoket
                    println!("Someting went wrong {:?}", err);
                }
            } //end match
        } //end if 
        UdpSocket::bind("0.0.0.0:0").await.unwrap() //createing new socket in case there is no socket left/channel closed
    } //end open

    pub async fn close(&self, socket: UdpSocket) -> bool {
        if self.counter.load(Ordering::SeqCst) <= self.max {
            let info = SocketInfo {
                scoket: socket,
                time: Instant::now() + Duration::from_secs(60 * 5),
            };
            self.counter.fetch_add(1, Ordering::SeqCst);
            println!("reuseing the socket");
            self.tx.send(info).await.unwrap();
            return true;
        }
        false
    }

    pub async fn remove_scoket(&self) {
        let current = self.counter.load(Ordering::SeqCst);
        if current > 1 {
            let mut receiver = self.rx.lock().await;
            match receiver.try_recv() {
                Ok(_) => {
                    self.counter.fetch_sub(1, Ordering::SeqCst);
                    return;
                }
                Err(err) => {
                    // if errors comes still will retrun the scoket
                    println!("Someting went wrong {:?}", err);
                    return;
                }
            } //end match
        }
    }
}

// also calculate the 10 percent of any number with round
fn calculate_initial_scokets(count: &u16) -> u16 {
    (*count * 10 + 5) / 100
}
