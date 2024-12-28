use customer_agent::api;
use dotenv::dotenv;
use std::net::SocketAddr;
use std::str::FromStr;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Main entry point for the customer agent service.
///
/// This function:
/// 1. Loads environment variables from .env file
/// 2. Creates and configures the API router
/// 3. Starts the HTTP server on localhost:3000
#[tokio::main]
async fn main() {
    // Initialize the logging subscriber
    FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .pretty()
        .init();

    info!("Starting customer agent service");
    // Load environment variables from .env file
    dotenv().ok();

    // Create and configure the API router
    let app = api::create_router().await;

    // Configure the server address
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);
    let addr = SocketAddr::from_str(&addr).expect("Invalid address format");

    info!("Server listening on {}", addr);
    // Start the HTTP server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
