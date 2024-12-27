use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::ChatRequest;
use crate::error::AppResult;
use crate::functions::OrderAssistant;
use crate::order::{Order, OrderItem, OrderStore};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub async fn handle_chat_message(
    store: &OrderStore,
    openai: &OrderAssistant,
    request: &ChatRequest,
) -> AppResult<Order> {
    let mut conn = store.get_connection()?;
    let mut order = Order::get(&mut conn, &request.order_id)?;

    let input = request.input.trim().to_lowercase();

    // Initialize the assistant
    Ok(order)
}
