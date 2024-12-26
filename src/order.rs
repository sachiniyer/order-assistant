use crate::chat::ChatMessage;
use crate::error::{AppError, AppResult};
use redis::{Client, Commands, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub order_id: String,
    pub order: Vec<OrderItem>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderItem {
    pub id: String,
    pub item_name: String,
    pub option_keys: Vec<String>,
    pub option_values: Vec<Vec<String>>,
    pub price: f64,
}

impl Order {
    pub fn new(order_id: String) -> Self {
        Self {
            order_id,
            order: Vec::new(),
            messages: Vec::new(),
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
