use std::fmt::Debug;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Arc;

use regex::Regex;
use teloxide::prelude::{ChatId, Request, Requester, UserId};
use teloxide::types::Chat;
use teloxide::{Bot, RequestError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

use crate::Birthday;

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
                    "Failed to get the bot token from environment variable",
                )),
            }
        }
    }
}

/// Saves provided data to a JSON file.
///
/// # Parameters
/// - `data`: The data to save, which must be an Arc<RwLock<T>> where T is Serialize.
/// - `backup_file_path`: The path to the file where data will be backed up.
///
/// # Returns
/// - `Ok(())` on success.
/// - `Err(e)` on error with `e` being an `io::Error`.
pub async fn save_to_json<T>(
    data: Arc<RwLock<T>>,
    backup_file_path: &PathBuf,
) -> Result<(), std::io::Error>
where
    T: serde::Serialize + Sync + Send + Debug,
{
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

/// Loads data from a JSON file.
///
/// # Parameters
///
/// - `backup_file_path`: The path to the JSON file from which to load the data.
///
/// # Returns
///
/// - `Result` containing either the loaded data wrapped in `Arc<RwLock<T>>` on success,
///   or an error in case of failure.
pub async fn load_from_json<T>(backup_file_path: &PathBuf) -> Result<Arc<RwLock<T>>, std::io::Error>
where
    T: serde::de::DeserializeOwned + Send + Sync + Debug,
{
    let mut file = tokio::fs::File::open(backup_file_path).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;

    let data: T = serde_json::from_str(&contents)?;
    log::debug!("{:?}", data);
    Ok(Arc::new(RwLock::new(data)))
}

/// Parses the input string to create a `Birthday` struct.
/// The input string should be in the format "name, date, @username" or "name, date".
///
/// # Arguments
///
/// * `input` - The input string to parse.
///
/// # Returns
///
/// A `Birthday` struct if the input is valid, otherwise `None`.
pub fn parse_birthday_info(input: &str) -> Option<Birthday> {
    let re =
        Regex::new(r"^(?P<name>\w+\s\w+), (?P<date>\d{2}-\d{2})(, @?(?P<username>\w+))?$").unwrap();
    if let Some(caps) = re.captures(input) {
        let name = caps.name("name").unwrap().as_str().to_string();
        let date = caps.name("date").unwrap().as_str().to_string();
        let username = caps
            .name("username")
            .map(|u| u.as_str().to_string())
            .unwrap_or_default();
        Some(Birthday {
            name,
            date,
            username,
        })
    } else {
        None
    }
}
