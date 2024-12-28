use redis::{Client, Commands, Connection};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::chat::ChatMessage;
use crate::error::{AppError, AppResult};
use crate::menu::ItemStatus;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    #[serde(rename = "orderId")]
    pub order_id: String,
    pub order: Vec<OrderItem>,
    pub messages: Vec<ChatMessage>,
    // NOTE(dev): Renaming this field for consistency, not because it goes through the API
    #[serde(rename = "threadId")]
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
    // NOTE(dev): Renaming this field for consistency, not because it goes through the API
    #[serde(rename = "itemStatus")]
    pub item_status: Option<ItemStatus>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderItemResponse {
    pub id: String,
    #[serde(rename = "itemName")]
    pub item_name: String,
    #[serde(rename = "optionKeys")]
    pub option_keys: Vec<String>,
    #[serde(rename = "optionValues")]
    pub option_values: Vec<Vec<String>>,
    pub price: f64,
}

impl Into<OrderItemResponse> for OrderItem {
    fn into(self) -> OrderItemResponse {
        OrderItemResponse {
            id: self.id,
            item_name: self.item_name,
            option_keys: self.option_keys,
            option_values: self.option_values,
            price: self.price,
        }
    }
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
    /// Creates a new empty order with the given ID.
    /// 
    /// # Arguments
    /// * `order_id` - The unique identifier for the order
    /// 
    /// # Returns
    /// * `Self` - A new Order instance
    pub fn new(order_id: String) -> Self {
        Self {
            order_id,
            order: Vec::new(),
            messages: Vec::new(),
            thread_id: None,
        }
    }

    /// Saves the order to Redis.
    /// 
    /// # Arguments
    /// * `conn` - Redis connection
    /// 
    /// # Returns
    /// * `AppResult<()>` - Success if saved
    pub async fn save(&self, conn: &mut Connection) -> AppResult<()> {
        let order_json = serde_json::to_string(&self)?;
        // NOTE(dev): weird typing because of https://github.com/rust-lang/rust/issues/123748
        conn.set::<_, _, ()>(&self.order_id, order_json)?;
        Ok(())
    }

    /// Retrieves an order from Redis by ID.
    /// 
    /// # Arguments
    /// * `conn` - Redis connection
    /// * `order_id` - The ID of the order to retrieve
    /// 
    /// # Returns
    /// * `AppResult<Self>` - The retrieved order or an error
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
    /// Creates a new OrderStore instance.
    /// 
    /// # Arguments
    /// * `client` - Redis client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Gets a connection from the Redis client.
    /// 
    /// # Returns
    /// * `AppResult<Connection>` - A Redis connection or an error
    pub fn get_connection(&self) -> AppResult<Connection> {
        Ok(self.client.get_connection()?)
    }
}
