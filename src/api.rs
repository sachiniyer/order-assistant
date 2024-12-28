use async_openai::{config::OpenAIConfig, Client as OpenAIClient};
#[allow(unused_imports)]
use axum::{
    extract::{Path, State},
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use redis::Client as RedisClient;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

use crate::chat::{handle_chat_message, ChatMessage};
use crate::error::AppResult;
use crate::functions::OrderAssistant;
use crate::menu::Menu;
use crate::order::{Order, OrderItemResponse, OrderStore};

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
    pub order: Vec<OrderItemResponse>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOrderResponse {
    order: Vec<OrderItemResponse>,
    messages: Vec<ChatMessage>,
}

/// Validates the API key from the request headers against the allowed API keys in the application state.
///
/// # Arguments
/// * `state` - Application state containing allowed API keys
/// * `req` - The incoming HTTP request
/// * `next` - The next middleware function to call if validation succeeds
///
/// # Returns
/// * `Result<Response, StatusCode>` - Success response if validated, UNAUTHORIZED status if invalid
async fn validate_api_key<B>(
    State(state): State<AppState>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("x-api-key")
        .and_then(|header| header.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = auth_header.trim_start_matches("Bearer ").trim();

    if state.api_keys.contains(token) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[derive(Clone)]
pub struct AppState {
    api_keys: Arc<HashSet<String>>,
    store: Arc<OrderStore>,
    menu: Arc<Menu>,
    assistant: Arc<TokioMutex<OrderAssistant>>,
}

/// Creates and configures the application router with all routes and middleware.
///
/// # Returns
/// * `Router` - Configured router with all routes and middleware attached
pub async fn create_router() -> Router {
    let api_keys: HashSet<String> = std::env::var("API_KEYS")
        .expect("API_KEYS environment variable is required")
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    let redis_client = RedisClient::open(redis_url).expect("Failed to connect to Redis");
    let store = OrderStore::new(redis_client);

    let menu = Menu::new().expect("Failed to load menu");

    let openai_config = OpenAIConfig::new()
        .with_api_key(std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is required"));
    let openai_client = OpenAIClient::with_config(openai_config);
    let assistant = OrderAssistant::new(openai_client);

    let assistant = Arc::new(TokioMutex::new(assistant));
    {
        let mut locked_assistant = assistant.lock().await;
        locked_assistant
            .initialize_assistant(&menu)
            .await
            .expect("Failed to initialize assistant");
    }

    let state = AppState {
        api_keys: Arc::new(api_keys),
        store: Arc::new(store),
        menu: Arc::new(menu),
        assistant,
    };

    Router::new()
        .route("/start", post(start_order))
        .route("/chat", post(send_chat_message))
        .route("/order/:order_id", get(get_order))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            validate_api_key,
        ))
        .with_state(state)
}

/// Initializes a new order and returns the order ID.
///
/// # Arguments
/// * `state` - Application state containing the order store
/// * `request` - The start order request containing location
///
/// # Returns
/// * `AppResult<Json<StartOrderResponse>>` - JSON response containing the new order ID
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

/// Processes a chat message for an order and returns the updated order state.
///
/// # Arguments
/// * `state` - Application state containing assistant and stores
/// * `request` - The chat request containing order ID and message
///
/// # Returns
/// * `AppResult<Json<ChatResponse>>` - JSON response with updated order and chat messages
async fn send_chat_message(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> AppResult<Json<ChatResponse>> {
    let assistant_lock = state.assistant.lock().await;

    let res = handle_chat_message(&state.store, &state.menu, &assistant_lock, &request).await?;
    Ok(Json(ChatResponse {
        order_id: request.order_id,
        order: res
            .order
            .iter()
            .map(|item| (*item).clone().into())
            .collect(),
        messages: res.messages,
    }))
}

/// Retrieves an existing order by ID.
///
/// # Arguments
/// * `state` - Application state containing the order store
/// * `order_id` - The ID of the order to retrieve
///
/// # Returns
/// * `AppResult<Json<GetOrderResponse>>` - JSON response containing the order details
async fn get_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> AppResult<Json<GetOrderResponse>> {
    let mut conn = state.store.get_connection()?;
    let order = Order::get(&mut conn, &order_id)?;

    Ok(Json(GetOrderResponse {
        order: order
            .order
            .iter()
            .map(|item| (*item).clone().into())
            .collect(),
        messages: order.messages,
    }))
}
