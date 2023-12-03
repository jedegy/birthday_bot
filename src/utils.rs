use teloxide::prelude::{ChatId, Request, Requester, UserId};
use teloxide::types::Chat;
use teloxide::{Bot, RequestError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Arc;

/// Represents places where bot is used
pub enum Place {
    Group,
    Channel,
    Chat,
}

impl std::fmt::Display for Place {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Place::Group => write!(f, "в группе"),
            Place::Channel => write!(f, "в канале"),
            Place::Chat => write!(f, "в чате"),
        }
    }
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
pub async fn is_admin(bot: &Bot, chat_id: ChatId, user_id: UserId) -> Result<bool, RequestError> {
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
pub fn is_maintainer(user_id: UserId) -> bool {
    user_id == UserId(super::MAINTAINER_USER_ID)
}

/// Function returns place where bot is used
///
/// # Arguments
///
/// * `chat` - The chat where bot is used
///
/// # Returns
///
/// A `Place` where bot is used.
pub fn get_place(chat: &Chat) -> Place {
    if chat.is_group() || chat.is_supergroup() {
        Place::Group
    } else if chat.is_channel() {
        Place::Channel
    } else {
        Place::Chat
    }
}

/// Retrieves the bot token from an environment variable or a file.
///
/// The token is read from the `BIRTHDAY_REMINDER_BOT_TOKEN` environment variable.
/// If the environment variable is not set, the token is read from a predefined file path.
///
/// # Arguments
///
/// * `path` - The path to the file containing the bot token.
///
/// # Returns
///
/// The bot token as a `String`.
pub fn get_token(path: Option<PathBuf>) -> std::io::Result<String> {
    match path {
        Some(path) => {
            let token_file = std::fs::File::open(path)?;
            let mut token = String::new();

            // Read the token from the file.
            std::io::BufReader::new(token_file).read_line(&mut token)?;

            log::info!("Using token retrieved from file");

            // Return the trimmed token.
            Ok(token.trim().to_string())
        }
        None => {
            log::warn!(
                "No token file path provided, trying to get the token from environment variable"
            );
            match std::env::var(super::BOT_TOKEN_ENV_VAR) {
                Ok(token) => {
                    log::info!("Using token retrieved from environment variable");
                    Ok(token)
                }
                Err(_) => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to get the bot token from environment variable"),
                )),
            }
        }
    }
}

/// Saves the data to a JSON file.
///
/// # Arguments
///
/// * `data` - The data to save.
/// * `backup_file_path` - The path to the JSON file.
///
/// # Returns
///
/// A `Result` indicating the data was saved or not.
pub async fn save_to_json(
    data: super::BirthdaysMap,
    backup_file_path: &PathBuf,
) -> Result<(), std::io::Error> {
    let data_read = data.read().await;
    log::debug!("{:?}", data_read);
    let json = serde_json::to_string(&*data_read)?;

    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(backup_file_path)
        .await?;

    file.write_all(json.as_bytes()).await?;
    Ok(())
}

/// Loads the data from a JSON file.
///
/// # Arguments
///
/// * `backup_file_path` - The path to the JSON file.
///
/// # Returns
///
/// A `Result` indicating the data was loaded or not.
pub async fn load_from_json(
    backup_file_path: &PathBuf,
) -> Result<super::BirthdaysMap, std::io::Error> {
    let mut file = tokio::fs::File::open(backup_file_path).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;

    let data = Arc::new(RwLock::new(serde_json::from_str(&contents)?));
    log::debug!("{:?}", data);
    Ok(data)
}

/// Returns the size in bytes of the birthday map.
///
/// # Arguments
///
/// * `birthday_map` - A thread-safe map of chat IDs to bot states and birthdays.
///
/// # Returns
///
/// The size in bytes of the birthday map.
pub async fn birthday_map_estimate_size(birthday_map: super::BirthdaysMap) -> usize {
    let map = birthday_map.read().await;
    let mut size = std::mem::size_of_val(&map);

    for (chat_id, (state, birthdays)) in map.iter() {
        size += std::mem::size_of_val(chat_id)
            + std::mem::size_of_val(state)
            + std::mem::size_of_val(birthdays);
    }

    size
}
