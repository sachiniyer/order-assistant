use customer_agent::api;
use dotenv::dotenv;
use std::net::SocketAddr;

/// Main entry point for the customer agent service.
///
/// This function:
/// 1. Loads environment variables from .env file
/// 2. Creates and configures the API router
/// 3. Starts the HTTP server on localhost:3000
#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv().ok();

    // Create and configure the API router
    let app = api::create_router().await;

    // Configure the server address
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // Start the HTTP server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
