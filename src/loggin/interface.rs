use chrono::Local;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
pub struct DnsLogger {
    sender: tokio::sync::mpsc::Sender<String>,
    pub message_count: Arc<AtomicUsize>,
}

impl DnsLogger {
    pub async fn new(sender_channel: tokio::sync::mpsc::Sender<String>) -> Self {
        Self {
            sender: sender_channel,
            message_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn log(&self, message: &str) {
        let now = Local::now().format("%Y-%m-%d %I:%M:%S %p").to_string();
        let log_line = format!("[{}] {}", now, message);
        self.sender.send(log_line).await.expect("send failed tx1");
        self.message_count.fetch_add(1, Ordering::Relaxed);
        #[cfg(debug_assertions)]
        {
            println!("{}", message);
        }
    }

    pub fn restart_counter(&self) {
        self.message_count.store(0, Ordering::SeqCst);
    }
}
