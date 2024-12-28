use async_openai::{error::OpenAIError, types::FunctionCall};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use uuid::Uuid;

use crate::api::ChatRequest;
use crate::error::{AppError, AppResult};
use crate::functions::{FunctionArgs, FunctionName, OrderAssistant};
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

pub async fn handle_function_call<'a>(
    function_call: &FunctionCall,
    order: &'a mut Order,
) -> AppResult<&'a mut Order> {
    let function_name = function_call.name.clone();
    let function_args = function_call.arguments.clone();

    let function_name: FunctionName = serde_json::from_str(&function_name)?;
    let function_args: FunctionArgs = serde_json::from_str(&function_args)?;

    Ok(match (function_name, function_args.clone()) {
        (FunctionName::AddItem, FunctionArgs::AddItem { .. }) => {
            handle_add_function(&function_args, order).await?
        }
        (FunctionName::RemoveItem, FunctionArgs::RemoveItem { .. }) => {
            handle_remove_function(&function_args, order).await?
        }
        (FunctionName::ModifyItem, FunctionArgs::ModifyItem { .. }) => {
            handle_modify_function(&function_args, order).await?
        }
        _ => {
            return Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
                "Invalid function call".to_string(),
            )))
        }
    })
}

pub async fn handle_add_function<'a>(
    function_args: &FunctionArgs,
    order: &'a mut Order,
) -> AppResult<&'a mut Order> {
    if let FunctionArgs::AddItem {
        item_name,
        option_keys,
        option_values,
        price,
    } = function_args
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
        });
        return Ok(order);
    }
    Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
        "Invalid function arguments".to_string(),
    )))
}

pub async fn handle_remove_function<'a>(
    function_args: &FunctionArgs,
    order: &'a mut Order,
) -> AppResult<&'a mut Order> {
    if let FunctionArgs::RemoveItem { order_id } = function_args {
        order.order.retain(|item| item.id != *order_id);
        return Ok(order);
    }
    Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
        "Invalid function arguments".to_string(),
    )))
}

pub async fn handle_modify_function<'a>(
    function_args: &FunctionArgs,
    order: &'a mut Order,
) -> AppResult<&'a mut Order> {
    if let FunctionArgs::ModifyItem {
        order_id,
        item_name,
        option_keys,
        option_values,
        price,
    } = function_args
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
