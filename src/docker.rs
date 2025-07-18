use bollard::Docker;
use bollard::query_parameters::ListContainersOptionsBuilder;
use std::default::Default;
use std::error::Error;
use tokio::sync::RwLock;
use std::{collections::HashMap,sync::Arc};


pub async fn  gather_docker(data:Arc<RwLock<HashMap<String,String>>>)->Result<(), Box<dyn Error>>{
    
    
    let mut write_me = data.write().await;
    
    
    let docker=Docker::connect_with_socket_defaults();
    let options=ListContainersOptionsBuilder::default().build();

    let containers = docker.expect("REASON").list_containers(Some(options)).await?;

    for container in containers {
        let names = container
        .names
        .unwrap_or_default()
        .iter()
        .map(|n| n.trim_start_matches('/').to_string())
        .collect::<Vec<String>>()
        .join(", ")+".docker.";

        let ip_address = container
        .network_settings
        .as_ref()
        .and_then(|net_settings| {
            net_settings.networks.as_ref()?.values().next()?.ip_address.clone()
        })
        .unwrap_or_else(|| "No IP".to_string());
    println!("adding {names}");
    write_me.entry(names.to_string()).or_insert(ip_address);    
    }

    Ok(())
}
