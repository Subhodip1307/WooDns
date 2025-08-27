use chrono::Local;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use tokio::sync::Mutex;

fn count_lines_simple(file_path: &str) -> Result<usize, std::io::Error> {
    let contents = fs::read_to_string(file_path)?;
    let line_count = contents.lines().count();
    Ok(line_count)
}

// logging
pub struct DnsLogger {
    file: Mutex<std::fs::File>,
    counter: Mutex<u16>,
    folder: String,
    file_number: Mutex<u8>,
}

impl DnsLogger {
    pub fn new(file_path: String) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(format!("{file_path}/woodns"))?;
        let formated_file_path = format!("{file_path}/woodns/output.log");
        let mut lines_numbers = 0;

        if let Ok(lines) = count_lines_simple(&formated_file_path) {
            lines_numbers = lines;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(formated_file_path)?;
        Ok(Self {
            file: Mutex::new(file),
            counter: Mutex::new(lines_numbers as u16),
            folder: file_path,
            file_number: Mutex::new(1),
        })
    }

    pub async fn log(&self, message: &str) {
        let mut counter = self.counter.lock().await;
        let mut file = self.file.lock().await;
        if *counter > 1000 {
            let mut file_num = self.file_number.lock().await;
            match OpenOptions::new()
                .create(true)
                .append(true)
                .open(format!("{}/woodns/{}_output.log", self.folder, *file_num))
            {
                Ok(new_file) => {
                    *file = new_file;
                    *file_num += 1;
                    *counter = 1;
                }
                Err(e) => {
                    eprintln!("Failed to create new log file: {}", e);
                    return; // Exit early on error
                }
            }
        } else {
            *counter += 1;
        }

        let now = Local::now().format("%Y-%m-%d %I:%M:%S %p").to_string();
        let log_line = format!("[{}] {}\n", now, message);
        let _ = file.write_all(log_line.as_bytes());
        let _ = file.flush();
        // Also print to console
        println!("{}", log_line.trim());
    }
}
