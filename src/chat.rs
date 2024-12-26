use crate::api::ChatRequest;
use crate::error::AppResult;
use crate::order::{Order, OrderItem, OrderStore};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

fn append_messages(order: &mut Order, user_msg: &str, assistant_msg: &str) {
    order.messages.push(ChatMessage {
        role: "user".to_string(),
        content: user_msg.to_string(),
    });
    order.messages.push(ChatMessage {
        role: "assistant".to_string(),
        content: assistant_msg.to_string(),
    });
}

pub async fn handle_chat_message(store: &OrderStore, request: &ChatRequest) -> AppResult<Order> {
    let mut conn = store.get_connection()?;
    let mut order = Order::get(&mut conn, &request.order_id)?;

    let input = request.input.trim().to_lowercase();

    // TODO(siyer): This needs to be written with an agent
    if input.starts_with("add ") {
        order = handle_add_item(&mut order, &request.input);
    } else if input.starts_with("remove ") {
        order = handle_remove_item(&mut order, &request.input);
    } else {
        append_messages(
            &mut order,
            &request.input,
            "I can help you add or remove items. Try 'add [item]' or 'remove [number]'",
        );
    }

    order.save(&mut conn).await?;
    Ok(order)
}

fn handle_add_item(order: &mut Order, input: &str) -> Order {
    // TODO(siyer): Instead use openai function calls to parse the input string here
    let item_name = input.trim()[4..].trim().to_string();

    let new_item = OrderItem {
        item_name: item_name.clone(),
        option_keys: Vec::new(),
        option_values: Vec::new(),
        id: Uuid::new_v4().to_string(),
        price: 0.0,
    };

    order.order.push(new_item);
    append_messages(order, input, &format!("Added {} to your order", item_name));

    order.clone()
}

fn handle_remove_item(order: &mut Order, input: &str) -> Order {
    // TODO(siyer): Instead use openai function calls to parse the input string here
    let index_str = input.trim()[7..].trim();

    match index_str.parse::<usize>() {
        Ok(i) => {
            let index = i - 1; // Redis is 1-based index
            if index < order.order.len() {
                let removed = order.order.remove(index);
                append_messages(
                    order,
                    input,
                    &format!("Removed {} from your order", removed.item_name),
                );
            } else {
                append_messages(order, input, "That item number doesn't exist in your order");
            }
        }
        Err(_) => {
            append_messages(
                order,
                input,
                "Please specify the item number to remove (e.g., 'remove 1')",
            );
        }
    }

    order.clone()
}
