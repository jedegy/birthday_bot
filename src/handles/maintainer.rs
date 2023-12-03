use teloxide::prelude::{Message, Requester, ResponseResult};
use teloxide::Bot;

use crate::ConfigParameters;

/// Handles maintainer commands for the bot.
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
pub async fn maintainer_commands_handler(
    bot: Bot,
    msg: Message,
    cmd: super::MaintainerCommands,
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    match cmd {
        super::MaintainerCommands::Status => handle_status_command(bot, msg, cfg).await,
    }
}

/// Handles the `status` command for the bot.
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
async fn handle_status_command(
    bot: Bot,
    msg: Message,
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    let mut reply_text = format!("Бот активен и работает в штатном режиме\n\n");

    reply_text += "Статус внутренних задач:\n";

    reply_text += if cfg.task_manager.is_birthday_reminder_active() {
        "Birthday Reminder Task (Активна) 🟢\n"
    } else {
        "Birthday Reminder Task (Неактивна) 🔴\n"
    };

    reply_text += if cfg.task_manager.is_daily_backup_active() {
        "Daily Backup Task (Активна) 🟢\n"
    } else {
        "Daily Backup Task (Неактивна) 🔴\n"
    };

    reply_text += if cfg.task_manager.is_health_check_active() {
        "Health Check Task (Активна) 🟢\n"
    } else {
        "Health Check Task (Неактивна) 🔴\n"
    };

    reply_text += format!(
        "\nУтилизация Birthday Map в байтах: {}\n\n",
        crate::utils::birthday_map_estimate_size(cfg.b_map.clone()).await
    )
    .as_str();

    reply_text += "Подробная информация по Birthday Map:\n";

    for (idx, (chat_id, (state, birthdays))) in cfg.b_map.read().await.iter().enumerate() {
        reply_text += match state {
            crate::State::Active => format!(
                "{}. Бот активен в чате {} и содержит {} дней рождений 🟢\n",
                idx,
                chat_id,
                birthdays.len()
            ),
            crate::State::WaitingJson => format!(
                "{}. Бот ожидает загрузки JSON файла в чате {} 🟡\n",
                idx, chat_id
            ),
            crate::State::Disabled => format!("{}. Бот отключен в {} 🔴\n", idx, chat_id),
        }
        .as_str();
    }

    bot.send_message(msg.chat.id, reply_text).await?;

    Ok(())
}
