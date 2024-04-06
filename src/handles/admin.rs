use teloxide::prelude::{Message, Requester, ResponseResult};
use teloxide::types::InputFile;
use teloxide::Bot;

use crate::handles::BUSY_MSG;
use crate::{Birthdays, ConfigParameters, State};

/// The message to send when the user sends a JSON file.
const JSON_MSG: &str =
    "Отправьте мне заполненный JSON файл с указанием дней рождений. Я отправил вам пример того, \
как должен выглядеть файл";

/// The message to send when the user sends a birthday to add.
const ADD_MSG: &str = "Отправьте мне день рождения в формате 'Имя Фамилия, ДД-ММ, @username' или 'Имя Фамилия, ДД-MM'. \
    Например, 'Иван Иванов, 01-01, @ivan' или 'Иван Иванов, 01-01'.\n \
    Для выхода из режима обновления дней рождений введите команду /cancel";

/// The message to send when the user wants to remove a birthday.
const REMOVE_MSG: &str = "Отправьте мне номер дня рождения, который хотите удалить. \n \
    Для выхода из режима обновления дней рождений введите команду /cancel";

/// The message to send when the user cancels the birthday addition mode.
const CANCEL_MSG: &str =
    "Режим обнолвения дней рождений отключен. Для активации уведомлений выполните команду /active";

/// The message to send when the user tries to cancel the birthday addition mode without adding any birthdays.
const CANCEL_EMPTY_LIST_MSG: &str = "Режим обновления дней рождений отключен. \
    Ни одного дня рождения не добавлено.";

/// The message to send when the user tries to cancel the birthday addition mode when it is already disabled.
const CANCEL_ALREADY_DISABLED_MSG: &str = "Режим обновления дней рождений уже отключен.";

/// The message to send when the user tries to activate the bot.
const ACTIVE_MSG: &str = "Уведомления от меня активны!🎉";

/// The message to send when the user tries to activate the bot when it is already active.
const ACTIVE_ALREADY_ACTIVE_MSG: &str = "Уведомления от меня уже активны!";

/// The message to send when the user tries to activate the bot without adding any birthdays via JSON.
const ACTIVE_WAITING_JSON_MSG: &str =
    "Прежде чем активировать уведомления, отправьте мне заполненный JSON файл с днями рождения или выполните \
    команду /cancel для выхода из режима обновления дней рождений";

/// The message to send when the user tries to activate the bot without adding any birthdays.
const ACTIVE_EMPTY_LIST: &str =
    "Ни одного дня рождения не добавлено 😞, поэтому уведомления от меня не активны. \
    Для добавления дней рождений выполните команду /add или /addmany";

/// The message to send when the user tries to activate the bot without adding any birthdays via `add` command.
const ACTIVE_WAITING_BIR_MSG: &str =
    "Прежде чем активировать уведомления, выполните команду /cancel, чтобы выйти из режима обновления дней рождений";

/// The message to send when the user tries to disable the bot.
const DISABLE_MSG: &str = "Уведомления от меня отключены!";

/// The message to send when the user tries to disable the bot when it is already disabled.
const DISABLE_ALREADY_DISABLED_MSG: &str = "Уведомления от меня уже отключены!";

/// The message to send when the user tries to disable the bot without adding any birthdays via `add` or `addmany` commands.
const DISABLE_WAITING_MSG: &str =
    "Прежде чем отключить уведомления, выполните команду /cancel для выхода из режима обновления дней рождений";

/// The message to send when the user tries to disable the bot without adding any birthdays.
const DISABLE_EMPTY_LIST: &str =
    "Ни одного дня рождения не добавлено 😞, поэтому уведомления от меня не активны. \
    Для добавления дней рождений выполните команду /add или /addmany";

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
        super::AdminCommands::Add => handle_add_command(bot, msg, cfg).await,
        super::AdminCommands::AddMany => handle_add_many_command(bot, msg, cfg).await,
        super::AdminCommands::Cancel => handle_cancel_command(bot, msg, cfg).await,
        super::AdminCommands::Active => handle_active_command(bot, msg, cfg).await,
        super::AdminCommands::Disable => handle_disable_command(bot, msg, cfg).await,
        super::AdminCommands::List => handle_list_command(bot, msg, cfg).await,
        super::AdminCommands::Remove => handle_remove_command(bot, msg, cfg).await,
    }
}

/// Handles the `add` command for the bot.
/// This function sets the bot state to `WaitingBirthday` for the chat and sends a message
/// to the chat with instructions on how to add a birthday.
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
async fn handle_add_command(bot: Bot, msg: Message, cfg: ConfigParameters) -> ResponseResult<()> {
    log::info!("Add command received from chat id {}", msg.chat.id);

    let mut b_map = cfg.b_map.write().await;

    match b_map.update_state(&msg.chat.id, State::WaitingBirthday) {
        Ok(_) => {
            bot.send_message(msg.chat.id, ADD_MSG).await?;
        }
        Err(_) => {
            bot.send_message(msg.chat.id, BUSY_MSG).await?;
        }
    }
    Ok(())
}

/// Handles the `addmany` command for the bot.
/// This function sets the bot state to `WaitingJson` for the chat and sends a message
/// to the chat with instructions on how to add a JSON file with birthdays.
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
async fn handle_add_many_command(
    bot: Bot,
    msg: Message,
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    log::info!("AddMany command received from chat id {}", msg.chat.id);

    let mut b_map = cfg.b_map.write().await;

    match b_map.update_state(&msg.chat.id, State::WaitingJson) {
        Ok(_) => {
            bot.send_message(msg.chat.id, JSON_MSG).await?;
            bot.send_document(msg.chat.id, InputFile::file(super::SAMPLE_JSON_FILE_PATH))
                .await?;
            Ok(())
        }
        Err(_) => {
            bot.send_message(msg.chat.id, BUSY_MSG).await?;
            Ok(())
        }
    }
}

/// Handles the `cancel` command for the bot.
/// This function sets the bot state to `Disabled` to disable the birthday addition mode for the chat
/// and sends a message to the chat to confirm the cancellation and list the current birthdays.
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
async fn handle_cancel_command(
    bot: Bot,
    msg: Message,
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    log::info!("Cancel command received from chat id {}", msg.chat.id);

    let mut b_map = cfg.b_map.write().await;

    match b_map.get_mut(&msg.chat.id) {
        Some((state, _)) => match state {
            State::WaitingBirthday | State::WaitingJson | State::WaitingRemoving => {
                match b_map.update_state(&msg.chat.id, State::Disabled) {
                    Ok(_) => {
                        bot.send_message(msg.chat.id, CANCEL_MSG).await?;
                        let (_, birthdays) = b_map.get(&msg.chat.id).unwrap();
                        bot.send_message(msg.chat.id, birthdays.list().as_str())
                            .await?;
                    }
                    Err(_) => {
                        bot.send_message(msg.chat.id, BUSY_MSG).await?;
                    }
                }
            }
            _ => {
                bot.send_message(msg.chat.id, CANCEL_ALREADY_DISABLED_MSG)
                    .await?;
            }
        },
        None => {
            if let Err(_) = b_map.insert(msg.chat.id, State::Disabled, Birthdays::default()) {
                bot.send_message(msg.chat.id, BUSY_MSG).await?;
            } else {
                bot.send_message(msg.chat.id, CANCEL_EMPTY_LIST_MSG).await?;
            }
        }
    }

    Ok(())
}

/// Handles the `list` command for the bot.
/// This function sends a message to the chat with the list of current birthdays for the chat.
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
    log::info!("List command received from chat id {}", msg.chat.id);

    let b_map = cfg.b_map.read().await;
    let birthdays_default = Birthdays::default();
    let birthdays = b_map
        .get(&msg.chat.id)
        .map(|(_, birthdays)| birthdays)
        .unwrap_or(&birthdays_default);

    bot.send_message(msg.chat.id, birthdays.list()).await?;

    Ok(())
}

/// Handles the `active` command for the bot.
/// This function activates the bot for the chat and sends a message to the chat to confirm the activation.
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
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    log::info!("Active command received from chat id {}", msg.chat.id);

    let mut b_map = cfg.b_map.write().await;

    match b_map.get_mut(&msg.chat.id) {
        Some((state, birthdays)) => match state {
            State::Disabled => {
                if birthdays.is_empty() {
                    bot.send_message(msg.chat.id, ACTIVE_EMPTY_LIST).await?;
                } else {
                    *state = State::Active;
                    bot.send_message(msg.chat.id, ACTIVE_MSG).await?;
                }
            }
            State::Active => {
                bot.send_message(msg.chat.id, ACTIVE_ALREADY_ACTIVE_MSG)
                    .await?;
            }
            State::WaitingJson => {
                bot.send_message(msg.chat.id, ACTIVE_WAITING_JSON_MSG)
                    .await?;
            }
            State::WaitingBirthday | State::WaitingRemoving => {
                bot.send_message(msg.chat.id, ACTIVE_WAITING_BIR_MSG)
                    .await?;
            }
        },
        None => {
            if let Err(_) = b_map.insert(msg.chat.id, State::Disabled, Birthdays::default()) {
                bot.send_message(msg.chat.id, BUSY_MSG).await?;
            } else {
                bot.send_message(msg.chat.id, ACTIVE_EMPTY_LIST).await?;
            }
        }
    }

    Ok(())
}

/// Handles the `disable` command for the bot.
/// This function disables the bot for the chat and sends a message to the chat to confirm the deactivation.
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

    let mut b_map = cfg.b_map.write().await;

    match b_map.get_mut(&msg.chat.id) {
        Some((state, _)) => match state {
            State::Disabled => {
                bot.send_message(msg.chat.id, DISABLE_ALREADY_DISABLED_MSG)
                    .await?;
            }
            State::Active => {
                *state = State::Disabled;
                bot.send_message(msg.chat.id, DISABLE_MSG).await?;
            }
            State::WaitingJson | State::WaitingBirthday | State::WaitingRemoving => {
                bot.send_message(msg.chat.id, DISABLE_WAITING_MSG).await?;
            }
        },
        None => {
            if let Err(_) = b_map.insert(msg.chat.id, State::Disabled, Birthdays::default()) {
                bot.send_message(msg.chat.id, BUSY_MSG).await?;
            } else {
                bot.send_message(msg.chat.id, DISABLE_EMPTY_LIST).await?;
            }
        }
    }

    Ok(())
}

/// Handles the `remove` command for the bot.
/// This function sets the bot state to `WaitingRemoving` for the chat and sends a message
/// to the chat with instructions on how to remove a birthday.
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
async fn handle_remove_command(
    bot: Bot,
    msg: Message,
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    log::info!("Remove command received from chat id {}", msg.chat.id);

    let mut b_map = cfg.b_map.write().await;

    match b_map.update_state(&msg.chat.id, State::WaitingRemoving) {
        Ok(_) => {
            bot.send_message(msg.chat.id, REMOVE_MSG).await?;
            let (_, birthdays) = b_map.get(&msg.chat.id).unwrap();
            bot.send_message(msg.chat.id, birthdays.list()).await?;
        }
        Err(_) => {
            bot.send_message(msg.chat.id, BUSY_MSG).await?;
        }
    }

    Ok(())
}
