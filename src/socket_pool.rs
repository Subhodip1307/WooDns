use futures_util::lock::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicU16, Ordering},
};
use tokio::net::UdpSocket;
use tokio::time::{Duration, Instant};

struct SocketInfo {
    socket: UdpSocket,
    ttl: Instant,
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
        for _ in 0..calculate_initial_sockets(&count) {
            let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
            let info = SocketInfo {
                socket,
                ttl: Instant::now() + Duration::from_secs(60 * 5),
            };
            tx.send(info).await.unwrap();
        }
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            counter: AtomicU16::new(calculate_initial_sockets(&count)),
            max: count,
        }
    }
    pub async fn open(&self) -> UdpSocket {
        if self.counter.load(Ordering::SeqCst) > 0 {
            //checking if there are any socket left
            let mut receiver = self.rx.lock().await;
            match receiver.recv().await {
                Some(socket_info) => {
                    self.counter.fetch_sub(1, Ordering::SeqCst);
                    return socket_info.socket;
                }
                None => {
                    // if errors comes still will retrun the socket
                    println!("Someting went wrong ");
                }
            } //end match
        } //end if 
        UdpSocket::bind("0.0.0.0:0").await.unwrap() //createing new socket in case there is no socket left/channel closed
    } //end open

    pub async fn close(&self, socket: UdpSocket) -> bool {
        /*
        this function will use to close an open socket and this needs to be immune from Race Condition

         */
        self.counter.fetch_add(1, Ordering::SeqCst);
        if self.counter.load(Ordering::SeqCst) <= self.max {
            let info = SocketInfo {
                socket,
                ttl: Instant::now() + Duration::from_secs(60 * 5),
            };
            self.counter.fetch_add(1, Ordering::SeqCst);
            println!("reuseing the socket");
            match self.tx.try_send(info) {
                Ok(_) => println!("reuseing the socket"),
                Err(_) => println!("the channel is full"),
            }
            return true;
        }
        self.counter.fetch_sub(1, Ordering::SeqCst);
        false
    }

    pub async fn remove_socket(&self) {
        let current = self.counter.load(Ordering::SeqCst);
        if current > 1 {
            let mut receiver = self.rx.lock().await;
            let socket_info = match receiver.try_recv() {
                Ok(value) => value,
                Err(err) => {
                    // if errors comes still will retrun the socket
                    println!("Someting went wrong {:?}", err);
                    return;
                }
            }; //end match
            let current = Instant::now();
            if current < socket_info.ttl {
                //if ttl is stil more than current time then add the socket again
                self.close(socket_info.socket).await;
            } else {
                self.counter.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }
}

// also calculate the 10 percent of any number with round
fn calculate_initial_sockets(count: &u16) -> u16 {
    (*count * 10 + 5) / 100
}
