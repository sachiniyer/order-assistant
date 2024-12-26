use customer_agent::api;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = api::create_router();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
