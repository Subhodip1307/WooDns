use std::{collections::HashMap, net::SocketAddr, sync::Arc,env};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use trust_dns_proto::op::{Message, MessageType, OpCode};
use trust_dns_proto::rr::{RData, Record};
use trust_dns_proto::serialize::binary::{BinDecodable, BinEncodable, BinEncoder};
mod docker;
mod loggin;
use loggin::DnsLogger;
use docker::gather_docker;
use docker::event_monitor;
use trust_dns_proto::op::ResponseCode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //getting custom loggin path
    let mut log_path=String::from("/var/log");
    match env::var("woodns_log_path"){
        Ok(value)=>log_path=value,
        Err(_)=>println!("no woodns_log_path Environment variable given"),
    }

    let logger = Arc::new(DnsLogger::new(log_path)?);

    logger.log("Server starting...").await;

    let dns_store = Arc::new(RwLock::new(HashMap::new()));
    let writer_hasmap = Arc::clone(&dns_store);
    let update_hasmap = Arc::clone(&dns_store);

    let docker_collection_log=Arc::clone(&logger);
    let docker_event_log=Arc::clone(&logger);
    tokio::spawn(async move{
        let _ =gather_docker(writer_hasmap,docker_collection_log).await;
    });//collect all dockers
    tokio::spawn(async move {
        let _ =event_monitor(update_hasmap,docker_event_log).await;
    });//track envents
    
    //check fot custom address
    let mut address=String::from("127.0.0.13");
    match env::var("host") {
        Ok(val) => address=val,
        Err(_) =>logger.log("no Host Environment variable given").await,
    }

    let socket = Arc::new(UdpSocket::bind(format!("{}:53",address)).await?);
    logger.log(&format!("DNS server listening on {address} UDP port 53")).await;
    
    let mut buf = [0u8; 512];

    loop {
        let (len, src) = socket.recv_from(&mut buf).await?;
        let data = buf[..len].to_vec();

        let store = Arc::clone(&dns_store);
        let socket = Arc::clone(&socket);
        let access_logger=Arc::clone(&logger);

        tokio::spawn(async move {
            if let Err(err) = handle_dns_query(data, src, store, socket,access_logger.clone()).await {
                access_logger.log(&format!("Error handling query from {}: {:?}", src, err)).await;
            }
        });
    }
}

async fn handle_dns_query(
    data: Vec<u8>,
    src: SocketAddr,
    store: Arc<RwLock<HashMap<String, String>>>,
    socket: Arc<UdpSocket>,
    logger: Arc<DnsLogger>
) -> anyhow::Result<()> {
    let request = Message::from_bytes(&data)?;
    
   
    let mut local_answers = Vec::new();
    let mut needs_forwarding = Vec::new();
    
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
                local_answers.push(record);
                logger.log(&format!("Request received for {}, resolved locally to {}", domain, ip_str)).await;
            }
        } else {
            needs_forwarding.push(query);
            logger.log(&format!("Request for {} not found locally", domain)).await;
        }
    }
    
   
    if !local_answers.is_empty() && needs_forwarding.is_empty() {
        let mut response = Message::new();
        response
            .set_id(request.id())
            .set_message_type(MessageType::Response)
            .set_op_code(OpCode::Query)
            .set_authoritative(true)
            .set_recursion_desired(request.recursion_desired())
            .set_recursion_available(false);
            
        for query in request.queries() {
            response.add_query(query.clone());
        }
        
        for answer in local_answers {
            response.add_answer(answer);
        }
        
        send_response(response, socket, src).await?;
        return Ok(());
    }
    
    // If we need to forward (either all queries or mixed), forward the entire request
    if !needs_forwarding.is_empty() {
        logger.log("Forwarding request to upstream DNS server").await;
        
        match forward_to_upstream_dns(&data).await {
            Ok(upstream_response) => {
                // If we had some local answers, we need to merge them with upstream
                if !local_answers.is_empty() {
                    if let Ok(mut upstream_msg) = Message::from_bytes(&upstream_response) {
                        // Add our local answers to the upstream response
                        for answer in local_answers {
                            upstream_msg.add_answer(answer);
                        }
                        
                        // Send the merged response
                        send_response(upstream_msg, socket, src).await?;
                    } else {
                        // If upstream parsing failed, send our local answers with SERVFAIL for others
                        send_local_with_servfail(request, local_answers, socket, src, logger).await?;
                    }
                } else {
                    // Just forward the upstream response as-is
                    socket.send_to(&upstream_response, src).await?;
                }
            }
            Err(e) => {
                logger.log(&format!("Error contacting upstream DNS: {:?}", e)).await;
                
                if !local_answers.is_empty() {
                    // Send local answers with SERVFAIL for the rest
                    send_local_with_servfail(request, local_answers, socket, src, logger).await?;
                } else {
                    // Send SERVFAIL for everything
                    let  response = create_error_response(&request, ResponseCode::ServFail);
                    send_response(response, socket, src).await?;
                }
            }
        }
    }
    
    Ok(())
}

async fn send_response(
    response: Message,
    socket: Arc<UdpSocket>,
    src: SocketAddr
) -> anyhow::Result<()> {
    let mut resp_buffer = Vec::with_capacity(512);
    let mut encoder = BinEncoder::new(&mut resp_buffer);
    response.emit(&mut encoder)?;
    socket.send_to(&resp_buffer, src).await?;
    Ok(())
}

async fn send_local_with_servfail(
    request: Message,
    local_answers: Vec<Record>,
    socket: Arc<UdpSocket>,
    src: SocketAddr,
    logger: Arc<DnsLogger>
) -> anyhow::Result<()> {
    logger.log("Sending local answers with SERVFAIL for upstream queries").await;
    
    let mut response = Message::new();
    response
        .set_id(request.id())
        .set_message_type(MessageType::Response)
        .set_op_code(OpCode::Query)
        .set_authoritative(false)
        .set_recursion_desired(request.recursion_desired())
        .set_recursion_available(true)
        .set_response_code(ResponseCode::ServFail);
        
    for query in request.queries() {
        response.add_query(query.clone());
    }
    
    for answer in local_answers {
        response.add_answer(answer);
    }
    
    send_response(response, socket, src).await
}

fn create_error_response(request: &Message, code: ResponseCode) -> Message {
    let mut response = Message::new();
    response
        .set_id(request.id())
        .set_message_type(MessageType::Response)
        .set_op_code(OpCode::Query)
        .set_authoritative(false)
        .set_recursion_desired(request.recursion_desired())
        .set_recursion_available(true)
        .set_response_code(code);
        
    for query in request.queries() {
        response.add_query(query.clone());
    }
    
    response
}

// Improved forwarding function with better error handling
async fn forward_to_upstream_dns(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let upstream_servers = ["8.8.8.8:53", "1.1.1.1:53", "208.67.222.222:53"];
    
    for &upstream_addr in &upstream_servers {
        match try_upstream_server(data, upstream_addr).await {
            Ok(response) => return Ok(response),
            Err(e) => {
                eprintln!("Failed to reach {}: {:?}", upstream_addr, e);
                continue;
            }
        }
    }
    
    Err(anyhow::anyhow!("All upstream DNS servers failed"))
}

async fn try_upstream_server(data: &[u8], upstream_addr: &str) -> anyhow::Result<Vec<u8>> {
    let upstream_socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    
    // Set a socket timeout
    upstream_socket.send_to(data, upstream_addr).await?;

    let mut buf = [0u8; 1024]; 
    let recv_result = timeout(Duration::from_secs(5), upstream_socket.recv_from(&mut buf)).await;
    
    match recv_result {
        Ok(Ok((len, _))) => Ok(buf[..len].to_vec()),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(anyhow::anyhow!("Upstream DNS request timed out for {}", upstream_addr)),
    }
}