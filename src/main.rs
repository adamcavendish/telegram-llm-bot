//! Telegram Bot with OpenAI integration
//!
//! A Telegram bot that responds to @ mentions using the OpenAI chat completions API.
//! It supports configurable model selection and customizable greeting messages.

use std::env;
use std::sync::Arc;

use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, CreateChatCompletionRequestArgs,
    },
};
use teloxide::{
    dispatching::UpdateFilterExt, prelude::*, types::MessageEntityKind, utils::command::BotCommands,
};

/// Bot commands that users can invoke
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "Display this help message")]
    Help,
    #[command(description = "Start the bot")]
    Start,
}

/// Bot configuration structure
#[derive(Clone, Debug)]
struct BotConfig {
    /// The greeting message to display for /start and /help commands
    greeting_message: String,
    /// The OpenAI API client
    oai_client: Arc<Client<OpenAIConfig>>,
    /// The LLM model to use for completions
    llm_model_name: String,
}

impl BotConfig {
    /// Create a new bot configuration from environment variables
    fn from_env() -> Self {
        // Setup OpenAI endpoint.
        let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
        let openai_api_base = env::var("OPENAI_API_BASE").expect("OPENAI_API_BASE must be set");
        let openai_model_name =
            env::var("OPENAI_MODEL_NAME").expect("OPENAI_MODEL_NAME must be set");
        let openai_config = OpenAIConfig::new()
            .with_api_key(openai_api_key)
            .with_api_base(openai_api_base);

        // Get custom greeting message from environment variable or use default
        let greeting_message = env::var("BOT_GREETING_MESSAGE").unwrap_or_else(|_| {
            format!(
                "Hello! I'm an AI assistant bot using {}. Mention me (@bot_username) in a message to talk to me.", 
                openai_model_name
            )
        });

        // Initialize OpenAI client with config
        let openai_client = Arc::new(Client::with_config(openai_config));

        Self {
            greeting_message,
            oai_client: openai_client,
            llm_model_name: openai_model_name,
        }
    }

    /// Get a clone of the OpenAI client
    fn openai_client(&self) -> Arc<Client<OpenAIConfig>> {
        Arc::clone(&self.oai_client)
    }
}

#[tokio::main]
async fn main() {
    // Initialize the logger
    pretty_env_logger::init();

    // Get Telegram bot token from environment variable
    let bot = Bot::from_env();

    // Load bot configuration from environment
    let config = BotConfig::from_env();

    log::info!("LLM bot started with model: {}", config.llm_model_name);

    // Set up the message handler
    let handler = create_message_handler(&config);

    // Start the bot
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![config.openai_client()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

/// Handle commands like /start and /help
async fn command_handler(bot: Bot, msg: Message, greeting: String) -> ResponseResult<()> {
    match Command::parse(msg.text().unwrap_or_default(), "bot") {
        Ok(cmd) => match cmd {
            Command::Help | Command::Start => {
                bot.send_message(msg.chat.id, greeting).await?;
            }
        },
        Err(_) => {
            // Not a command or couldn't parse
        }
    }

    Ok(())
}

/// Extract the message text from a Telegram message
fn extract_message_text(msg: &Message) -> String {
    msg.text().unwrap_or("Hello").to_string()
}

/// Handle mentions to the bot
async fn handle_mention(
    bot: Bot,
    msg: Message,
    client: Arc<Client<OpenAIConfig>>,
    model_name: &str,
) -> ResponseResult<()> {
    // Extract the message text without the mention
    let message_text = extract_message_text(&msg);

    // Send a "typing" action to show the bot is processing
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

    // Send request to OpenAI and handle the response
    match send_openai_request(&client, model_name, &message_text).await {
        Ok(content) => {
            // Reply with the AI-generated response
            bot.send_message(msg.chat.id, content).await?;
        }
        Err(error) => {
            log::error!("OpenAI request error: {}", error);
            bot.send_message(
                msg.chat.id,
                "Sorry, I encountered an error while processing your request.",
            )
            .await?;
        }
    }

    Ok(())
}

/// Send a request to OpenAI and return the response content
async fn send_openai_request(
    client: &Client<OpenAIConfig>,
    model_name: &str,
    message_text: &str,
) -> Result<String, String> {
    // Create the request to OpenAI
    let request = CreateChatCompletionRequestArgs::default()
        .model(model_name)
        .messages([ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(message_text.to_string()),
                name: None,
            },
        )])
        .build()
        .map_err(|e| format!("Failed to build request: {}", e))?;

    // Send the request to OpenAI
    let response = client
        .chat()
        .create(request)
        .await
        .map_err(|e| format!("OpenAI API error: {:?}", e))?;

    // Extract the response content
    if let Some(choice) = response.choices.first() {
        if let Some(content) = &choice.message.content {
            Ok(content.clone())
        } else {
            Err("Received an empty response".to_string())
        }
    } else {
        Err("No response choices available".to_string())
    }
}

/// Create the message handler for the bot
fn create_message_handler(
    config: &BotConfig,
) -> Handler<'static, DependencyMap, ResponseResult<()>, teloxide::dispatching::DpHandlerDescription>
{
    // Clone the config to move into the closures
    let config = config.clone();

    Update::filter_message()
        .branch(dptree::entry().filter_command::<Command>().endpoint(
            move |bot: Bot, msg: Message| {
                let greeting = config.greeting_message.clone();
                async move { command_handler(bot, msg, greeting).await }
            },
        ))
        .branch(dptree::filter(is_mention_message).endpoint(
            move |bot: Bot, msg: Message, client: Arc<Client<OpenAIConfig>>| {
                let model = config.llm_model_name.clone();
                async move { handle_mention(bot, msg, client, &model).await }
            },
        ))
}

/// Check if a message contains a mention
fn is_mention_message(msg: &Message) -> bool {
    if let Some(entities) = msg.entities() {
        entities
            .iter()
            .any(|entity| matches!(entity.kind, MessageEntityKind::Mention))
    } else {
        false
    }
}
