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
                        "item_name": { "type": "string", "description": "The name of the item to add." }
                    },
                    "required": ["item_name"]
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
                        "item_index": { "type": "integer", "description": "The index of the item to remove." }
                    },
                    "required": ["item_index"]
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
                        "item_name": { "type": "string", "description": "The name of the item to add." }
                    },
                    "required": ["item_name"]
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
}
