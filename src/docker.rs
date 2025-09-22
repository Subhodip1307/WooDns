use crate::loggin::DnsLogger;
use crate::storage_system::DockerStorage;
use bollard::Docker;
use bollard::models::EventMessage;
use bollard::models::EventMessageTypeEnum;
use bollard::query_parameters::EventsOptions;
use bollard::query_parameters::InspectContainerOptionsBuilder;
use bollard::query_parameters::ListContainersOptionsBuilder;
use futures_util::stream::StreamExt;
use std::default::Default;
use std::error::Error;
use std::sync::Arc;

pub async fn gather_docker(
    data: Arc<DockerStorage>,
    logger: Arc<DnsLogger>,
) -> Result<(), Box<dyn Error>> {
    let docker = Docker::connect_with_socket_defaults().unwrap();
    let options = ListContainersOptionsBuilder::default().all(false).build();

    let containers = docker.list_containers(Some(options)).await?;

    for container in containers {
        // getting the container names
        if let Some(names_array) = container.names {
            let name = names_array[0].trim_start_matches("/"); //container name
            if let Some(networks) = container.network_settings.and_then(|ns| ns.networks) {
                for (_, settings) in networks {
                    if let Some(ip_address) = settings.ip_address {
                        data.set(format!("{name}.docker."), ip_address).await;
                        logger.log(&format!("adding {name}.docker.")).await;
                    }
                }
            }
        }
    } //end for loop
    logger.log("Docker discovery complete").await;
    Ok(())
}

pub async fn event_monitor(data: Arc<DockerStorage>, event_logger: Arc<DnsLogger>) {
    let docker = Docker::connect_with_socket_defaults().unwrap();
    let mut events = docker.events(Some(EventsOptions::default())).boxed();

    while let Some(Ok(event)) = events.next().await {
        let event_log = Arc::clone(&event_logger);

        if event.typ == Some(EventMessageTypeEnum::CONTAINER)
            && let Some(ref action) = event.action
        {
            match action.as_str() {
                "start" => {
                    if (handle_started_container(&event, &docker, &data, &event_log).await).is_ok()
                    {
                        event_log.log("DNS Record Updated").await;
                    } //end ok 
                }
                "kill" | "die" | "stop" => {
                    if (handle_stopped_container(&event, &data, &event_log).await).is_err() {
                        event_log.log("Failed to remove container from DNS").await;
                    }
                }
                _ => {}
            }
        }
    }
}

// remove the stoped containers from records list
async fn handle_stopped_container(
    event: &EventMessage,
    data: &Arc<DockerStorage>,
    logger: &Arc<DnsLogger>,
) -> Result<(), ()> {
    if let Some(actor) = &event.actor
        && let Some(attributes) = &actor.attributes
        && let Some(name) = attributes.get("name")
    {
        // name = stoped container name
        let remove_data: bool = { data.remove(&format!("{name}.docker.")).await };
        if remove_data {
            logger
                .log(&format!("docker Container Stoped: {name} "))
                .await;
            logger.log(&format!("{name} Removed from DNS list")).await;
        } else {
            println!("{name} Not found in DNS List");
        }
        return Ok(());
    }

    Err(())
}

async fn handle_started_container(
    event: &EventMessage,
    docker: &Docker,
    data: &Arc<DockerStorage>,
    logger: &Arc<DnsLogger>,
) -> Result<(), ()> {
    if let Some(actor) = &event.actor
        && let Some(attributes) = &actor.attributes
        && let Some(name) = attributes.get("name")
    {
        logger
            .log(&format!("New Docker Container detected {name}"))
            .await;
        // getting the Ip of the new contaienr
        if let Some(container_ip_address) = get_container_ip(docker, name).await {
            logger
                .log(&format!(
                    "container name is {name} and it's ip is {container_ip_address}"
                ))
                .await;
            data.set(format!("{name}.docker."), container_ip_address)
                .await; //write data
        }

        return Ok(());
    }

    Err(())
}

// get ip from container name
async fn get_container_ip(docker: &Docker, container_name: &str) -> Option<String> {
    let options = InspectContainerOptionsBuilder::default().build();
    let info = docker
        .inspect_container(container_name, Some(options))
        .await
        .ok()?;

    info.network_settings
        .and_then(|ns| ns.networks)
        .and_then(|nets| nets.values().find_map(|net| net.ip_address.clone()))
}
