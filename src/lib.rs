//! Customer Agent Service
//!
//! A real-time order management system powered by AI that handles customer interactions
//! and order processing for restaurants.
//!
//! # Architecture
//!
//! The system is built using a modular architecture with the following components:
//!
//! ## Core Components
//!
//! * `api` - RESTful API endpoints using Axum framework
//! * `chat` - Chat message processing and AI interaction handling
//! * `functions` - OpenAI function definitions and assistant management
//! * `menu` - Menu configuration and item validation
//! * `order` - Order management and persistence
//! * `error` - Error handling and HTTP response mapping
//!
//! ## Design
//!
//! ### API Layer (`api.rs`)
//! - Built with Axum web framework
//! - RESTful endpoints for order management
//! - API key authentication middleware
//! - Shared application state management
//!
//! ### Storage Layer
//! - Redis for order persistence
//! - serde serialization for data storage
//!
//! ### AI Integration (`functions.rs`, `chat.rs`)
//! - Function calling for structured interactions
//! - Asynchronous message processing
//! - Thread-based conversation management
//!
//! ### Menu System (`menu.rs`)
//! - JSON-based menu configuration
//! - Rule Validation for orders
//!
//! # Environment Configuration
//!
//! The service requires several environment variables:
//!
//! ```bash
//! REDIS_URL=redis://localhost:6379    # Redis connection URL
//! OPENAI_API_KEY=your-key-here        # OpenAI API key
//! API_KEYS=key1,key2                  # Comma-separated API keys
//! MENU_FILE=static/menu.json          # Path to menu configuration
//! HOST=127.0.0.1                      # Server host
//! PORT=3000                           # Server port
//! OPENAI_MODEL=gpt-4                  # OpenAI model to use
//! RUST_LOG=info                       # Logging level
//! ```
//!
//! # Error Handling
//!
//! The service uses a custom error type (`AppError`) that handles:
//! - Redis operations
//! - JSON serialization
//! - OpenAI API calls
//! - Input validation
//! - Resource not found
//! - System errors
//!
//! # Docker Support
//!
//! Run the service using:
//! ```bash
//! docker-compose up
//! ```
//!
//! # API Endpoints
//!
//! ## POST /start
//! Initializes a new chat session for a given location with an empty order and chat state.
//!
//! ### Request
//! ```json
//! {
//!   "location": "string"  // Name of the restaurant location
//! }
//! ```
//!
//! ### Response
//! ```json
//! {
//!   "orderId": "string"  // Unique identifier for the order
//! }
//! ```
//!
//! ## POST /chat
//! Generate the next response and update the order accordingly based on your input.
//!
//! ### Request
//! ```json
//! {
//!   "orderId": "string",  // ID of the order to update
//!   "input": "string",    // Customer's message
//!   "location": "string"  // Restaurant location
//! }
//! ```
//!
//! ### Response
//! ```json
//! {
//!   "orderId": "string",
//!   "order": [
//!     {
//!       "itemName": "string",
//!       "optionKeys": ["string"],
//!       "optionValues": [["string"]],
//!       "id": "string",
//!       "price": number
//!     }
//!   ],
//!   "messages": [
//!     {
//!       "role": "user" | "assistant",
//!       "content": "string"
//!     }
//!   ]
//! }
//! ```
//!
//! ## GET /order/:order_id
//! Retrieves the current state of the order and associated chat messages for a given orderId.
//!
//! ### Response
//! ```json
//! {
//!   "order": [
//!     {
//!       "itemName": "string",
//!       "optionKeys": ["string"],
//!       "optionValues": [["string"]],
//!       "id": "string",
//!       "price": number
//!     }
//!   ],
//!   "messages": [
//!     {
//!       "role": "user" | "assistant",
//!       "content": "string"
//!     }
//!   ]
//! }
//! ```
//!
//! # Example Usage
//!
//! ```rust
//! use reqwest::Client;
//! use serde_json::json;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::new();
//!     let api_key = "your-api-key";
//!
//!     // Start a new order
//!     let start_response = client.post("http://localhost:3000/start")
//!         .header("x-api-key", format!("Bearer {}", api_key))
//!         .json(&json!({
//!             "location": "Test Location"
//!         }))
//!         .send()
//!         .await?;
//!
//!     let order = start_response.json::<serde_json::Value>().await?;
//!     let order_id = order["orderId"].as_str().unwrap();
//!
//!     // Send a chat message
//!     let chat_response = client.post("http://localhost:3000/chat")
//!         .header("x-api-key", format!("Bearer {}", api_key))
//!         .json(&json!({
//!             "orderId": order_id,
//!             "input": "I would like a cheeseburger",
//!             "location": "Test Location"
//!         }))
//!         .send()
//!         .await?;
//!
//!     // Get the order details
//!     let order_response = client.get(&format!("http://localhost:3000/order/{}", order_id))
//!         .header("x-api-key", format!("Bearer {}", api_key))
//!         .send()
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod chat;
pub mod error;
pub mod functions;
pub mod menu;
pub mod order;
