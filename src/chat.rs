use async_openai::{error::OpenAIError, types::FunctionCall};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::api::ChatRequest;
use crate::error::{AppError, AppResult};
use crate::functions::{
    AddItemArgs, FunctionArgs, FunctionName, ListItemsArgs, ModifyItemArgs, OrderAssistant,
    RemoveItemArgs,
};
use crate::menu::Menu;
use crate::order::{Order, OrderItem, OrderStore};

/// Represents a single message in the chat conversation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    /// The role of who sent the message (user/assistant)
    pub role: String,
    /// The content of the message
    pub content: String,
}

/// Represents the possible roles in a chat conversation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChatRole {
    /// Message from the user
    User,
    /// Message from the AI assistant
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

/// Processes a chat message and updates the order state accordingly.
///
/// # Arguments
/// * `store` - The order storage interface
/// * `menu` - The restaurant menu
/// * `assistant` - The AI assistant instance
/// * `request` - The chat request containing the message
///
/// # Returns
/// * `AppResult<Order>` - The updated order after processing the message
pub async fn handle_chat_message(
    store: &OrderStore,
    menu: &Menu,
    assistant: &OrderAssistant,
    request: &ChatRequest,
) -> AppResult<Order> {
    info!("Processing chat message for order: {}", request.order_id);
    debug!("Chat input: {}", request.input);

    let mut conn = store.get_connection()?;
    debug!("Retrieving order from storage");
    let mut order = Order::get(&mut conn, &request.order_id)?;

    info!("Handling message with AI assistant");
    assistant
        .handle_message(&request.input, &request.location, &mut order, menu)
        .await?;

    debug!("Saving updated order to storage");
    order.save(&mut conn).await?;
    info!("Chat message processing completed");
    Ok(order.clone())
}

/// Handles function calls from the AI assistant and updates the order accordingly.
///
/// # Arguments
/// * `function_call` - The function call details from the assistant
/// * `menu` - The restaurant menu
/// * `order` - The current order state
///
/// # Returns
/// * `AppResult<&mut Order>` - The updated order after executing the function
pub async fn handle_function_call<'a>(
    function_call: &FunctionCall,
    menu: &Menu,
    order: &'a mut Order,
) -> AppResult<&'a mut Order> {
    info!("Processing function call: {}", function_call.name);
    let function_name = function_call.name.clone();
    let function_args = function_call.arguments.clone();

    debug!("Parsing function name: {}", function_name);
    let function_name: FunctionName = serde_plain::from_str(&function_name)?;

    debug!("Parsing function arguments: {}", function_args);
    let function_args = match function_name {
        FunctionName::AddItem => {
            debug!("Parsing AddItem arguments");
            FunctionArgs::AddItem(serde_json::from_str::<AddItemArgs>(&function_args)?)
        }
        FunctionName::RemoveItem => {
            debug!("Parsing RemoveItem arguments");
            FunctionArgs::RemoveItem(serde_json::from_str::<RemoveItemArgs>(&function_args)?)
        }
        FunctionName::ModifyItem => {
            debug!("Parsing ModifyItem arguments");
            FunctionArgs::ModifyItem(serde_json::from_str::<ModifyItemArgs>(&function_args)?)
        }
        FunctionName::ListItems => {
            debug!("Parsing ListItems arguments");
            FunctionArgs::ListItems(serde_json::from_str::<ListItemsArgs>(&function_args)?)
        }
    };

    info!("Executing function: {:?}", function_name.clone());
    match (function_name.clone(), function_args.clone()) {
        (FunctionName::AddItem, FunctionArgs::AddItem { .. }) => {
            handle_add_function(&function_args, order).await?
        }
        (FunctionName::RemoveItem, FunctionArgs::RemoveItem { .. }) => {
            handle_remove_function(&function_args, order).await?
        }
        (FunctionName::ModifyItem, FunctionArgs::ModifyItem { .. }) => {
            handle_modify_function(&function_args, order).await?
        }
        (FunctionName::ListItems, FunctionArgs::ListItems { .. }) => {
            handle_list_function(&function_args, order).await?
        }
        _ => {
            error!("Invalid function call combination: {:?}", function_name);
            return Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
                "Invalid function call".to_string(),
            )));
        }
    };
    debug!("Validating order items {:?}", order);
    for item in &mut order.order {
        item.item_status = Some(menu.validate_item(&item.to_owned())?);
    }
    debug!("Validated order items {:?}", order);

    info!("Function execution completed successfully");
    Ok(order)
}

/// Processes an add item function call.
///
/// # Arguments
/// * `function_args` - The arguments for adding an item
/// * `order` - The current order state
///
/// # Returns
/// * `AppResult<&mut Order>` - The updated order with the new item
pub async fn handle_add_function<'a>(
    function_args: &FunctionArgs,
    order: &'a mut Order,
) -> AppResult<&'a mut Order> {
    if let FunctionArgs::AddItem(AddItemArgs {
        item_name,
        option_keys,
        option_values,
        price,
    }) = function_args
    {
        info!("Adding item '{}' to order", item_name);
        debug!(
            "Item details - Price: {}, Options: {:?}",
            price, option_keys
        );

        let item_id = Uuid::new_v4().to_string();
        debug!("Generated item ID: {}", item_id);

        order.order.push(OrderItem {
            id: item_id.clone(),
            item_name: item_name.clone(),
            option_keys: match option_keys {
                Some(keys) => keys.clone(),
                None => vec![],
            },
            option_values: match option_values {
                Some(values) => values.clone(),
                None => vec![],
            },
            price: *price,
            item_status: None,
        });
        info!("Successfully added item {} to order", item_id);
        return Ok(order);
    }
    error!("Invalid arguments for add_item function");
    Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
        "Invalid function arguments".to_string(),
    )))
}

/// Processes a remove item function call.
///
/// # Arguments
/// * `function_args` - The arguments for removing an item
/// * `order` - The current order state
///
/// # Returns
/// * `AppResult<&mut Order>` - The updated order with the item removed
pub async fn handle_remove_function<'a>(
    function_args: &FunctionArgs,
    order: &'a mut Order,
) -> AppResult<&'a mut Order> {
    if let FunctionArgs::RemoveItem(RemoveItemArgs { order_id }) = function_args {
        info!("Removing item {} from order", order_id);
        let initial_count = order.order.len();
        order.order.retain(|item| item.id != *order_id);
        let removed_count = initial_count - order.order.len();
        debug!("Removed {} items from order", removed_count);
        return Ok(order);
    }
    error!("Invalid arguments for remove_item function");
    Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
        "Invalid function arguments".to_string(),
    )))
}

/// Processes a modify item function call.
///
/// # Arguments
/// * `function_args` - The arguments for modifying an item
/// * `order` - The current order state
///
/// # Returns
/// * `AppResult<&mut Order>` - The updated order with the modified item
pub async fn handle_modify_function<'a>(
    function_args: &FunctionArgs,
    order: &'a mut Order,
) -> AppResult<&'a mut Order> {
    if let FunctionArgs::ModifyItem(ModifyItemArgs {
        order_id,
        item_name,
        option_keys,
        option_values,
        price,
    }) = function_args
    {
        info!("Modifying item {} in order", order_id);
        debug!("New values - Name: {}, Price: {}", item_name, price);

        let item = order
            .order
            .iter_mut()
            .find(|item| item.id == *order_id)
            .ok_or(AppError::OpenAIError(OpenAIError::InvalidArgument(
                "Item not found".to_string(),
            )))?;

        debug!("Updating item properties");
        item.item_name = item_name.clone();
        item.option_keys = match option_keys {
            Some(keys) => keys.clone(),
            None => vec![],
        };
        item.option_values = match option_values {
            Some(values) => values.clone(),
            None => vec![],
        };
        item.price = *price;
        info!("Successfully modified item {}", order_id);
        return Ok(order);
    }
    error!("Invalid arguments for modify_item function");
    Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
        "Invalid function arguments".to_string(),
    )))
}

/// Processes a list items function call.
///
/// # Arguments
/// * `function_args` - The arguments for listing items
/// * `order` - The current order state
///
/// # Returns
/// * `AppResult<&mut Order>` - The order with potentially filtered items
pub async fn handle_list_function<'a>(
    function_args: &FunctionArgs,
    order: &'a mut Order,
) -> AppResult<&'a mut Order> {
    if let FunctionArgs::ListItems(ListItemsArgs { limit }) = function_args {
        if let Some(limit) = limit {
            order.order = order.order.iter().take(*limit).cloned().collect();
        }
        return Ok(order);
    }
    Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
        "Invalid function arguments".to_string(),
    )))
}
