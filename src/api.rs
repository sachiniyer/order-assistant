use axum::{
    extract::Path,
    routing::{get, post},
    Json, Router,
};
use redis::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::chat::ChatMessage;
use crate::error::AppResult;
use crate::order::{Order, OrderItem, OrderStore};

#[derive(Debug, Serialize, Deserialize)]
pub struct StartOrderRequest {
    pub location: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartOrderResponse {
    pub order_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub order_id: String,
    pub input: String,
    pub location: String,
}

type ChatResponse = Order;

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOrderResponse {
    order: Vec<OrderItem>,
    messages: Vec<ChatMessage>,
}

#[derive(Clone)]
pub struct AppState {
    store: OrderStore,
}

pub fn create_router() -> Router {
    let redis_client = Client::open("redis://127.0.0.1/").expect("Failed to connect to Redis");
    let store = OrderStore::new(redis_client);
    let state = AppState { store };

    Router::new()
        .route("/start", post(start_order))
        .route("/chat", post(send_chat_message))
        .route("/order/:order_id", get(get_order))
        .with_state(state)
}

async fn start_order(
    state: axum::extract::State<AppState>,
    Json(_request): Json<StartOrderRequest>,
) -> AppResult<Json<StartOrderResponse>> {
    let order_id = Uuid::new_v4().to_string();
    let mut conn = state.store.get_connection()?;

    let order = Order::new(order_id.clone());
    order.save(&mut conn).await?;

    Ok(Json(StartOrderResponse { order_id }))
}

async fn send_chat_message(
    state: axum::extract::State<AppState>,
    Json(request): Json<ChatRequest>,
) -> AppResult<Json<ChatResponse>> {
    let response = crate::chat::handle_chat_message(&state.store, &request).await?;
    Ok(Json(response))
}

async fn get_order(
    state: axum::extract::State<AppState>,
    Path(order_id): Path<String>,
) -> AppResult<Json<GetOrderResponse>> {
    let mut conn = state.store.get_connection()?;
    let order = Order::get(&mut conn, &order_id)?;

    Ok(Json(GetOrderResponse {
        order: order.order,
        messages: order.messages,
    }))
}
