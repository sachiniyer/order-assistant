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
use tracing::{debug, error, info};

use crate::chat::{handle_function_call, ChatMessage, ChatRole};
use crate::error::{AppError, AppResult};
use crate::menu::Menu;
use crate::order::Order;

// TODO(siyer): Build a macro to do this whole process for each of the functions
//              Something similar to https://github.com/frankfralick/openai-func-enums

/// Available function names for the AI assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionName {
    /// Function to add an item to the order
    ///
    #[serde(rename = "add_item")]
    AddItem,
    /// Function to remove an item from the order
    #[serde(rename = "remove_item")]
    RemoveItem,
    /// Function to modify an existing item
    #[serde(rename = "modify_item")]
    ModifyItem,
    /// Function to list items in the order
    #[serde(rename = "list_items")]
    ListItems,
}

impl Display for FunctionName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FunctionName::AddItem => write!(f, "add_item"),
            FunctionName::RemoveItem => write!(f, "remove_item"),
            FunctionName::ModifyItem => write!(f, "modify_item"),
            FunctionName::ListItems => write!(f, "list_items"),
        }
    }
}

// NOTE(dev): Extra verbosity in structs is to enable strict deserialization based on function name
/// Arguments for adding an item to the order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddItemArgs {
    /// Name of the item to add
    #[serde(rename = "itemName")]
    pub item_name: String,
    /// Optional customization keys
    #[serde(rename = "optionKeys")]
    pub option_keys: Option<Vec<String>>,
    /// Values for the customization options
    #[serde(rename = "optionValues")]
    pub option_values: Option<Vec<Vec<String>>>,
    // TODO(siyer): Could just calculate price using menu.rs, but trusting GPT for now
    /// Price of the item with options
    pub price: f64,
}

/// Arguments for removing an item from the order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveItemArgs {
    /// ID of the order item to remove
    #[serde(rename = "orderId")]
    pub order_id: String,
}

/// Arguments for modifying an existing item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyItemArgs {
    /// ID of the order item to modify
    #[serde(rename = "orderId")]
    pub order_id: String,
    /// New name for the item
    #[serde(rename = "itemName")]
    pub item_name: String,
    /// New option keys
    #[serde(rename = "optionKeys")]
    pub option_keys: Option<Vec<String>>,
    /// New option values
    #[serde(rename = "optionValues")]
    pub option_values: Option<Vec<Vec<String>>>,
    // TODO(siyer): Could just calculate price using menu.rs, but trusting GPT for now
    /// Updated price
    pub price: f64,
}

/// Arguments for listing items in the order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItemsArgs {
    /// Optional limit on number of items to return
    pub limit: Option<usize>,
}

/// Possible function arguments for the AI assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionArgs {
    /// Arguments for adding an item
    AddItem(AddItemArgs),
    /// Arguments for removing an item
    RemoveItem(RemoveItemArgs),
    /// Arguments for modifying an item
    ModifyItem(ModifyItemArgs),
    /// Arguments for listing items
    ListItems(ListItemsArgs),
}

/// AI assistant for managing orders
#[derive(Clone)]
pub struct OrderAssistant {
    client: Client<OpenAIConfig>,
    assistant: Option<String>,
}

impl OrderAssistant {
    /// Creates a new OrderAssistant instance.
    ///
    /// # Arguments
    /// * `client` - The OpenAI API client
    pub fn new(client: Client<OpenAIConfig>) -> Self {
        debug!("Creating new OrderAssistant instance");
        Self {
            client,
            assistant: None,
        }
    }

    /// Initializes the AI assistant with the restaurant menu and function definitions.
    ///
    /// # Arguments
    /// * `menu` - The restaurant menu to train the assistant with
    ///
    /// # Returns
    /// * `AppResult<()>` - Success if initialization completes
    pub async fn initialize_assistant(&mut self, menu: &Menu) -> AppResult<()> {
        info!("Initializing AI assistant with menu");
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
        debug!("Using OpenAI model: {}", model);
        let create_assistant_request = CreateAssistantRequestArgs::default()
        // TODO(siyer): Consider moving the menu to a file upload call instead of adding it to instructions
        .instructions(format!("You are an order management assistant.
                               - Talk as if you were taking orders in a drive thru.
                               - Use the provided functions to manage the items in orders.
                               - Ensure that every item has all of its requirements met and contains the Completed status
                               - Try to parallelize the tool calls as much as possible (e.g. submit all 5 additions at the same time)
                               - At the end of the conversation give the final price of the items in the cart
                               Use the follow menu: \n\n {}", serde_json::to_string_pretty(&menu)?))
        .model(model)
        .tools(vec![
            FunctionObject {
                name: FunctionName::AddItem.to_string(),
                description: Some("Add an item to the order.".into()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        // TODO(siyer): Consider adding enum validation for these fields
                        //              https://platform.openai.com/docs/guides/function-calling#function-definitions
                        // TODO(siyer): Figure out how to force gpt to call functions parallelly (it has the capabilities to do so)
                        //              If I can't figure out prompting, change the function definition to take an array instead
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
            FunctionObject {
                name: FunctionName::ListItems.to_string(),
                description: Some("List all the items in the order.".into()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "number", "description": "Optional field to limit to the amount of items to list that should default to false unless under token pressure" }
                    },
                    "required": []
                })),
                strict: None,
            }.into(),
        ])
        .build()?;

        debug!("Creating assistant with OpenAI API");
        let assistant = self
            .client
            .assistants()
            .create(create_assistant_request)
            .await?;
        self.assistant = Some(assistant.id);
        info!("AI assistant initialized successfully");

        Ok(())
    }

    /// Creates a new conversation thread with the assistant.
    ///
    /// # Arguments
    /// * `location` - The restaurant location
    ///
    /// # Returns
    /// * `AppResult<String>` - The ID of the created thread
    pub async fn create_thread(&self, location: &String) -> AppResult<String> {
        debug!("Creating new thread for location: {}", location);
        let thread = self
            .client
            .threads()
            .create(CreateThreadRequest::default())
            .await?;
        debug!("Created thread with ID: {}", thread.id);
        Ok(thread.id)
    }

    /// Polls the assistant thread until completion or action required.
    ///
    /// # Arguments
    /// * `thread_id` - The conversation thread ID
    /// * `run_id` - The current run ID
    /// * `order` - The current order state
    /// * `menu` - The restaurant menu
    ///
    /// # Returns
    /// * `AppResult<RunObject>` - The final run state
    pub async fn poll_thread(
        &self,
        thread_id: &String,
        run_id: &String,
        order: &mut Order,
        menu: &Menu,
    ) -> AppResult<RunObject> {
        debug!(
            "Starting to poll thread. Thread ID: {}, Run ID: {}, Order ID: {}",
            thread_id, run_id, order.order_id
        );
        let mut run = self
            .client
            .threads()
            .runs(thread_id)
            .retrieve(run_id)
            .await?;
        loop {
            match run.status {
                RunStatus::Completed => {
                    info!(
                        "Run completed successfully. Thread ID: {}, Run ID: {}, Order ID: {}",
                        thread_id, run_id, order.order_id
                    );
                    return Ok(run);
                }
                RunStatus::Queued | RunStatus::InProgress | RunStatus::Cancelling => {
                    debug!("Run {} in state: {:?}", run_id, run.status);
                    run = self
                        .client
                        .threads()
                        .runs(thread_id)
                        .retrieve(run_id)
                        .await?;
                }
                RunStatus::RequiresAction => {
                    info!(
                        "Run requires action. Thread ID: {}, Run ID: {}, Order ID: {}",
                        thread_id, run_id, order.order_id
                    );
                    let mut tool_outputs: Vec<ToolsOutputs> = vec![];
                    if run.required_action.is_none() {
                        error!("Run {} requires action but no action specified", run.id);
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
                    debug!("Processing {} tool calls", tool_calls.len());
                    for tool_call in tool_calls {
                        debug!(
                            "Executing tool call: {} (ID: {}) for Order ID: {}",
                            tool_call.function.name, tool_call.id, order.order_id
                        );
                        let tool_output =
                            handle_function_call(&tool_call.function, menu, order).await?;
                        tool_outputs.push(ToolsOutputs {
                            tool_call_id: Some(tool_call.id),
                            output: Some(tool_output.to_string()),
                        });
                    }
                    debug!("Submitting {} tool outputs", tool_outputs.len());
                    let _response = self
                        .client
                        .threads()
                        .runs(thread_id)
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
                        .runs(thread_id)
                        .retrieve(run_id)
                        .await?;
                }
                _ => {
                    error!(
                        "Run in unexpected state: {:?}. Thread ID: {}, Run ID: {}, Order ID: {}",
                        run.status, thread_id, run_id, order.order_id
                    );
                    return Err(AppError::OpenAIError(OpenAIError::InvalidArgument(
                        format!("{:?}", run),
                    )));
                }
            }
            debug!("Waiting before next poll for run {}", run_id);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    /// Processes a chat message through the AI assistant.
    ///
    /// # Arguments
    /// * `message` - The user's message
    /// * `location` - The restaurant location
    /// * `order` - The current order state
    /// * `menu` - The restaurant menu
    ///
    /// # Returns
    /// * `AppResult<&mut Order>` - The updated order after processing
    pub async fn handle_message<'a>(
        &self,
        message: &str,
        location: &String,
        order: &'a mut Order,
        menu: &Menu,
    ) -> AppResult<&'a mut Order> {
        info!(
            "Processing message for Order ID: {} at location: {}",
            order.order_id, location
        );

        let thread_id = match &order.thread_id {
            Some(thread_id) => {
                debug!(
                    "Using existing thread. Thread ID: {}, Order ID: {}",
                    thread_id, order.order_id
                );
                thread_id.clone()
            }
            None => {
                info!(
                    "Creating new thread for Order ID: {} at location: {}",
                    order.order_id, location
                );
                let chat_message = ChatMessage {
                    role: ChatRole::Assistant.to_string(),
                    content: format!("Welcome to {}, what can I get started for you", location),
                };
                order.messages.push(chat_message);
                let thread_id = self.create_thread(location).await?;
                debug!(
                    "Created new thread. Thread ID: {}, Order ID: {}",
                    thread_id, order.order_id
                );
                order.thread_id = Some(thread_id.clone());
                thread_id
            }
        };

        debug!("Adding user message to order history");
        order.messages.push(ChatMessage {
            role: ChatRole::User.to_string(),
            content: message.to_owned(),
        });

        debug!(
            "Creating message in OpenAI thread. Thread ID: {}, Order ID: {}",
            thread_id, order.order_id
        );
        let _response = self
            .client
            .threads()
            .messages(&thread_id)
            .create(CreateMessageRequest {
                role: MessageRole::User,
                content: message.to_owned().into(),
                ..Default::default()
            })
            .await?;

        info!("Creating new run for thread {}", thread_id);
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
        debug!("Created run: {}", response.id);

        let _run_result = self
            .poll_thread(&thread_id, &response.id, order, menu)
            .await?;

        debug!("Retrieving latest message from thread");
        let messages = self
            .client
            .threads()
            .messages(&thread_id)
            .list(&[("limit", "1")])
            .await?;

        if let Some(message) = messages.data.first() {
            if let Some(MessageContent::Text(content)) = message.content.first() {
                debug!("Processing assistant response: {}", content.text.value);
                let _response = self
                    .client
                    .threads()
                    .messages(&thread_id)
                    .create(CreateMessageRequest {
                        role: MessageRole::Assistant,
                        content: content.text.value.clone().into(),
                        ..Default::default()
                    })
                    .await?;

                let chat_message = ChatMessage {
                    role: ChatRole::Assistant.to_string(),
                    content: content.text.value.clone(),
                };
                order.messages.push(chat_message);
                debug!("Added assistant response to order history");
            }
        }

        info!(
            "Message processing completed. Thread ID: {}, Order ID: {}",
            thread_id, order.order_id
        );
        Ok(order)
    }
}
