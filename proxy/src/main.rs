// Proxy does 3 things per connection
// Accept the TCP connection
// Calls CheckAccess on the control plane (user + resource = backend address)
// Forwards or drops - if allowed , tokio::io::copy::bi-directional between client and backend socket
// if denied it just drops the connection

// Each connection gets its own tokio::spawn task so, accepts are non blocking

use clap::Parser;
use proto::policy::{policy_service_client::PolicyServiceClient, AccessRequest};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "0.0.0.0:8080")]
    listen: SocketAddr,

    #[arg(long, default_value = "127.0.0.1:9000")]
    backend: String,

    #[arg(long, default_value = "127.0.0.1:50051")]
    control_plane: String,

    #[arg(long, default_value = "alice")]
    user: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let listener = TcpListener::bind(args.listen).await.unwrap_or_else(|err| {
        eprintln!("ERROR: Failed to bind listener : {err}");
        std::process::exit(1);
    });
    info!("Proxy listening on {}", args.listen);

    loop {
        let (client_conn, peer_addr) = listener.accept().await.unwrap_or_else(|err| {
            eprintln!("ERROR: Failed to accept connection: {err}");
            std::process::exit(1);
        });

        let backend = args.backend.clone();
        let cp_addr = args.control_plane.clone();
        let user = args.user.clone();

        tokio::spawn(async move {
            handle_connection(client_conn, backend, cp_addr, user).await;
        });
    }
}

async fn handle_connection(
    mut client_conn: TcpStream,
    backend: String,
    cp_addr: String,
    user: String,
) {
    let mut policy_client = PolicyServiceClient::connect(cp_addr)
        .await
        .unwrap_or_else(|err| {
            eprintln!("ERROR: Could  not connect to control plane {err}");
            std::process::exit(1);
        });

    let response = policy_client
        .check_access(AccessRequest {
            user_id: user.clone(),
            resource: backend.clone(),
            action: "connect".to_string(),
            token: String::new(),
        })
        .await
        .unwrap_or_else(|err| {
            eprintln!("ERROR: Check Access failed: {err}");
            std::process::exit(1);
        })
        .into_inner();

    if !response.allowed {
        warn!(user, resource = backend, "DENY - closing connection");
        return;
    }

    info!(user, resource = backend, "ALLOW - forwarding ");

    let mut backend_conn = TcpStream::connect(&backend).await.unwrap_or_else(|err| {
        eprintln!("ERROR: Could not connect to backend {err}");
        std::process::exit(1);
    });

    tokio::io::copy_bidirectional(&mut client_conn, &mut backend_conn)
        .await
        .unwrap_or_else(|err| {
            warn!("ERROR: Connection ended with error {err}");
            (0, 0)
        });
}
