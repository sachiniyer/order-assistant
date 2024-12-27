use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
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

#[derive(Debug, Serialize, Deserialize, Clone)]
enum ChatRole {
    User,
    Assistant,
}

impl Display for ChatRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChatRole::User => write!(f, "user"),
            ChatRole::Assistant => write!(f, "assistant"),
        }
    }
}
pub async fn handle_chat_message(
    store: &OrderStore,
    assistant: &OrderAssistant,
    request: &ChatRequest,
) -> AppResult<Order> {
    let mut conn = store.get_connection()?;
    let mut order = Order::get(&mut conn, &request.order_id)?;
    order.messages.push(ChatMessage {
        role: ChatRole::User.to_string(),
        content: request.input.clone(),
    });

    assistant
        .handle_message(&request.input, &request.location, &mut order)
        .await?;

    order.save(&mut conn).await?;
    Ok(order.clone())
}
