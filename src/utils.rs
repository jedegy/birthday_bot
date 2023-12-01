use teloxide::prelude::{ChatId, Request, Requester, UserId};
use teloxide::types::Chat;
use teloxide::{Bot, RequestError};

use std::io::BufRead;
use std::path::PathBuf;

/// Function formats admin commands description
///
/// # Arguments
///
/// * `desc` - The description of admin commands
/// * `place` - The place where bot is used
pub fn format_admin_commands_desc(desc: &str, place: &str) -> String {
    let mut result = String::new();
    for line in desc.split_terminator('\n') {
        result.push_str(&format!("{} {}\n", line, place));
    }
    result
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
/// A `String` indicating the place where bot is used.
pub fn get_place(chat: &Chat) -> String {
    if chat.is_group() || chat.is_supergroup() {
        "в этой группе".into()
    } else if chat.is_channel() {
        "в этом канале".into()
    } else {
        "в этом чате".into()
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
