use async_tempfile::TempFile;
use chrono::{Duration, Utc};
use serde::Deserialize;
use teloxide::dispatching::{DpHandlerDescription, UpdateFilterExt};
use teloxide::net::Download;
use teloxide::types::InputFile;
use teloxide::{
    prelude::*,
    types::{Update, UserId},
    utils::command::BotCommands,
    RequestError,
};

use tokio::sync::RwLock;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io::BufRead;
use std::sync::Arc;

/// The file path for the sample JSON birthdays file.
const SAMPLE_JSON_FILE_PATH: &str = "sample.json";
/// The file path for the token file.
const TOKEN_FILE_PATH: &str = "token.txt";
/// The username of the bot maintainer.
const MAINTAINER_USERNAME: &str = "dsemak";
/// The user ID of the bot maintainer.
const MAINTAINER_USER_ID: u64 = 437067064;

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
#[derive(Clone, PartialEq)]
enum State {
    Active,
    Disabled,
    WaitingJson,
}

/// Represents the configuration parameters for the bot.
#[derive(Clone)]
struct ConfigParameters {
    bot_maintainer: UserId,
    maintainer_username: Option<String>,
}

/// Enum defining simple commands for the bot.
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "Привет! Это бот для отправки уведомлений о днях рождениях. Команды бота:"
)]
enum Command {
    /// Displays the description of the bot.
    #[command(description = "Отображает описание бота.")]
    Help,
    /// Displays the administrator of the bot.
    #[command(description = "Отображает администратора данного бота.")]
    Maintainer,
}

/// Enum defining maintainer commands for the bot.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum MaintainerCommands {
    #[command(description = "Включает бота в текущем чате.")]
    Active,
    #[command(description = "Отключает бота в текущем чате.")]
    Disable,
}

/// The main function for the bot, using Tokio.
#[tokio::main]
async fn main() {
    // Initialize logging
    pretty_env_logger::init();
    log::info!("Starting dispatching birthday reminder bot...");

    // Create a new bot instance
    let bot = Bot::new(get_token());
    let birthdays_map = Arc::new(RwLock::new(HashMap::<ChatId, (State, Birthdays)>::new()));

    // Spawn a Tokio task for sending birthday reminders
    tokio::spawn(send_birthday_reminders(
        bot.clone(),
        Arc::clone(&birthdays_map),
    ));

    // Set configuration parameters
    let parameters = ConfigParameters {
        bot_maintainer: UserId(MAINTAINER_USER_ID),
        maintainer_username: Some(String::from(MAINTAINER_USERNAME)),
    };

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
                .filter_command::<Command>()
                .endpoint(commands_handler),
        )
        // Branch for handling maintainer commands
        .branch(
            dptree::filter(|cfg: ConfigParameters, msg: Message| {
                msg.from()
                    .map(|user| user.id == cfg.bot_maintainer)
                    .unwrap_or_default()
            })
            .filter_command::<MaintainerCommands>()
            .endpoint(move |msg: Message, bot: Bot, cmd: MaintainerCommands| {
                let b_map_for_active = b_map.clone();
                let b_map_for_disable = b_map.clone();

                async move {
                    match cmd {
                        MaintainerCommands::Active => {
                            handle_active_command(bot, msg, b_map_for_active).await
                        }
                        MaintainerCommands::Disable => {
                            handle_disable_command(bot, msg, b_map_for_disable).await
                        }
                    }
                }
            }),
        )
        // Branch for handling document messages
        .branch(dptree::endpoint(move |msg: Message, bot: Bot| {
            let b_map = Arc::clone(&birthdays_map);
            async move { handle_document(bot, msg, b_map).await }
        }))
}

/// Handles the activation command for the bot.
///
/// # Arguments
///
/// * `bot` - The bot instance.
/// * `msg` - The message triggering the command.
/// * `birthday_map` - A thread-safe map of chat IDs to bot states and birthdays.
///
/// # Returns
///
/// A `ResponseResult` indicating the success or failure of the command.
async fn handle_active_command(
    bot: Bot,
    msg: Message,
    birthday_map: Arc<RwLock<HashMap<ChatId, (State, Birthdays)>>>,
) -> ResponseResult<()> {
    log::info!("Active command received from chat id {}", msg.chat.id);

    let (reply_msg, send_sample) = {
        let mut map = birthday_map.write().await;

        match map.entry(msg.chat.id) {
            Entry::Occupied(mut entry) => {
                let entry = entry.get_mut();

                match (*entry).0 {
                    State::Active | State::Disabled if (*entry).1.birthdays.is_empty() => {
                        ("Отправьте json файл с указанием дней рождений", true)
                    }
                    State::Disabled => {
                        (*entry).0 = State::Active;
                        ("Напоминания от бота снова активны.", false)
                    }
                    State::Active => ("Напоминания от бота уже активны в данном чате.", false),
                    State::WaitingJson => ("Отправьте json файл с указанием дней рождений", true),
                }
            }
            Entry::Vacant(entry) => {
                entry.insert((State::WaitingJson, Birthdays::default()));
                ("Отправьте json файл с указанием дней рождений", true)
            }
        }
    };
    bot.send_message(msg.chat.id, reply_msg).await?;

    if send_sample {
        bot.send_document(msg.chat.id, InputFile::file(SAMPLE_JSON_FILE_PATH))
            .await?;
    }

    Ok(())
}

/// Handles the disable command for the bot.
///
/// # Arguments
///
/// * `bot` - The bot instance.
/// * `msg` - The message triggering the command.
/// * `birthday_map` - A thread-safe map of chat IDs to bot states and birthdays.
///
/// # Returns
///
/// A `ResponseResult` indicating the success or failure of the command.
async fn handle_disable_command(
    bot: Bot,
    msg: Message,
    birthday_map: Arc<RwLock<HashMap<ChatId, (State, Birthdays)>>>,
) -> ResponseResult<()> {
    log::info!("Disable command received from chat id {}", msg.chat.id);

    let reply_text = {
        let mut map = birthday_map.write().await;

        match map.entry(msg.chat.id) {
            Entry::Occupied(mut entry) => {
                let (state, _) = entry.get_mut();
                match *state {
                    State::Disabled => "Напоминания уже отключены для данного чата".to_string(),
                    _ => {
                        *state = State::Disabled;
                        "Напоминания отключены для данного чата".to_string()
                    }
                }
            }
            Entry::Vacant(entry) => {
                entry.insert((State::Disabled, Birthdays::default()));
                "Напоминания отключены для данного чата".to_string()
            }
        }
    };

    bot.send_message(msg.chat.id, &reply_text).await?;
    Ok(())
}

/// Handles document messages for the bot.
///
/// # Arguments
///
/// * `bot` - The bot instance.
/// * `msg` - The message containing the document.
/// * `b_map` - A thread-safe map of chat IDs to bot states and birthdays.
///
/// # Returns
///
/// A `ResponseResult` indicating the success or failure of the command.
async fn handle_document(
    bot: Bot,
    msg: Message,
    b_map: Arc<RwLock<HashMap<ChatId, (State, Birthdays)>>>,
) -> ResponseResult<()> {
    if let Some(doc) = msg.document() {
        log::info!("Document received from chat id {}", msg.chat.id);

        let mut b_map = b_map.write().await;

        let download_file = b_map
            .get(&msg.chat.id)
            .map_or(false, |entry| entry.0 == State::WaitingJson);

        if download_file {
            let file = doc.file.clone();
            log::info!("Downloading file {} from chat id {}", file.id, msg.chat.id);

            let file_info = bot.get_file(file.id).send().await?;
            let mut temp_file = TempFile::new().await.unwrap();
            bot.download_file(&file_info.path, &mut temp_file).await?;

            let file_content: String = tokio::fs::read_to_string(temp_file.file_path()).await?;

            match serde_json::from_str(&file_content) {
                Ok(birthdays) => {
                    b_map.insert(msg.chat.id, (State::Active, birthdays));
                    bot.send_message(msg.chat.id, "Дни рождения успешно загружены")
                        .await?;
                }
                Err(e) => {
                    log::error!("Failed to parse the file content: {}", e);
                    b_map.insert(msg.chat.id, (State::Disabled, Birthdays::default()));
                    bot.send_message(msg.chat.id, "Отправленный файл не корректный")
                        .await?;
                }
            }
        }
    }

    Ok(())
}

/// Retrieves the bot token from a file.
///
/// The token is read from a predefined file path (`TOKEN_FILE_PATH`).
///
/// # Returns
///
/// The bot token as a `String`.
fn get_token() -> String {
    // Open the token file.
    let token_file = std::fs::File::open(TOKEN_FILE_PATH).unwrap();
    let mut token = String::new();

    // Read the token from the file.
    std::io::BufReader::new(token_file)
        .read_line(&mut token)
        .unwrap();

    // Return the trimmed token.
    token.trim().to_string()
}

/// Handles commands for the bot.
///
/// # Arguments
///
/// * `cfg` - Configuration parameters for the bot.
/// * `bot` - The bot instance.
/// * `me` - Information about the bot itself.
/// * `msg` - The message triggering the command.
/// * `cmd` - The specific command being processed.
///
/// # Returns
///
/// A `Result` indicating the success or failure of the command handling.
async fn commands_handler(
    cfg: ConfigParameters,
    bot: Bot,
    me: teloxide::types::Me,
    msg: Message,
    cmd: Command,
) -> Result<(), RequestError> {
    // Determine the response based on the command.
    let text = match cmd {
        Command::Help => {
            if msg.from().unwrap().id == cfg.bot_maintainer {
                format!(
                    "{}\n{}",
                    Command::descriptions(),
                    MaintainerCommands::descriptions()
                )
            } else if msg.chat.is_group() || msg.chat.is_supergroup() {
                Command::descriptions().username_from_me(&me).to_string()
            } else {
                Command::descriptions().to_string()
            }
        }
        Command::Maintainer => {
            if msg.from().unwrap().id == cfg.bot_maintainer {
                "Администратор вы!".into()
            } else if let Some(username) = cfg.maintainer_username {
                format!("Администратор @{username}")
            } else {
                format!("Администратор ID {}", cfg.bot_maintainer)
            }
        }
    };

    // Send the response back.
    bot.send_message(msg.chat.id, text).await?;

    Ok(())
}

/// Sends birthday reminders on a daily basis.
///
/// # Arguments
///
/// * `bot` - The bot instance.
/// * `birthdays_map` - A thread-safe map of chat IDs to bot states and birthdays.
///
/// This function sends reminders about upcoming birthdays to chats
/// with an active bot state. The reminders are sent at 9:00 AM UTC daily.
async fn send_birthday_reminders(
    bot: Bot,
    birthdays_map: Arc<RwLock<HashMap<ChatId, (State, Birthdays)>>>,
) {
    loop {
        // Calculate the time for the next reminder.
        let now = Utc::now().naive_utc();
        let next_run = (now + Duration::days(1))
            .date()
            .and_hms_opt(9, 0, 0)
            .unwrap();
        let duration_until_next_run = (next_run - now).to_std().unwrap_or_default();

        // Sleep until the next reminder time.
        tokio::time::sleep(duration_until_next_run).await;

        // Determine birthdays for the next day.
        let tomorrow = (Utc::now() + Duration::days(1)).format("%d-%m").to_string();
        let mut output = Vec::new();
        {
            let b_map = birthdays_map.read().await;

            for (chat_id, (state, vec)) in b_map.iter() {
                if State::Active == *state {
                    for birthday in vec.birthdays.iter() {
                        if birthday.date == tomorrow {
                            let text = format!(
                                "Завтра день рождения у замечательного человека {} ({})! ",
                                birthday.name, birthday.username
                            );
                            output.push((*chat_id, text));
                        }
                    }
                }
            }
        }

        // Send the reminders.
        for (chat_id, text) in output {
            bot.send_message(chat_id, text).await.unwrap();
        }
    }
}
