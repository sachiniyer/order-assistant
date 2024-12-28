use redis::{Client, Commands, Connection};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::chat::ChatMessage;
use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    #[serde(rename = "orderId")]
    pub order_id: String,
    pub order: Vec<OrderItem>,
    pub messages: Vec<ChatMessage>,
    // NOTE(dev): Renaming this field for consistency, not because it goes through the API
    pub thread_id: Option<String>,
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "Order"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderItem {
    pub id: String,
    #[serde(rename = "itemName")]
    pub item_name: String,
    #[serde(rename = "optionKeys")]
    pub option_keys: Vec<String>,
    #[serde(rename = "optionValues")]
    pub option_values: Vec<Vec<String>>,
    pub price: f64,
}

impl fmt::Display for OrderItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "OrderItem"),
        }
    }
}

impl Order {
    pub fn new(order_id: String) -> Self {
        Self {
            order_id,
            order: Vec::new(),
            messages: Vec::new(),
            thread_id: None,
        }
    }

    pub async fn save(&self, conn: &mut Connection) -> AppResult<()> {
        let order_json = serde_json::to_string(&self)?;
        // NOTE(dev): weird typing because of https://github.com/rust-lang/rust/issues/123748
        conn.set::<_, _, ()>(&self.order_id, order_json)?;
        Ok(())
    }

    pub fn get(conn: &mut Connection, order_id: &str) -> AppResult<Self> {
        let order_json: Option<String> = conn.get(order_id)?;
        match order_json {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Err(AppError::OrderNotFound(order_id.to_string())),
        }
    }
}

#[derive(Clone)]
pub struct OrderStore {
    client: Client,
}

impl OrderStore {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn get_connection(&self) -> AppResult<Connection> {
        Ok(self.client.get_connection()?)
    }
}
