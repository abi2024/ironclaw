use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestUserMessageArgs,
        ChatCompletionTool, ChatCompletionToolArgs, ChatCompletionToolType,
        FunctionObjectArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use anyhow::{Result, Context};
use tracing::info;
use serde_json::Value;

// Import our internal Tool Definition
use crate::registry::ToolRecord; 

pub struct Brain {
    client: Client<OpenAIConfig>,
    model: String,
}

impl Brain {
    pub fn new() -> Result<Self> {
        dotenvy::dotenv().ok(); 
        let api_key = std::env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY must be set in .env")?;
        let model = std::env::var("OPENAI_MODEL")
            .unwrap_or_else(|_| "gpt-4o".to_string());

        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);

        info!("Brain connected. Model: {}", model);
        Ok(Self { client, model })
    }

    pub async fn say_hello(&self) -> Result<String> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages([
                ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content("Hello! Reply with 'System Online'.")
                        .build()?
                )
            ])
            .build()?;

        let response = self.client.chat().create(request).await?;
        Ok(response.choices[0].message.content.clone().unwrap_or_default())
    }

    // --- NEW: The Planner ---
    // This function takes a user's task and the list of available tools.
    // It asks the LLM to decide if a tool should be called.
    pub async fn plan(&self, task: &str, tools: &[ToolRecord]) -> Result<Option<Value>> {
        
        // 1. Convert our Registry Tools -> OpenAI Tools
        let openai_tools: Vec<ChatCompletionTool> = tools.iter().map(|t| {
            ChatCompletionToolArgs::default()
                .r#type(ChatCompletionToolType::Function)
                .function(
                    FunctionObjectArgs::default()
                        .name(&t.name)
                        .description(&t.description)
                        .parameters(t.parameters.clone()) // Pass the JSON Schema directly
                        .build()
                        .unwrap()
                )
                .build()
                .unwrap()
        }).collect();

        // 2. Prepare the Request
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages([
                ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(task)
                        .build()?
                )
            ])
            .tools(openai_tools) // <--- Give the LLM the menu
            .build()?;

        // 3. Send to AI
        let response = self.client.chat().create(request).await?;
        let choice = &response.choices[0];

        // 4. Check if the AI wants to use a tool
        if let Some(tool_calls) = &choice.message.tool_calls {
            if let Some(first_call) = tool_calls.first() {
                // Return the raw Tool Call JSON (Function Name + Arguments)
                let call_data = serde_json::to_value(first_call)?;
                return Ok(Some(call_data));
            }
        }

        // If no tool needed, return None
        Ok(None)
    }
}