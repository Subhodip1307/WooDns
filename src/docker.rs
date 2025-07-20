use bollard::Docker;
use bollard::query_parameters::ListContainersOptionsBuilder;
use std::default::Default;
use std::error::Error;
use tokio::sync::RwLock;
use std::{collections::HashMap,sync::Arc};

use bollard::query_parameters::EventsOptions;
use futures_util::stream::StreamExt;
use bollard::models::EventMessageTypeEnum;


pub async fn  gather_docker(data:Arc<RwLock<HashMap<String,String>>>)->Result<(), Box<dyn Error>>{
    let mut write_me = data.write().await;
    let docker=Docker::connect_with_socket_defaults().unwrap();
    let options=ListContainersOptionsBuilder::default().build();

    let containers = docker.list_containers(Some(options)).await?;

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


pub async fn event_monitor(data:Arc<RwLock<HashMap<String,String>>>) {
    let docker = Docker::connect_with_socket_defaults().unwrap();
    let mut events = docker.events(Some(EventsOptions::default())).boxed();
    
    while let Some(Ok(event)) = events.next().await {
        // println!("{:#?}", event);
        if event.typ == Some(EventMessageTypeEnum::CONTAINER) {
            if let Some(ref action) = event.action {
                match action.as_str() {
                    "start" | "connect" => {
                        println!("Docker Started");
                    }
                    "kill" => {
                        if let Some(actor) = &event.actor {
                            if let Some(attributes) = &actor.attributes {
                                if let Some(name) = attributes.get("name") {
                                    println!("docker Container Stoped: {name} ");
                                //    removeing the key
                                    let mut map_write = data.write().await;
                                    if let Some(_)=map_write.remove(&format!("{name}.docker.")){
                                        println!("{name} Removed from DNS list");
                                    } else{
                                        println!("{name} Not found in DNS List");
                                    }
                                }
                            }
                        }
                       
                    }
                    _ => {}
                }
            }
        }
    }
}



