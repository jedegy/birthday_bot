mod handles;
mod tasks;
mod utils;

use serde::Deserialize;
use teloxide::dispatching::{DpHandlerDescription, UpdateFilterExt};
use teloxide::{
    prelude::*,
    types::{Update, UserId},
    RequestError,
};
use tokio::sync::RwLock;

use std::collections::HashMap;
use std::sync::Arc;

/// The file path for the token file.
const TOKEN_FILE_PATH: &str = "token.txt";

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
#[tokio::main]
async fn main() {
    // Initialize logging
    pretty_env_logger::init();
    log::info!("Starting dispatching birthday reminder bot...");

    // Set configuration parameters
    let parameters = ConfigParameters {
        bot_maintainer: UserId(MAINTAINER_USER_ID),
    };

    // Create a new bot instance
    let bot = Bot::new(utils::get_token(TOKEN_FILE_PATH));
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

    // Build the handler for the bot
    let handler = build_handler(Arc::clone(&birthdays_map));

    // Create and dispatch the bot using the configured dispatcher
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
                        || (msg.chat.is_group()
                            && utils::is_admin(&bot, msg.chat.id, user.id)
                                .await
                                .unwrap_or_default())
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
