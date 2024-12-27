use async_openai::{
    config::OpenAIConfig,
    types::{
        AssistantStreamEvent, CreateAssistantRequestArgs, CreateMessageRequest,
        CreateMessageRequestContent, CreateRunRequest, CreateThreadRequest, FunctionObject,
        MessageDeltaContent, MessageRole, RunObject, SubmitToolOutputsRunRequest, ToolsOutputs,
    },
    Client,
};

use crate::error::AppResult;
use crate::menu::Menu;
use crate::order::Order;

#[derive(Clone)]
pub struct OrderAssistant {
    client: Client<OpenAIConfig>,
    assistant: Option<String>,
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
                name: "add_item".into(),
                description: Some("Add an item to the order.".into()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        // TODO(siyer): Add enum validation for these fields (https://platform.openai.com/docs/guides/function-calling#function-definitions)
                        // Pretty sure, I can map directly from menu.json into these fields
                        "itemName": { "type": "string", "description": "The name of the item to add." },
                        "optionKeys": { "type": "array",  "items": { "type": "string" }, "description": "The options for the item." },
                        "optionValues": { "type": "array", "items": { "type": "string" }, "description": "The values for the options." }
                    },
                    "required": ["itemName"]
                })),
                strict: None,
            }
            .into(),
            FunctionObject {
                name: "remove_item".into(),
                description: Some("Remove an item from the order.".into()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "orderId": { "type": "integer", "description": "The id of the order item to remove from the orders list." }
                    },
                    "required": ["orderId"]
                })),
                strict: None,
            }
            .into(),
            FunctionObject {
                name: "modify_item".into(),
                description: Some("Modify an item in the order.".into()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "orderId": { "type": "integer", "description": "The id of the order item to modify from the orders list." },
                        "itemName": { "type": "string", "description": "The name of the item to modify." },
                        "optionKeys": { "type": "array",  "items": { "type": "string" }, "description": "The options for the item." },
                        "optionValues": { "type": "array", "items": { "type": "string" }, "description": "The values for the options." }
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

        Ok(order)
    }
}
