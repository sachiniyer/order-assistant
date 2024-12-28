use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        CreateAssistantRequestArgs, CreateMessageRequest, CreateRunRequest, CreateThreadRequest,
        FunctionObject, MessageContent, MessageRole, RunObject, RunStatus,
        SubmitToolOutputsRunRequest, ToolsOutputs,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

use crate::chat::{handle_function_call, ChatMessage, ChatRole};
use crate::error::{AppError, AppResult};
use crate::menu::Menu;
use crate::order::Order;

#[derive(Clone)]
pub struct OrderAssistant {
    client: Client<OpenAIConfig>,
    assistant: Option<String>,
}
// TODO(siyer): Build a macro to do this whole process for each of the functions
//              Something similar to https://github.com/frankfralick/openai-func-enums

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionName {
    #[serde(rename = "add_item")]
    AddItem,
    #[serde(rename = "remove_item")]
    RemoveItem,
    #[serde(rename = "modify_item")]
    ModifyItem,
}

impl Display for FunctionName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FunctionName::AddItem => write!(f, "add_item"),
            FunctionName::RemoveItem => write!(f, "remove_item"),
            FunctionName::ModifyItem => write!(f, "modify_item"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionArgs {
    AddItem {
        #[serde(rename = "itemName")]
        item_name: String,
        #[serde(rename = "optionKeys")]
        option_keys: Option<Vec<String>>,
        #[serde(rename = "optionValues")]
        option_values: Option<Vec<Vec<String>>>,
        price: f64,
    },
    RemoveItem {
        #[serde(rename = "orderId")]
        order_id: String,
    },
    ModifyItem {
        #[serde(rename = "orderId")]
        order_id: String,
        #[serde(rename = "itemName")]
        item_name: String,
        #[serde(rename = "optionKeys")]
        option_keys: Option<Vec<String>>,
        #[serde(rename = "optionValues")]
        option_values: Option<Vec<Vec<String>>>,
        price: f64,
    },
}

impl OrderAssistant {
    pub fn new(client: Client<OpenAIConfig>) -> Self {
        Self {
            client,
            assistant: None,
        }
    }

    pub async fn initialize_assistant(&mut self, menu: &Menu) -> AppResult<()> {
        let create_assistant_request = CreateAssistantRequestArgs::default()
        // TODO(siyer): Consider moving the menu to a file upload call instead of adding it to instructions
        .instructions(format!("You are an order management assistant. \
                               Use the provided functions to manage the items in orders. \
                               \n\n Use the follow menu: \n\n {}", serde_json::to_string_pretty(&menu)?))
        .model("gpt-4o")
        .tools(vec![
            FunctionObject {
                name: FunctionName::AddItem.to_string(),
                description: Some("Add an item to the order.".into()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        // TODO(siyer): Consider adding enum validation for these fields
                        //              https://platform.openai.com/docs/guides/function-calling#function-definitions
                        "itemName": { "type": "string", "description": "The name of the item to add." },
                        "optionKeys": { "type": "array",  "items": { "type": "string" }, "description": "The options for the item." },
                        "optionValues": { "type": "array", "items": { "type": "array", "items": {"type": "string"} }, "description": "The values for the options." },
                        "price": { "type": "number", "description": "The price of the item." }
                    },
                    "required": ["itemName"]
                })),
                strict: None,
            }
            .into(),
            FunctionObject {
                name: FunctionName::RemoveItem.to_string(),
                description: Some("Remove an item from the order.".into()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "orderId": { "type": "string", "description": "The id of the order item to remove from the orders list." }
                    },
                    "required": ["orderId"]
                })),
                strict: None,
            }
            .into(),
            FunctionObject {
                name: FunctionName::ModifyItem.to_string(),
                description: Some("Modify an item in the order.".into()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "orderId": { "type": "string", "description": "The id of the order item to modify from the orders list." },
                        "itemName": { "type": "string", "description": "The name of the item to modify." },
                        "optionKeys": { "type": "array",  "items": { "type": "string" }, "description": "The options for the item." },
                        "optionValues": { "type": "array", "items": { "type": "array", "items": {"type": "string"} }, "description": "The values for the options." },
                        "price": { "type": "number", "description": "The price of the item." }
                    },
                    "required": ["orderId", "itemName"]
                })),
                strict: None,
            }
            .into(),
        ])
        .build()?;

        let assistant = self
            .client
            .assistants()
            .create(create_assistant_request)
            .await?;
        self.assistant = Some(assistant.id);

        Ok(())
    }

    async fn create_thread(&self, location: &String) -> AppResult<String> {
        let thread = self
            .client
            .threads()
            .create(CreateThreadRequest::default())
            .await?;

        let _message = self
            .client
            .threads()
            .messages(&thread.id)
            .create(CreateMessageRequest {
                role: MessageRole::Assistant,
                content: format!("Welcome to {}, what can I get started for you", location).into(),
                ..Default::default()
            })
            .await?;

        Ok(thread.id)
    }

    pub async fn poll_thread(
        &self,
        thread_id: &String,
        run_id: &String,
        order: &mut Order,
    ) -> AppResult<RunObject> {
        let mut run = self
            .client
            .threads()
            .runs(&thread_id)
            .retrieve(&run_id)
            .await?;
        loop {
            match run.status {
                RunStatus::Completed => return Ok(run),
                RunStatus::Queued | RunStatus::InProgress | RunStatus::Cancelling => {
                    run = self
                        .client
                        .threads()
                        .runs(&thread_id)
                        .retrieve(&run_id)
                        .await?;
                }
                RunStatus::RequiresAction => {
                    let mut tool_outputs: Vec<ToolsOutputs> = vec![];
                    if run.required_action.is_none() {
                        return Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
                            format!("{:?}", run),
                        )));
                    };
                    let tool_calls = run
                        .required_action
                        .to_owned()
                        .unwrap()
                        .submit_tool_outputs
                        .tool_calls;
                    for tool_call in tool_calls {
                        let tool_output = handle_function_call(&tool_call.function, order).await?;
                        tool_outputs.push(ToolsOutputs {
                            tool_call_id: Some(tool_call.id),
                            output: Some(tool_output.to_string()),
                        });
                    }
                    let _response = self
                        .client
                        .threads()
                        .runs(&thread_id)
                        .submit_tool_outputs(
                            run_id,
                            SubmitToolOutputsRunRequest {
                                tool_outputs,
                                stream: Some(false),
                            },
                        )
                        .await?;
                    run = self
                        .client
                        .threads()
                        .runs(&thread_id)
                        .retrieve(&run_id)
                        .await?;
                }
                _ => {
                    return Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
                        format!("{:?}", run),
                    )))
                }
            }
            // NOTE(dev): Wait for a second before re-querying the run status
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn handle_message<'a>(
        &self,
        message: &String,
        location: &String,
        order: &'a mut Order,
    ) -> AppResult<&'a mut Order> {
        let thread_id = match &order.thread_id {
            Some(thread_id) => thread_id.clone(),
            None => self.create_thread(location).await?,
        };

        let response = self
            .client
            .threads()
            .messages(&thread_id)
            .create(CreateMessageRequest {
                role: MessageRole::User,
                content: message.clone().into(),
                ..Default::default()
            })
            .await?;
        println!("Response: {:?}", response);

        let response = self
            .client
            .threads()
            .runs(&thread_id)
            .create(CreateRunRequest {
                assistant_id: self.assistant.as_ref().unwrap().to_string(),
                stream: Some(false),
                ..Default::default()
            })
            .await?;
        println!("Response: {:?}", response);
        let run_result = self.poll_thread(&thread_id, &response.id, order).await?;
        println!("Run Result: {:?}", run_result);

        let messages = self
            .client
            .threads()
            .messages(&thread_id)
            .list(&[("limit", "1")])
            .await?;
        if let Some(message) = messages.data.get(0) {
            if let Some(MessageContent::Text(content)) = message.content.get(0) {
                let chat_message = ChatMessage {
                    role: ChatRole::Assistant.to_string(),
                    content: content.text.value.clone(),
                };
                order.messages.push(chat_message);
            }
        }

        println!("Messages: {:?}", messages);
        Ok(order)
    }
}
