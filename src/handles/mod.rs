use teloxide::prelude::{Message, Requester, ResponseResult, UserId};
use teloxide::types::InputFile;
use teloxide::utils::command::BotCommands;
use teloxide::Bot;

mod admin;
mod document;
mod maintainer;

pub use admin::admin_commands_handler;
pub use document::document_handler;
pub use maintainer::maintainer_commands_handler;

/// The file path for the sample JSON birthdays file.
const SAMPLE_JSON_FILE_PATH: &str = "sample.json";

/// The greetings message for the bot.
const GREETINGS_MSG: &str =
    "Привет! Этот бот создан для тех, кто постоянно забывает про дни рождения😁\n
С помощью него вы никогда не забудете поздравить своих друзей, коллег по работе или родственников.\n
Для более подробной информации по настройке используйте команду /help.";

/// The messages to send when checking the control requested.
const CREATOR_MESSAGE: &str = "Вы мой создатель!🙏";
const ADMIN_INTERACTION_PREFIX: &str = "Вы можете взаимодействовать со мной ";
const NO_INTERACTION_PREFIX: &str = "К сожалению, вы не можете взаимодействовать со мной ";

/// Enum defining maintainer commands for the bot.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum MaintainerCommands {
    #[command(description = "Проверяет статус бота")]
    Status,
}

/// Enum defining admin commands for the bot.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum AdminCommands {
    #[command(description = "Включает уведомления о днях рождениях от меня")]
    Active,
    #[command(description = "Отключает уведомления о днях рождениях от меня")]
    Disable,
    #[command(description = "Отображает список дней рождений")]
    List,
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

/// Handles base commands for the bot.
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
pub async fn base_commands_handler(
    bot: Bot,
    me: teloxide::types::Me,
    msg: Message,
    cmd: Command,
) -> ResponseResult<()> {
    // Determine the user ID of the message sender.
    let user_id = msg.from().unwrap().id;

    match cmd {
        Command::Start => {
            bot.send_message(msg.chat.id, GREETINGS_MSG.to_string())
                .await?;
        }
        Command::Help => handle_help_command(&bot, &me, &msg, user_id).await?,
        Command::CheckControl => handle_check_control_command(&bot, &msg, user_id).await?,
        Command::File => {
            bot.send_document(msg.chat.id, InputFile::file(SAMPLE_JSON_FILE_PATH))
                .await?;
        }
    }

    Ok(())
}

/// Function handles `check control` command
///
/// # Arguments
///
/// * `bot` - The bot instance
/// * `msg` - The message triggering the command
/// * `user_id` - The user id
///
/// # Returns
///
/// A `Result` indicating the success or failure of the command handling.
async fn handle_check_control_command(
    bot: &Bot,
    msg: &Message,
    user_id: UserId,
) -> ResponseResult<()> {
    let place = super::utils::get_place(&msg.chat);

    let text = if super::utils::is_maintainer(user_id) {
        CREATOR_MESSAGE.to_string()
    } else {
        match super::utils::is_admin(&bot, msg.chat.id, user_id).await {
            Ok(is_admin) if is_admin => format!("{}{}!😄", ADMIN_INTERACTION_PREFIX, place),
            _ => format!("{}{}😞", NO_INTERACTION_PREFIX, place),
        }
    };

    bot.send_message(msg.chat.id, text).await?;

    Ok(())
}

/// Function handles `help` command
///
/// # Arguments
///
/// * `bot` - The bot instance
/// * `me` - Information about the bot itself
/// * `msg` - The message triggering the command
/// * `user_id` - The user id
///
/// # Returns
///
/// A `Result` indicating the success or failure of the command handling.
async fn handle_help_command(
    bot: &Bot,
    me: &teloxide::types::Me,
    msg: &Message,
    user_id: UserId,
) -> ResponseResult<()> {
    let place = super::utils::get_place(&msg.chat);

    let is_admin = super::utils::is_admin(&bot, msg.chat.id, user_id)
        .await
        .unwrap_or_default();
    let is_maintainer = super::utils::is_maintainer(user_id);

    let base_description =
        if msg.chat.is_group() || msg.chat.is_supergroup() || msg.chat.is_channel() {
            Command::descriptions().username_from_me(&me).to_string()
        } else {
            Command::descriptions().to_string()
        };

    let admin_description =
        if msg.chat.is_group() || msg.chat.is_supergroup() || msg.chat.is_channel() {
            AdminCommands::descriptions()
                .username_from_me(&me)
                .to_string()
        } else {
            AdminCommands::descriptions().to_string()
        };

    let mut text = base_description;
    if is_maintainer
        || (is_admin && (msg.chat.is_group() || msg.chat.is_supergroup() || msg.chat.is_channel()))
        || msg.chat.is_chat()
    {
        text = format!(
            "{}\n{}",
            text,
            admin_description
                .lines()
                .map(|line| format!("{} {}\n", line, place))
                .collect::<String>()
        );
    }

    if is_maintainer {
        let maintainer_description =
            if msg.chat.is_group() || msg.chat.is_supergroup() || msg.chat.is_channel() {
                MaintainerCommands::descriptions()
                    .username_from_me(&me)
                    .to_string()
            } else {
                MaintainerCommands::descriptions().to_string()
            };
        text = format!("{}\n{}", text, maintainer_description);
    }

    bot.send_message(msg.chat.id, text).await?;

    Ok(())
}
