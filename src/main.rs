mod args;
mod handles;
mod tasks;
mod utils;

use clap::Parser;
use serde::Deserialize;
use teloxide::{
    dispatching::{DpHandlerDescription, HandlerExt, UpdateFilterExt},
    dptree,
    prelude::{ChatId, DependencyMap, Dispatcher, Handler, LoggingErrorHandler, Message},
    types::{Update, UserId},
    Bot, RequestError,
};
use tokio::sync::RwLock;

use std::collections::HashMap;
use std::sync::Arc;

/// The user ID of the bot maintainer.
const MAINTAINER_USER_ID: u64 = 437067064;

const BOT_TOKEN_ENV_VAR: &str = "BIRTHDAY_REMINDER_BOT_TOKEN";

type BirthdaysMap = Arc<RwLock<HashMap<ChatId, (State, Birthdays)>>>;

/// Represents a birthday with a name, date, and username.
#[derive(Clone, Debug, Default, Deserialize)]
struct Birthday {
    name: String,
    date: String,
    username: String,
}

/// Represents a list of birthdays.
#[derive(Clone, Debug, Default, Deserialize)]
struct Birthdays {
    birthdays: Vec<Birthday>,
}

/// Represents the state of the bot.
#[derive(Clone, PartialEq, Debug)]
enum State {
    Active,
    Disabled,
    WaitingJson,
}

/// Represents the configuration parameters for the bot.
#[derive(Clone)]
struct ConfigParameters {
    bot_maintainer: UserId,
}

/// The main function for the bot, using Tokio.
#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    // Parse command-line arguments
    let args = args::Args::parse();

    // Initialize logging
    pretty_env_logger::init();

    // Set configuration parameters
    let parameters = ConfigParameters {
        bot_maintainer: UserId(args.maintainer_user_id.unwrap_or(MAINTAINER_USER_ID)),
    };

    log::info!("Bot maintainer user ID: {}", parameters.bot_maintainer);

    // Get the bot token
    let token = match utils::get_token(args.token_path) {
        Ok(token) => token,
        Err(e) => {
            log::error!("Failed to get the bot token: {}", e);
            return Err(e);
        }
    };

    // Create a new bot instance
    let bot = Bot::new(token);
    let bot_cloned = bot.clone();

    // Create a thread-safe map of chat IDs to bot states and birthdays
    let birthdays_map = Arc::new(RwLock::new(HashMap::<ChatId, (State, Birthdays)>::new()));
    let birthdays_map_cloned = Arc::clone(&birthdays_map);

    // Spawn a Tokio task for sending birthday reminders
    tokio::spawn(async move {
        loop {
            match tasks::send_birthday_reminders(bot_cloned.clone(), birthdays_map_cloned.clone())
                .await
            {
                Ok(_) => break,
                Err(e) => log::error!("Error sending birthday reminders: {}", e),
            }
        }
    });

    log::info!("Birthday reminder task successfully spawned");

    // Build the handler for the bot
    let handler = build_handler(Arc::clone(&birthdays_map));

    // Create and dispatch the bot using the configured dispatcher
    log::info!("Starting dispatching birthday reminder bot...");
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![parameters])
        .default_handler(|upd| async move {
            log::info!("Unhandled update: {:?}", upd);
        })
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

/// Builds the handler for processing bot updates.
fn build_handler(
    birthdays_map: Arc<RwLock<HashMap<ChatId, (State, Birthdays)>>>,
) -> Handler<'static, DependencyMap, Result<(), RequestError>, DpHandlerDescription> {
    // Clone the birthdays map for each branch
    let b_map = Arc::clone(&birthdays_map);

    // Create the update filter for messages
    Update::filter_message()
        // Branch for handling simple commands
        .branch(
            dptree::entry()
                .filter_command::<handles::Command>()
                .endpoint(handles::commands_handler),
        )
        // Branch for handling maintainer commands
        .branch(
            dptree::filter_async(|cfg: ConfigParameters, msg: Message, bot: Bot| async move {
                if let Some(user) = msg.from() {
                    user.id == cfg.bot_maintainer
                        || ((msg.chat.is_group()
                            || msg.chat.is_supergroup()
                            || msg.chat.is_channel())
                            && utils::is_admin(&bot, msg.chat.id, user.id)
                                .await
                                .unwrap_or_default())
                        || msg.chat.is_chat()
                } else {
                    false
                }
            })
            .filter_command::<handles::AdminCommands>()
            .endpoint(move |msg: Message, bot: Bot, cmd: handles::AdminCommands| {
                let b_map_for_active = b_map.clone();
                let b_map_for_disable = b_map.clone();

                async move {
                    match cmd {
                        handles::AdminCommands::Active => {
                            handles::handle_active_command(bot, msg, b_map_for_active).await
                        }
                        handles::AdminCommands::Disable => {
                            handles::handle_disable_command(bot, msg, b_map_for_disable).await
                        }
                    }
                }
            }),
        )
        // Branch for handling document messages
        .branch(dptree::endpoint(move |msg: Message, bot: Bot| {
            let b_map = Arc::clone(&birthdays_map);
            async move { handles::handle_document(bot, msg, b_map).await }
        }))
}
