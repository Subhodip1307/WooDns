use std::{collections::HashMap, net::SocketAddr, sync::Arc,env};

use tokio::net::UdpSocket;
use tokio::sync::RwLock;

use trust_dns_proto::op::{Message, MessageType, OpCode};
use trust_dns_proto::rr::{RData, Record};
use trust_dns_proto::serialize::binary::{BinDecodable, BinEncodable, BinEncoder};
mod docker;
use docker::gather_docker;
use docker::event_monitor;
use trust_dns_proto::op::ResponseCode;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dns_store = Arc::new(RwLock::new(HashMap::new()));
    let writer_hasmap = Arc::clone(&dns_store);
    let update_hasmap = Arc::clone(&dns_store);

    tokio::spawn(async move{
        let _ =gather_docker(writer_hasmap).await;
    });

    tokio::spawn(async move {
        let _ =event_monitor(update_hasmap).await;
    });

    let mut address=String::from("127.0.0.13");
    match env::var("host") {
        Ok(val) => address=val,
        Err(_) => println!("No Environment variable given"),
    }
    let socket = Arc::new(UdpSocket::bind(format!("{}:53",address)).await?);
    println!("DNS server listening on {address} UDP port 53");

    

    let mut buf = [0u8; 512];

    loop {
        let (len, src) = socket.recv_from(&mut buf).await?;
        let data = buf[..len].to_vec();

        let store = Arc::clone(&dns_store);
        let socket = Arc::clone(&socket);

        tokio::spawn(async move {
            if let Err(err) = handle_dns_query(data, src, store, socket).await {
                eprintln!("Error handling query from {}: {:?}", src, err);
            }
        });
    }
}

async fn handle_dns_query(
    data: Vec<u8>,
    src: SocketAddr,
    store: Arc<RwLock<HashMap<String, String>>>,
    socket: Arc<UdpSocket>,
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
            }
        println!("request recived for {} , reolved to {}",domain,ip_str);
        }
        else{
            println!("request received for {domain}, forwarding to upstream");

          
            let upstream_bytes = forward_to_upstream_dns(&data).await?;

           
            let upstream_msg = match Message::from_bytes(&upstream_bytes) {
                Ok(m) => m,
                Err(err) => {
                    eprintln!("Failed to parse upstream DNS reply: {:?}", err);
                    
                    response.set_response_code(ResponseCode::ServFail);
                    let mut buf = Vec::with_capacity(512);
                    let mut enc = BinEncoder::new(&mut buf);
                    response.emit(&mut enc)?;
                    socket.send_to(&buf, src).await?;
                    return Ok(());
                }
            };

           
            response.set_recursion_available(true);
            for answer in upstream_msg.answers() {
                response.add_answer(answer.clone());
            }
            for auth in upstream_msg.name_servers() {
                response.add_name_server(auth.clone());
            }
            for add in upstream_msg.additionals() {
                response.add_additional(add.clone());
            }

            
            let mut resp_buffer = Vec::with_capacity(512);
            {
                let mut encoder = BinEncoder::new(&mut resp_buffer);
                response.emit(&mut encoder)?;
            }
            socket.send_to(&resp_buffer, src).await?;
            return Ok(());

        }

        response.add_query(query.clone());
    }

    let mut resp_buffer = Vec::with_capacity(512);
    let mut encoder = BinEncoder::new(&mut resp_buffer);
    response.emit(&mut encoder)?;

    socket.send_to(&resp_buffer, src).await?;
    Ok(())
}


async fn forward_to_upstream_dns(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let upstream_addr = "8.8.8.8:53";
    let upstream_socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    upstream_socket.send_to(data, upstream_addr).await?;

    let mut buf = [0u8; 512];
    let (len, _) = upstream_socket.recv_from(&mut buf).await?;
    Ok(buf[..len].to_vec())
}