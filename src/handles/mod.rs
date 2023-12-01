use teloxide::prelude::{Message, Requester};
use teloxide::types::InputFile;
use teloxide::utils::command::BotCommands;
use teloxide::{Bot, RequestError};

mod admin;
mod document;

pub use admin::{handle_active_command, handle_disable_command};
pub use document::handle_document;

/// The file path for the sample JSON birthdays file.
const SAMPLE_JSON_FILE_PATH: &str = "sample.json";

/// The greetings message for the bot.
const GREETINGS_MSG: &str =
    "Привет! Этот бот создан для тех, кто постоянно забывает про дни рождения😁\n
С помощью него вы никогда не забудете поздравить своих друзей, коллег по работе или родственников.\n
Для более подробной информации по настройке используйте команду /help.";

/// Enum defining admin commands for the bot.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum AdminCommands {
    #[command(description = "Включает уведомления о днях рождениях от меня")]
    Active,
    #[command(description = "Отключает уведомления о днях рождениях от меня")]
    Disable,
}

/// Enum defining simple commands for the bot.
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "С помощью этих команд вы можете взаимодействовать и управлять мной.🤖\n\n\
    Основные команды доступны только для администраторов групп и каналов, а также если вы добавили меня в чат.\n\
    Проверить свой статус можно с помощью команды /checkcontrol"
)]
pub enum Command {
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
pub async fn commands_handler(
    bot: Bot,
    me: teloxide::types::Me,
    msg: Message,
    cmd: Command,
) -> Result<(), RequestError> {
    // Determine the user ID of the message sender.
    let user_id = msg.from().unwrap().id;

    // Determine the place where the bot is used.
    let place = super::utils::get_place(&msg.chat);

    // Determine the response based on the command.
    let text = match cmd {
        Command::Start => GREETINGS_MSG.to_string(),
        Command::Help => {
            if msg.chat.is_group() || msg.chat.is_supergroup() || msg.chat.is_channel() {
                if super::utils::is_maintainer(user_id)
                    || super::utils::is_admin(&bot, msg.chat.id, user_id)
                        .await
                        .unwrap_or_default()
                {
                    format!(
                        "{}\n{}",
                        Command::descriptions().username_from_me(&me).to_string(),
                        super::utils::format_admin_commands_desc(
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
                    super::utils::format_admin_commands_desc(
                        &AdminCommands::descriptions().to_string(),
                        &place
                    )
                )
            }
        }
        Command::CheckControl => {
            if super::utils::is_maintainer(user_id) {
                "Вы мой создатель!🙏".into()
            } else if super::utils::is_admin(&bot, msg.chat.id, user_id)
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
