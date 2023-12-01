use std::collections::hash_map::Entry;
use teloxide::prelude::{Message, Requester, ResponseResult};
use teloxide::types::InputFile;
use teloxide::Bot;

use crate::utils::get_place;
use crate::{Birthdays, BirthdaysMap, State};

/// The message to send when the user sends a JSON file.
const JSON_MSG: &str =
    "Отправьте мне заполненный JSON файл с указанием дней рождений. Я отправил вам пример того, \
как должен выглядеть файл";

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
pub async fn handle_active_command(
    bot: Bot,
    msg: Message,
    birthday_map: BirthdaysMap,
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
        bot.send_document(msg.chat.id, InputFile::file(super::SAMPLE_JSON_FILE_PATH))
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
pub async fn handle_disable_command(
    bot: Bot,
    msg: Message,
    birthday_map: BirthdaysMap,
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
