use std::{collections::HashMap, env, net::SocketAddr, sync::Arc};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::{Duration, timeout};
use trust_dns_proto::op::{Message, MessageType, OpCode};
use trust_dns_proto::rr::{RData, Record};
use trust_dns_proto::serialize::binary::{BinDecodable, BinEncodable, BinEncoder};
mod docker;
mod loggin;
use docker::event_monitor;
use docker::gather_docker;
use loggin::DnsLogger;
use trust_dns_proto::op::ResponseCode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Runing Version 2.0.1");
    //getting custom loggin path
    let mut log_path = String::from("/var/log");

    match env::var("woodns_log_path") {
        Ok(value) => log_path = value,
        Err(_) => println!("no woodns_log_path Environment variable given"),
    }

    let logger = Arc::new(DnsLogger::new(log_path)?);

    logger.log("Server starting...").await;

    let dns_store = Arc::new(RwLock::new(HashMap::new()));
    let writer_hasmap = Arc::clone(&dns_store);
    let update_hasmap = Arc::clone(&dns_store);

    let docker_collection_log = Arc::clone(&logger);
    let docker_event_log = Arc::clone(&logger);

    tokio::spawn(async move {
        let _ = gather_docker(writer_hasmap, docker_collection_log).await;
    }); //collect all dockers
    tokio::spawn(async move {
        let _ = event_monitor(update_hasmap, docker_event_log).await;
    }); //track envents

    //check fot custom address
    let mut address = String::from("127.0.0.13");
    match env::var("host") {
        Ok(val) => address = val,
        Err(_) => logger.log("no Host Environment variable given").await,
    }

    let socket = Arc::new(UdpSocket::bind(format!("{}:53", address)).await?);
    logger
        .log(&format!("DNS server listening on {address} UDP port 53"))
        .await;

    let mut buf = [0u8; 512];

    loop {
        let (len, src) = socket.recv_from(&mut buf).await?;
        let data = buf[..len].to_vec();

        let store = Arc::clone(&dns_store);
        let socket = Arc::clone(&socket);
        let access_logger = Arc::clone(&logger);

        tokio::spawn(async move {
            if let Err(err) =
                handle_dns_query(data, src, store, socket, access_logger.clone()).await
            {
                access_logger
                    .log(&format!("Error handling query from {}: {:?}", src, err))
                    .await;
            }
        });
    }
}

async fn handle_dns_query(
    data: Vec<u8>,
    src: SocketAddr,
    store: Arc<RwLock<HashMap<String, String>>>,
    socket: Arc<UdpSocket>,
    logger: Arc<DnsLogger>,
) -> anyhow::Result<()> {
    let request = Message::from_bytes(&data)?;
    let mut response = Message::new();

    response
        .set_id(request.id())
        .set_message_type(MessageType::Response)
        .set_op_code(OpCode::Query)
        .set_authoritative(true)
        .set_recursion_desired(request.recursion_desired())
        .set_recursion_available(false);

    let mut found_local = false;

    for query in request.queries() {
        let domain = query.name().to_ascii();
        let ip_opt = {
            let db = store.read().await;
            db.get(&domain).cloned()
        };

        if let Some(ip_str) = ip_opt {
            if let Ok(ip) = ip_str.parse::<std::net::Ipv4Addr>() {
                let record = Record::from_rdata(
                    query.name().clone(),
                    300, // TTL in seconds
                    RData::A(trust_dns_proto::rr::rdata::A(ip)),
                );
                response.add_answer(record);
                found_local = true;
                logger
                    .log(&format!(
                        "Request received for {} , resolved to {}",
                        domain, ip_str
                    ))
                    .await;
            }
        }
        response.add_query(query.clone());
    }

    // If we found local records, send our response
    if found_local {
        let mut resp_buffer = Vec::with_capacity(512);
        let mut encoder = BinEncoder::new(&mut resp_buffer);
        response.emit(&mut encoder)?;
        socket.send_to(&resp_buffer, src).await?;
        return Ok(());
    }

    // Otherwise, forward to system DNS
    logger
        .log("Request not found locally, forwarding to system DNS")
        .await;
    match forward_to_system_dns(&data, &logger).await {
        Ok(upstream_bytes) => {
            socket.send_to(&upstream_bytes, src).await?;
        }
        Err(e) => {
            logger
                .log(&format!("Error forwarding to system DNS: {:?}", e))
                .await;
            response.set_response_code(ResponseCode::ServFail);
            let mut resp_buffer = Vec::with_capacity(512);
            let mut encoder = BinEncoder::new(&mut resp_buffer);
            response.emit(&mut encoder)?;
            socket.send_to(&resp_buffer, src).await?;
        }
    }

    Ok(())
}

// Forward to system DNS with fallback
async fn forward_to_system_dns(data: &[u8], logger: &Arc<DnsLogger>) -> anyhow::Result<Vec<u8>> {
    let mut server = String::from("8.8.8.8");
    if let Ok(dns_ip) = env::var("fallback") {server =dns_ip}
    match try_dns_server(data, &server).await {
        Ok(response) => Ok(response),
        Err(e) => {
            logger
                .log(&format!("Failed to query DNS server {}: {:?}", server, e))
                .await;
            Err(anyhow::anyhow!("All DNS servers failed"))
        }
    }
}

async fn try_dns_server(data: &[u8], server_addr: &str) -> anyhow::Result<Vec<u8>> {
    let upstream_socket = UdpSocket::bind("0.0.0.0:0").await?;
    upstream_socket.send_to(data, server_addr).await?;

    let mut buf = [0u8; 512];
    let recv_result = timeout(Duration::from_secs(2), upstream_socket.recv_from(&mut buf)).await;

    match recv_result {
        Ok(Ok((len, _))) => Ok(buf[..len].to_vec()),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(anyhow::anyhow!("DNS request timed out")),
    }
}
