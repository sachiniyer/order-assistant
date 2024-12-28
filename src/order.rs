use redis::{Client, Commands, Connection};
use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::{debug, info};

use crate::chat::ChatMessage;
use crate::error::{AppError, AppResult};
use crate::menu::ItemStatus;

/// Represents a customer's order
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    /// Unique identifier for the order
    #[serde(rename = "orderId")]
    pub order_id: String,
    /// List of items in the order
    pub order: Vec<OrderItem>,
    /// Chat message history
    pub messages: Vec<ChatMessage>,
    // NOTE(dev): Renaming this field for consistency, not because it goes through the API
    /// ID of the associated chat thread
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

/// Represents a single item in an order
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderItem {
    /// Unique identifier for the order item
    pub id: String,
    /// Name of the menu item
    #[serde(rename = "itemName")]
    pub item_name: String,
    /// Keys for the selected options
    #[serde(rename = "optionKeys")]
    pub option_keys: Vec<String>,
    /// Values for the selected options
    #[serde(rename = "optionValues")]
    pub option_values: Vec<Vec<String>>,
    /// Total price including options
    pub price: f64,
    // NOTE(dev): Renaming this field for consistency, not because it goes through the API
    /// Validation status of the item
    #[serde(rename = "itemStatus")]
    pub item_status: Option<ItemStatus>,
}

/// API response format for order items
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderItemResponse {
    /// Unique identifier for the order item
    pub id: String,
    /// Name of the menu item
    #[serde(rename = "itemName")]
    pub item_name: String,
    /// Keys for the selected options
    #[serde(rename = "optionKeys")]
    pub option_keys: Vec<String>,
    /// Values for the selected options
    #[serde(rename = "optionValues")]
    pub option_values: Vec<Vec<String>>,
    /// Total price including options
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
        debug!("Creating new order with ID: {}", order_id);
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
        debug!("Saving order {} with {} items", self.order_id, self.order.len());
        let order_json = serde_json::to_string(&self)?;
        conn.set::<_, _, ()>(&self.order_id, order_json)?;
        debug!("Order {} saved successfully", self.order_id);
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
        debug!("Retrieving order: {}", order_id);
        let order_json: Option<String> = conn.get(order_id)?;
        match order_json {
            Some(json) => {
                let order: Self = serde_json::from_str(&json)?;
                debug!("Retrieved order {} with {} items", order_id, order.order.len());
                Ok(order)
            }
            None => {
                info!("Order not found: {}", order_id);
                Err(AppError::OrderNotFound(order_id.to_string()))
            }
        }
    }
}

/// Interface for order storage operations
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
