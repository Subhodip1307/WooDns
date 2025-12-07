use crate::dns_handler::RemoteDnsCache;
use crate::storage_system::DockerStorage;
use bytes::Bytes;
use dashmap::DashMap;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use std::{convert::Infallible, io, path::Path, sync::Arc};
use tokio::{fs, net::UnixListener};

fn json_body(s: &'static str) -> Full<Bytes> {
    // s should be valid JSON; we’re just sending bytes.
    Full::new(Bytes::from_static(s.as_bytes()))
}
// curl --unix-socket uds-http.sock http://localhost/ld

pub async fn access(
    remote: Arc<DashMap<String, RemoteDnsCache>>,
    docker: Arc<DockerStorage>,
) -> io::Result<()> {
    let sock_path = "./uds-http.sock";

    if Path::new(sock_path).exists() {
        fs::remove_file(sock_path).await?;
    }

    let listener = UnixListener::bind(sock_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(sock_path)?.permissions();
        perms.set_mode(0o666); // adjust as needed
        fs::set_permissions(sock_path, perms).await?;
    }

    loop {
        let d1_service = Arc::clone(&docker);
        let r1_service = Arc::clone(&remote);
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            let d1_service = Arc::clone(&d1_service);
            let svc = service_fn(move |req: Request<Incoming>| {
                let d1 = Arc::clone(&d1_service);
                let r1 = Arc::clone(&r1_service);
                async move {
                    let (status, body, ctype) = match (req.method(), req.uri().path()) {
                        (&Method::GET, "/ld") => {
                            let mut all_entries = String::new();
                            let data = d1.list_all().await;
                            data.into_iter().for_each(|(k, v)| {
                                all_entries.push_str(&format!("{} => {} \n", k, v));
                            });

                            let let_the = Full::new(Bytes::from(all_entries));
                            (200, let_the, "application/txt")
                        }
                        (&Method::GET, "/lr") => {
                            let mut all_entries = String::new();
                            r1.iter().for_each(|k| {
                                all_entries.push_str(&format!("{} \n", k.key().clone()));
                            });
                            let let_the = Full::new(Bytes::from(all_entries));
                            (200, let_the, "application/txt")
                        }
                        _ => (
                            200,
                            json_body("Method:\tGET\nURL\t/ld\t/lr"),
                            "application/txt",
                        ),
                    };

                    let resp = Response::builder()
                        .status(status)
                        .header("content-type", ctype)
                        .body(body)
                        .unwrap();

                    Ok::<_, Infallible>(resp)
                }
            });

            let io = TokioIo::new(stream);
            if let Err(err) = http1::Builder::new().serve_connection(io, svc).await {
                eprintln!("connection error: {err}");
            }
        });
    }
}
