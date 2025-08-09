use tokio::sync::Mutex;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

// logging
pub struct DnsLogger {
    file:Mutex<std::fs::File>,
}

impl DnsLogger {
    pub fn new(file_path:String) ->Result<Self,std::io::Error>{
        std::fs::create_dir_all(&format!("{file_path}/woodns"))?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&format!("{file_path}/woodns/output.log"))?;
        Ok(Self {
            file:Mutex::new(file),  
        })
    }
    pub async fn log(&self,message:&str){
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let log_line = format!("[{}] {}\n", now, message);
        
        let mut file = self.file.lock().await ;
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        // Also print to console
        println!("{}", log_line.trim());
    }
}
