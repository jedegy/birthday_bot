use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use teloxide::{
    dispatching::{DpHandlerDescription, HandlerExt, UpdateFilterExt},
    dptree,
    prelude::{DependencyMap, Dispatcher, Handler, LoggingErrorHandler, Message},
    types::{Update, UserId},
    Bot, RequestError,
};
use tokio::sync::RwLock;

mod args;
mod birthday;
mod handles;
mod state;
mod tasks;
mod utils;

pub use birthday::{Birthday, Birthdays, BirthdaysMap, BirthdaysMapThreadSafe};
pub use state::State;

/// The user ID of the bot maintainer.
const MAINTAINER_USER_ID: u64 = 437067064;

/// The name of the environment variable for the bot token.
const BOT_TOKEN_ENV_VAR: &str = "BIRTHDAY_REMINDER_BOT_TOKEN";

/// Represents the configuration parameters for the bot.
#[derive(Clone)]
struct ConfigParameters {
    /// The user ID of the bot maintainer.
    bot_maintainer: UserId,
    /// The task manager for the bot.
    task_manager: Arc<tasks::Manager>,
    /// The thread-safe map of chat IDs to bot states and birthdays.
    b_map: BirthdaysMapThreadSafe,
    /// The path to the backup file.
    backup_path: std::path::PathBuf,
}

/// The main function for the bot, using Tokio.
#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    // Parse command-line arguments
    let args = args::Args::parse();

    // Initialize logging
    pretty_env_logger::init();

    // Get the bot token
    let token = match utils::get_token(args.token_path) {
        Ok(token) => token,
        Err(e) => {
            log::error!("Failed to get the bot token: {}", e);
            return Err(e);
        }
    };

    // Load data from backup file if it exists
    let birthdays_map = if Path::new(&args.backup_path).exists() {
        log::info!("Loading data from backup file {:?}...", args.backup_path);
        utils::load_from_json(&args.backup_path)
            .await
            .map_err(|e| {
                log::error!("Error during loading backup file: {}", e);
                e
            })
            .unwrap_or_else(|_| Arc::new(RwLock::new(BirthdaysMap::default())))
    } else {
        // Create a thread-safe map of chat IDs to bot states and birthdays
        Arc::new(RwLock::new(BirthdaysMap::default()))
    };
    let birthdays_map_cloned = Arc::clone(&birthdays_map);
    let birthdays_map_cloned_for_backup = Arc::clone(&birthdays_map);

    // Create a new bot instance
    let bot = Bot::new(token);
    let bot_for_br = bot.clone();
    let bot_for_hc = bot.clone();

    // Create a task manager
    let task_manager = tasks::Manager::new(
        tokio::spawn(async move {
            loop {
                match tasks::send_birthday_reminders(
                    bot_for_br.clone(),
                    birthdays_map_cloned.clone(),
                )
                .await
                {
                    Ok(_) => (),
                    Err(e) => log::error!("Error during sending birthday reminders: {}", e),
                }
            }
        }), // Birthday reminder
        tokio::spawn(tasks::health_check_task(bot_for_hc)), // Health check
        tokio::spawn(tasks::daily_backup_task(
            birthdays_map_cloned_for_backup.clone(),
            args.backup_path.clone(),
        )), // Daily backup
    );

    // Set configuration parameters
    let parameters = ConfigParameters {
        bot_maintainer: UserId(args.maintainer_user_id.unwrap_or(MAINTAINER_USER_ID)),
        task_manager: Arc::from(task_manager),
        b_map: birthdays_map,
        backup_path: args.backup_path,
    };

    log::info!("Bot maintainer user ID: {}", parameters.bot_maintainer);

    // Create and dispatch the bot using the configured dispatcher
    log::info!("Starting dispatching birthday reminder bot...");
    Dispatcher::builder(bot, build_handler())
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
fn build_handler() -> Handler<'static, DependencyMap, Result<(), RequestError>, DpHandlerDescription>
{
    // Create the update filter for messages
    Update::filter_message()
        // Branch for handling simple commands
        .branch(
            dptree::entry()
                .filter_command::<handles::Command>()
                .endpoint(handles::base_commands_handler),
        )
        .branch(
            dptree::filter_async(|msg: Message, cfg: ConfigParameters| async move {
                msg.from()
                    .map_or(false, |user| user.id == cfg.bot_maintainer)
            })
            .filter_command::<handles::MaintainerCommands>()
            .endpoint(handles::maintainer_commands_handler),
        )
        // Branch for handling admin commands
        .branch(
            dptree::filter_async(|bot: Bot, msg: Message, cfg: ConfigParameters| async move {
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
            .endpoint(handles::admin_commands_handler),
        )
        // Branch for handling common commands
        .branch(
            dptree::filter_async(|bot: Bot, msg: Message, cfg: ConfigParameters| async move {
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
            .endpoint(handles::common_commands_handler),
        )
}
