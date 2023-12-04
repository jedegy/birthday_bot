use teloxide::prelude::{Message, Requester, ResponseResult};
use teloxide::types::InputFile;
use teloxide::Bot;

use crate::utils::get_place;
use crate::{Birthdays, ConfigParameters, GlobalState, State};

use std::collections::hash_map::Entry;

/// The message to send when the user sends a JSON file.
const JSON_MSG: &str =
    "Отправьте мне заполненный JSON файл с указанием дней рождений. Я отправил вам пример того, \
как должен выглядеть файл";

const BUSY_MSG: &str =
    "К сожалению, в данный момент я не могу принимать новые запросы из-за высокой нагрузки 😞 \
Попробуйте повторить запрос позже";

/// Handles admin commands for the bot.
///
/// # Arguments
///
/// * `bot` - The bot instance.
/// * `msg` - The message triggering the command.
/// * `cmd` - The specific command being processed.
/// * `cfg` - Configuration parameters for the bot.
///
/// # Returns
///
/// A `ResponseResult` indicating the success or failure of the command.
pub async fn admin_commands_handler(
    bot: Bot,
    msg: Message,
    cmd: super::AdminCommands,
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    match cmd {
        super::AdminCommands::Active => handle_active_command(bot, msg, cfg).await,
        super::AdminCommands::Disable => handle_disable_command(bot, msg, cfg).await,
        super::AdminCommands::List => handle_list_command(bot, msg, cfg).await,
    }
}

/// Handles the `list` command for the bot.
///
/// # Arguments
///
/// * `bot` - The bot instance.
/// * `msg` - The message triggering the command.
/// * `cfg` - Configuration parameters for the bot.
///
/// # Returns
///
/// A `ResponseResult` indicating the success or failure of the command.
async fn handle_list_command(bot: Bot, msg: Message, cfg: ConfigParameters) -> ResponseResult<()> {
    let b_map = cfg.b_map.read().await;
    let birthdays_default = Birthdays::default();
    let birthdays = b_map
        .get(&msg.chat.id)
        .map(|(_, birthdays)| birthdays)
        .unwrap_or(&birthdays_default);

    if birthdays.len() == 0 {
        bot.send_message(msg.chat.id, "Список дней рождений пуст")
            .await?;
    } else {
        let mut reply_text = String::from("Список дней рождений:\n");
        for (idx, birthday) in birthdays.get_birthdays().iter().enumerate() {
            reply_text += format!(
                "{}. {} - {} {}\n",
                idx, birthday.name, birthday.date, birthday.username
            )
            .as_str();
        }
        bot.send_message(msg.chat.id, reply_text).await?;
    }

    Ok(())
}

/// Handles the activation command for the bot.
///
/// # Arguments
///
/// * `bot` - The bot instance.
/// * `msg` - The message triggering the command.
/// * `cfg` - Configuration parameters for the bot.
///
/// # Returns
///
/// A `ResponseResult` indicating the success or failure of the command.
async fn handle_active_command(
    bot: Bot,
    msg: Message,
    mut cfg: ConfigParameters,
) -> ResponseResult<()> {
    log::info!("Active command received from chat id {}", msg.chat.id);

    let b_map_size = crate::utils::birthday_map_estimate_size(cfg.b_map.clone()).await;
    let place = get_place(&msg.chat);

    let (reply_msg, send_sample) = {
        let mut map = cfg.b_map.write().await;

        map.get_mut(&msg.chat.id)
            .map(|(state, birthdays)| match state {
                State::Active | State::Disabled if birthdays.get_birthdays().is_empty() => {
                    if b_map_size >= crate::birthday::BIRTHDAY_MAP_LIMIT {
                        cfg.state = GlobalState::Busy;
                        (BUSY_MSG.into(), false)
                    } else {
                        *state = State::WaitingJson;
                        (JSON_MSG.into(), false)
                    }
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
                if b_map_size >= crate::birthday::BIRTHDAY_MAP_LIMIT {
                    cfg.state = GlobalState::Busy;
                    (BUSY_MSG.into(), false)
                } else {
                    map.insert(msg.chat.id, (State::WaitingJson, Birthdays::default()));
                    (JSON_MSG.into(), true)
                }
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
/// * `cfg` - Configuration parameters for the bot.
///
/// # Returns
///
/// A `ResponseResult` indicating the success or failure of the command.
async fn handle_disable_command(
    bot: Bot,
    msg: Message,
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    log::info!("Disable command received from chat id {}", msg.chat.id);

    let place = get_place(&msg.chat);

    let reply_text = {
        let mut map = cfg.b_map.write().await;
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
