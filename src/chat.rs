use async_openai::{error::OpenAIError, types::FunctionCall};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use uuid::Uuid;

use crate::api::ChatRequest;
use crate::error::{AppError, AppResult};
use crate::functions::{
    AddItemArgs, FunctionArgs, FunctionName, ListItemsArgs, ModifyItemArgs, OrderAssistant,
    RemoveItemArgs,
};
use crate::menu::Menu;
use crate::order::{Order, OrderItem, OrderStore};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChatRole {
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
    let mut conn = store.get_connection()?;
    let mut order = Order::get(&mut conn, &request.order_id)?;
    assistant
        .handle_message(&request.input, &request.location, &mut order, menu)
        .await?;

    order.save(&mut conn).await?;
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
    let function_name = function_call.name.clone();
    let function_args = function_call.arguments.clone();

    let function_name: FunctionName = serde_plain::from_str(&function_name)?;

    let function_args = match function_name {
        FunctionName::AddItem => {
            FunctionArgs::AddItem(serde_json::from_str::<AddItemArgs>(&function_args)?)
        }
        FunctionName::RemoveItem => {
            FunctionArgs::RemoveItem(serde_json::from_str::<RemoveItemArgs>(&function_args)?)
        }
        FunctionName::ModifyItem => {
            FunctionArgs::ModifyItem(serde_json::from_str::<ModifyItemArgs>(&function_args)?)
        }
        FunctionName::ListItems => {
            FunctionArgs::ListItems(serde_json::from_str::<ListItemsArgs>(&function_args)?)
        }
    };

    match (function_name, function_args.clone()) {
        (FunctionName::AddItem, FunctionArgs::AddItem { .. }) => {
            handle_add_function(&function_args, order).await?
        }
        (FunctionName::RemoveItem, FunctionArgs::RemoveItem { .. }) => {
            handle_remove_function(&function_args, order).await?
        }
        (FunctionName::ModifyItem, FunctionArgs::ModifyItem { .. }) => {
            handle_modify_function(&function_args, order).await?
        }
        (FunctionName::ListItems, FunctionArgs::ListItems { .. }) => order,
        _ => {
            return Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
                "Invalid function call".to_string(),
            )))
        }
    };
    for item in &mut order.order {
        item.item_status = Some(menu.validate_item(&item)?);
    }
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
        order.order.push(OrderItem {
            id: Uuid::new_v4().to_string(),
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
        return Ok(order);
    }
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
        order.order.retain(|item| item.id != *order_id);
        return Ok(order);
    }
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
        let item = order
            .order
            .iter_mut()
            .find(|item| item.id == *order_id)
            .ok_or(AppError::OpenAIError(OpenAIError::InvalidArgument(
                "Item not found".to_string(),
            )))?;

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
        return Ok(order);
    }
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
