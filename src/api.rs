use async_openai::{config::OpenAIConfig, Client as OpenAIClient};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use redis::Client as RedisClient;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::chat::{handle_chat_message, ChatMessage};
use crate::error::AppResult;
use crate::functions::OrderAssistant;
use crate::menu::Menu;
use crate::order::{Order, OrderItem, OrderStore};

#[derive(Debug, Serialize, Deserialize)]
pub struct StartOrderRequest {
    pub location: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartOrderResponse {
    #[serde(rename = "orderId")]
    pub order_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    #[serde(rename = "orderId")]
    pub order_id: String,
    pub input: String,
    pub location: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    #[serde(rename = "orderId")]
    pub order_id: String,
    pub order: Vec<OrderItem>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOrderResponse {
    order: Vec<OrderItem>,
    messages: Vec<ChatMessage>,
}

#[derive(Clone)]
pub struct AppState {
    store: OrderStore,
    // NOTE(dev): This enables request level control over the assistant
    #[allow(dead_code)]
    menu: Menu,
    assistant: OrderAssistant,
}

pub async fn create_router() -> Router {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    let redis_client = RedisClient::open(redis_url).expect("Failed to connect to Redis");
    let store = OrderStore::new(redis_client);

    let menu = Menu::new().expect("Failed to load menu");

    let openai_config = OpenAIConfig::new()
        .with_api_key(std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is required"));
    let mut assistant = OrderAssistant::new(OpenAIClient::with_config(openai_config));
    // TODO(siyer): Re-enable when I have perms to create assistants
    // assistant
    //     .initialize_assistant(&menu)
    //     .await
    //     .expect("Failed to initialize assistant");

    let state = AppState {
        store,
        assistant,
        menu,
    };

    Router::new()
        .route("/start", post(start_order))
        .route("/chat", post(send_chat_message))
        .route("/order/:order_id", get(get_order))
        .with_state(state)
}

async fn start_order(
    State(state): State<AppState>,
    Json(_request): Json<StartOrderRequest>,
) -> AppResult<Json<StartOrderResponse>> {
    let order_id = Uuid::new_v4().to_string();
    let mut conn = state.store.get_connection()?;

    let order = Order::new(order_id.clone());
    order.save(&mut conn).await?;

    Ok(Json(StartOrderResponse { order_id }))
}

async fn send_chat_message(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> AppResult<Json<ChatResponse>> {
    let res = handle_chat_message(&state.store, &state.assistant, &request).await?;
    Ok(Json(ChatResponse {
        order_id: request.order_id,
        order: res.order,
        messages: res.messages,
    }))
}

async fn get_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> AppResult<Json<GetOrderResponse>> {
    let mut conn = state.store.get_connection()?;
    let order = Order::get(&mut conn, &order_id)?;

    Ok(Json(GetOrderResponse {
        order: order.order,
        messages: order.messages,
    }))
}
