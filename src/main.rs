use async_tempfile::TempFile;
use chrono::{Duration, Utc};
use serde::Deserialize;
use teloxide::dispatching::{DpHandlerDescription, UpdateFilterExt};
use teloxide::net::Download;
use teloxide::types::{Chat, InputFile};
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
/// The user ID of the bot maintainer.
const MAINTAINER_USER_ID: u64 = 437067064;

const GREETINGS_MSG: &str =
    "Привет! Этот бот создан для тех, кто постоянно забывает про дни рождения😁\n
С помощью него вы никогда не забудете поздравить своих друзей, коллег по работе или родственников.\n
Для более подробной информации по настройке используйте команду /help.";

const JSON_MSG: &str =
    "Отправьте мне заполненный JSON файл с указанием дней рождений. Я отправил вам пример того, \
как должен выглядеть файл";

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

/// Enum defining simple commands for the bot.
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "С помощью этих команд вы можете взаимодействовать и управлять мной.🤖\n\n\
    Основные команды доступны только для администраторов групп и каналов, а также если вы добавили меня в чат.\n\
    Проверить свой статус можно с помощью команды /checkcontrol"
)]
enum Command {
    /// Displays the hello message for the bot.
    #[command(description = "Отображает приветственное сообщение")]
    Start,
    /// Displays the description of the bot.
    #[command(description = "Отображает это сообщение")]
    Help,
    /// Displays the administrator of the bot.
    #[command(description = "Запускает проверку прав")]
    CheckControl,
    /// Sends a sample JSON file with birthdays.
    #[command(description = "Попросить меня отправить вам пример заполненного JSON файла")]
    File,
}

/// Enum defining admin commands for the bot.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum AdminCommands {
    #[command(description = "Включает уведомления о днях рождениях от меня")]
    Active,
    #[command(description = "Отключает уведомления о днях рождениях от меня")]
    Disable,
}

/// Function checks that user has admin rights
///
/// # Arguments
///
/// * `bot` - The bot instance
/// * `chat_id` - The chat id
/// * `user_id` - The user id
///
/// # Returns
///
/// A `Result` indicating the user has admin rights or not.
async fn is_admin(bot: &Bot, chat_id: ChatId, user_id: UserId) -> Result<bool, RequestError> {
    let admins = bot.get_chat_administrators(chat_id).send().await?;
    Ok(admins.iter().any(|admin| admin.user.id == user_id))
}

/// Function checks that user is maintainer
///
/// # Arguments
///
/// * `user_id` - The user id
///
/// # Returns
///
/// A `bool` indicating the user is maintainer or not.
fn is_maintainer(user_id: UserId) -> bool {
    user_id == UserId(MAINTAINER_USER_ID)
}

/// Function returns place where bot is used
///
/// # Arguments
///
/// * `chat` - The chat where bot is used
///
/// # Returns
///
/// A `String` indicating the place where bot is used.
fn get_place(chat: &Chat) -> String {
    if chat.is_group() || chat.is_supergroup() {
        "в этой группе".into()
    } else if chat.is_channel() {
        "в этом канале".into()
    } else {
        "в этом чате".into()
    }
}

/// Function formats admin commands description
///
/// # Arguments
///
/// * `desc` - The description of admin commands
/// * `place` - The place where bot is used
fn format_admin_commands_desc(desc: &str, place: &str) -> String {
    let mut result = String::new();
    for line in desc.split_terminator('\n') {
        result.push_str(&format!("{} {}\n", line, place));
    }
    result
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
    let bot = Bot::new(get_token());
    let bot_cloned = bot.clone();

    // Create a thread-safe map of chat IDs to bot states and birthdays
    let birthdays_map = Arc::new(RwLock::new(HashMap::<ChatId, (State, Birthdays)>::new()));
    let birthdays_map_cloned = Arc::clone(&birthdays_map);

    // Spawn a Tokio task for sending birthday reminders
    tokio::spawn(async move {
        loop {
            match send_birthday_reminders(bot_cloned.clone(), birthdays_map_cloned.clone()).await {
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
                .filter_command::<Command>()
                .endpoint(commands_handler),
        )
        // Branch for handling maintainer commands
        .branch(
            dptree::filter_async(|cfg: ConfigParameters, msg: Message, bot: Bot| async move {
                if let Some(user) = msg.from() {
                    user.id == cfg.bot_maintainer
                        || (msg.chat.is_group()
                            && is_admin(&bot, msg.chat.id, user.id)
                                .await
                                .unwrap_or_default())
                } else {
                    false
                }
            })
            .filter_command::<AdminCommands>()
            .endpoint(move |msg: Message, bot: Bot, cmd: AdminCommands| {
                let b_map_for_active = b_map.clone();
                let b_map_for_disable = b_map.clone();

                async move {
                    match cmd {
                        AdminCommands::Active => {
                            handle_active_command(bot, msg, b_map_for_active).await
                        }
                        AdminCommands::Disable => {
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

    let place = get_place(&msg.chat);

    let (reply_msg, send_sample) = {
        let mut map = birthday_map.write().await;

        map.get_mut(&msg.chat.id)
            .map(|(state, birthdays)| match state {
                State::Active | State::Disabled if birthdays.birthdays.is_empty() => {
                    *state = State::WaitingJson;
                    (JSON_MSG.into(), false)
                }
                State::Disabled => {
                    *state = State::Active;
                    (
                        format!("Уведомления от меня снова активны {}", place),
                        false,
                    )
                }
                State::Active => (format!("Уведомления от меня уже активны {}", place), false),
                State::WaitingJson => (JSON_MSG.into(), false),
            })
            .unwrap_or({
                map.insert(msg.chat.id, (State::WaitingJson, Birthdays::default()));
                (JSON_MSG.into(), true)
            })
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

    let place = get_place(&msg.chat);

    let reply_text = {
        let mut map = birthday_map.write().await;
        match map.entry(msg.chat.id) {
            Entry::Occupied(mut entry) => {
                let (state, _) = entry.get_mut();
                match *state {
                    State::Disabled => format!("Уведомления от меня уже отключены {}", place),
                    _ => {
                        *state = State::Disabled;
                        format!("Уведомления от меня отключены {}. Для повторной активации выполните команду /active", place)
                    }
                }
            }
            Entry::Vacant(entry) => {
                entry.insert((State::Disabled, Birthdays::default()));
                format!("Уведомления от меня отключены {}. Для повторной активации выполните команду /active", place)
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
                    bot.send_message(
                        msg.chat.id,
                        "Дни рождения успешно загружены🎉 \
                    Напоминания будут приходить ровно в день рождение в 7:00 UTC",
                    )
                    .await?;
                }
                Err(e) => {
                    log::error!("Failed to parse the file content: {}", e);
                    b_map.insert(msg.chat.id, (State::WaitingJson, Birthdays::default()));
                    bot.send_message(
                        msg.chat.id,
                        "К сожалению, отправленный файл не корректный или содержит ошибки😔 \
                    Проверьте его и отправьте ещё раз",
                    )
                    .await?;
                }
            }
        }
    }

    Ok(())
}

/// Retrieves the bot token from an environment variable or a file.
///
/// The token is read from the `BIRTHDAY_REMINDER_BOT_TOKEN` environment variable.
/// If the environment variable is not set, the token is read from a predefined file path (`TOKEN_FILE_PATH`).
///
/// # Returns
///
/// The bot token as a `String`.
fn get_token() -> String {
    // Try to get the token from the environment variable.
    match std::env::var("BIRTHDAY_REMINDER_BOT_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            // If the environment variable is not set, read the token from the file.
            let token_file = std::fs::File::open(TOKEN_FILE_PATH).unwrap();
            let mut token = String::new();

            // Read the token from the file.
            std::io::BufReader::new(token_file)
                .read_line(&mut token)
                .unwrap();

            // Return the trimmed token.
            token.trim().to_string()
        }
    }
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
    bot: Bot,
    me: teloxide::types::Me,
    msg: Message,
    cmd: Command,
) -> Result<(), RequestError> {
    // Determine the user ID of the message sender.
    let user_id = msg.from().unwrap().id;

    // Determine the place where the bot is used.
    let place = get_place(&msg.chat);

    // Determine the response based on the command.
    let text = match cmd {
        Command::Start => GREETINGS_MSG.to_string(),
        Command::Help => {
            if msg.chat.is_group() || msg.chat.is_supergroup() || msg.chat.is_channel() {
                if is_maintainer(user_id)
                    || is_admin(&bot, msg.chat.id, user_id)
                        .await
                        .unwrap_or_default()
                {
                    format!(
                        "{}\n{}",
                        Command::descriptions().username_from_me(&me).to_string(),
                        format_admin_commands_desc(
                            &AdminCommands::descriptions()
                                .username_from_me(&me)
                                .to_string(),
                            &place
                        )
                    )
                } else {
                    Command::descriptions().username_from_me(&me).to_string()
                }
            } else {
                format!(
                    "{}\n{}",
                    Command::descriptions().to_string(),
                    format_admin_commands_desc(&AdminCommands::descriptions().to_string(), &place)
                )
            }
        }
        Command::CheckControl => {
            if is_maintainer(user_id) {
                "Вы мой создатель!🙏".into()
            } else if is_admin(&bot, msg.chat.id, user_id)
                .await
                .unwrap_or_default()
            {
                format!("Вы можете взаимодействовать со мной {}!😄", place)
            } else {
                format!(
                    "К сожалению, вы не можете взаимодействовать со мной {}😞",
                    place
                )
            }
        }
        Command::File => {
            bot.send_document(msg.chat.id, InputFile::file(SAMPLE_JSON_FILE_PATH))
                .await?;
            return Ok(());
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
/// with an active bot state. The reminders are sent at 7:00 AM UTC daily.
async fn send_birthday_reminders(
    bot: Bot,
    birthdays_map: Arc<RwLock<HashMap<ChatId, (State, Birthdays)>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Calculate the time for the next reminder.
        let now = Utc::now().naive_utc();
        let next_run = (now + Duration::days(1))
            .date()
            .and_hms_opt(7, 0, 0)
            .unwrap();
        let duration_until_next_run = (next_run - now).to_std().unwrap_or_default();

        // Sleep until the next reminder time.
        tokio::time::sleep(duration_until_next_run).await;

        let mut output = Vec::new();
        {
            let b_map = birthdays_map.read().await;

            for (chat_id, (state, vec)) in b_map.iter() {
                if State::Active == *state {
                    for birthday in vec.birthdays.iter() {
                        if birthday.date == Utc::now().format("%d-%m").to_string() {
                            let username_text = if !birthday.username.is_empty() {
                                format!("({})", birthday.username)
                            } else {
                                "".into()
                            };

                            let text = format!(
                                "Поздравьте сегодня замечательного человека с днем рождения {} {}!🎉",
                                birthday.name, username_text
                            );
                            output.push((*chat_id, text));
                        }
                    }
                }
            }
        }

        // Send the reminders.
        for (chat_id, text) in output {
            bot.send_message(chat_id, text).await?;
        }
    }
}
