use customer_agent::api;
use dotenv::dotenv;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let app = api::create_router().await;

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
