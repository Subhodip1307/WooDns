use bollard::Docker;
use bollard::query_parameters::ListContainersOptionsBuilder;
use std::default::Default;
use std::error::Error;
use tokio::sync::RwLock;
use std::{collections::HashMap,sync::Arc};

use bollard::query_parameters::EventsOptions;
use futures_util::stream::StreamExt;
use bollard::models::EventMessageTypeEnum;
use bollard::models::EventMessage;
use bollard::query_parameters::InspectContainerOptionsBuilder;

pub async fn  gather_docker(data:Arc<RwLock<HashMap<String,String>>>)->Result<(), Box<dyn Error>>{
    let mut write_me = data.write().await;
    let docker=Docker::connect_with_socket_defaults().unwrap();
    let options=ListContainersOptionsBuilder::default().build();

    let containers = docker.list_containers(Some(options)).await?;

    for container in containers {
        // getting the container names
        if let Some(names_array)=container.names {
            let name=names_array[0].trim_start_matches("/");//container name
            if let Some(networks)=container.network_settings.and_then(|ns|ns.networks) {
                for (_,settings) in networks {
                    if let Some(ip_address)=settings.ip_address {
                        write_me.entry(format!("{name}.docker.")/*container name*/).or_insert(ip_address); 
                        println!("adding {name}.docker.");
                    }
                }
               
            }
        }  
    }//end for loop
    Ok(())
}


pub async fn event_monitor(data:Arc<RwLock<HashMap<String,String>>>) {
    let docker = Docker::connect_with_socket_defaults().unwrap();
    let mut events = docker.events(Some(EventsOptions::default())).boxed();
    
    while let Some(Ok(event)) = events.next().await {
        
        if event.typ == Some(EventMessageTypeEnum::CONTAINER) {
            // println!("{:#?}", event);
            if let Some(ref action) = event.action {
                match action.as_str() {
                    "start"  => {
                        if let Ok(_)=handle_started_container(&event,&docker,&data).await{
                            println!("current dns {:#?}",data);
                        }//end ok 
                    }
                    "kill" | "die" | "stop" => {
                        if let Err(_) = handle_stopped_container(&event, &data).await {
                            println!("Failed to remove container from DNS");
                        }       
                    }
                    _ => {}
                }
            }
        }
    }
}



// remove the stoped containers from records list
async fn handle_stopped_container(event:&EventMessage,data:&Arc<RwLock<HashMap<String,String>>>)->Result<(), ()>{
    if let Some(actor) = &event.actor {
        if let Some(attributes) = &actor.attributes {
            if let Some(name) = attributes.get("name") {
                println!("docker Container Stoped: {name} ");
                let mut map_write = data.write().await;
                if let Some(_)=map_write.remove(&format!("{name}.docker.")){
                    println!("{name} Removed from DNS list");
                } else{
                    println!("{name} Not found in DNS List");
                }
                return Ok(());
            }
        }
    }
    Err(())
}

async fn handle_started_container(event:&EventMessage,docker: &Docker,data:&Arc<RwLock<HashMap<String,String>>>)->Result<(), ()>{
    if let Some(actor) = &event.actor {
        if let Some(attributes) = &actor.attributes {
            if let Some(name) = attributes.get("name") {
                println!("New Docker Container detected {name}");
                // getting the Ip of the new contaienr
                if let Some(container_ip_address)=get_container_ip(&docker,name).await{
                    let mut map_write = data.write().await;//write data into the stroage
                    println!("container name is {name} and it's ip is {container_ip_address}");
                    map_write.entry(format!("{name}.docker.").to_string()).or_insert(container_ip_address);  
                }

                return Ok(());
            }
        }
    }
    Err(())
}

// get ip from container name
async fn get_container_ip(docker: &Docker, container_name: &str) -> Option<String> {
    let options = InspectContainerOptionsBuilder::default()
        .build();
    let info = docker
        .inspect_container(container_name, Some(options))
        .await
        .ok()?;

    info.network_settings
        .and_then(|ns| ns.networks)
        .and_then(|nets| nets.values().find_map(|net| net.ip_address.clone()))
}