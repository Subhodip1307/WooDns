use std::sync::atomic::{AtomicU16, Ordering};
use tokio::fs::OpenOptions as TokioOpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

#[cfg(debug_assertions)]
const MAX_LOG_SIZE: u16 = 100;

#[cfg(not(debug_assertions))]
const MAX_LOG_SIZE: u16 = 1000;

pub struct LogHandeler {
    file: String, //full file path
    folder: String,
    pub file_number: AtomicU16,
    pub line_counter: AtomicU16,
}

impl LogHandeler {
    pub async fn init(location: String) -> Self {
        /*
        Create LogHandeler obj and return that and also decide the log file name
         */
        let folder_path = format!("{location}/woodns");
        tokio::fs::create_dir_all(&folder_path).await.unwrap();
        let files_count = get_file_count(&folder_path).await.unwrap_or(0);
        if files_count <= 1 {
            let file_name = format!("{}/output.log", folder_path);
            let line_count = count_file_line(&file_name).await.unwrap();
            return Self {
                file: file_name,
                line_counter: AtomicU16::new(line_count),
                folder: folder_path,
                file_number: AtomicU16::new(files_count),
            };
        } //endif

        let (file_name, line_count) = decide_file(&folder_path, &AtomicU16::new(files_count)).await;

        Self {
            file: file_name,
            line_counter: AtomicU16::new(line_count),
            folder: folder_path,
            file_number: AtomicU16::new(files_count),
        }
    }
    pub async fn bluk_write(&mut self, reciver: &mut tokio::sync::mpsc::Receiver<String>) {
        let logs_vect = get_messages(reciver);
        if !logs_vect.is_empty() {
            if self.line_counter.load(Ordering::Relaxed) > MAX_LOG_SIZE {
                let (new_file_location, _) = decide_file(&self.folder, &self.file_number).await;
                println!("new log file name is {}", new_file_location);
                self.file = new_file_location;
                self.line_counter.store(0, Ordering::SeqCst);
            }
            let file = TokioOpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file)
                .await
                .unwrap();
            let mut writer = BufWriter::new(file);
            self.line_counter
                .fetch_add(logs_vect.len() as u16, Ordering::SeqCst);
            for line in logs_vect {
                writer.write_all(line.as_bytes()).await.unwrap();
                writer.write_all(b"\n").await.unwrap();
            }
            writer.flush().await.unwrap();
        } //end if
        // empty vec no write
    }
}
// get all messages
fn get_messages(reciver: &mut tokio::sync::mpsc::Receiver<String>) -> Vec<String> {
    let mut logs_vect: Vec<String> = Vec::new();
    while let Ok(msg) = reciver.try_recv() {
        logs_vect.push(msg);
    }
    logs_vect
}

// file line count
async fn count_file_line(path: &String) -> anyhow::Result<u16> {
    let file = TokioOpenOptions::new()
        .read(true)
        .create(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .await?;

    let metadata = file.metadata().await?;
    if metadata.len() == 0 {
        return Ok(0);
    }

    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut count: u16 = 0;

    while (lines.next_line().await?).is_some() {
        count += 1;
    }

    Ok(count)
}
// decide file name
async fn decide_file(folder_path: &String, files_count: &AtomicU16) -> (String, u16) {
    let files_numbers = files_count.load(Ordering::Relaxed);

    let mut file_name: String = {
        if files_numbers <= 1 {
            format!("{}/output.log", folder_path)
        } else {
            format!("{}/output.log.{}", folder_path, files_numbers)
        }
    };
    let file_line_count = count_file_line(&file_name).await.unwrap_or(0);
    // TODO: get max log size from config file
    if file_line_count >= MAX_LOG_SIZE {
        file_name = format!("{}/output.log.{}", folder_path, files_numbers + 1);
        files_count.fetch_add(1, Ordering::Relaxed);
        println!(
            "old file counter {} is increase to {}",
            files_numbers,
            files_count.load(Ordering::Relaxed)
        );
    }
    (file_name, file_line_count)
}

pub async fn get_file_count(folder: &String) -> anyhow::Result<u16> {
    let mut dirs = tokio::fs::read_dir(&folder).await?;
    let mut count: u16 = 0;
    while (dirs.next_entry().await?).is_some() {
        count += 1;
    }
    Ok(count)
}

pub async fn all_write_now(file_path: String, reciver: &mut tokio::sync::mpsc::Receiver<String>) {
    let logs_vect = get_messages(reciver);
    let file = TokioOpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .await
        .unwrap();
    let mut writer = BufWriter::new(file);
    for line in logs_vect {
        writer.write_all(line.as_bytes()).await.unwrap();
        writer.write_all(b"\n").await.unwrap();
    }
    writer.flush().await.unwrap();
}
