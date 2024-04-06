use async_tempfile::TempFile;
use teloxide::net::Download;
use teloxide::prelude::{Message, Request, Requester, ResponseResult};
use teloxide::types::{ChatId, Document};
use teloxide::Bot;

use crate::handles::BUSY_MSG;
use crate::{ConfigParameters, State};

/// Handles common commands for the bot.
/// This function triggers for all messages in chats and depending on the bot state, it processes
/// the message accordingly.
///
/// If the bot is in the `WaitingJson` state, it processes the document message.
/// If the bot is in the `WaitingBirthday` state, it processes the text message.
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
pub async fn common_commands_handler(
    bot: Bot,
    msg: Message,
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let b_map = cfg.b_map.read().await;

    let state = b_map
        .get(&msg.chat.id)
        .map_or(State::Disabled, |(state, _)| state.clone());

    drop(b_map);

    match state {
        State::WaitingJson => {
            if let Some(doc) = msg.document() {
                document_handler(doc, bot, chat_id, cfg).await?
            }
        }
        State::WaitingBirthday => {
            if let Some(text) = msg.text() {
                add_handler(text, bot, chat_id, cfg).await?
            }
        }
        State::WaitingRemoving => {
            if let Some(text) = msg.text() {
                remove_handler(text, bot, chat_id, cfg).await?
            }
        }
        _ => {}
    }

    Ok(())
}

/// Handles text messages for the bot.
/// This function processes the received text as a birthday and updates the bot state accordingly
/// if the input is valid.
///
/// # Arguments
///
/// * `text` - The reference to the received text.
/// * `bot` - The bot instance.
/// * `msg` - The message containing the text.
///
/// # Returns
///
/// A `ResponseResult` indicating the success or failure of the command.
pub async fn add_handler(
    text: &str,
    bot: Bot,
    chat_id: ChatId,
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    log::info!("Birthday received from chat id {}", chat_id);

    if let Some(birthday) = crate::utils::parse_birthday_info(text) {
        let mut b_map = cfg.b_map.write().await;

        if let Err(err) = b_map.update_birthdays(&chat_id, birthday) {
            log::error!("Birthday not added for chat id {}: {:?}", chat_id, err);
            bot.send_message(chat_id, BUSY_MSG).await?;
        } else {
            log::info!("Birthday added for chat id {}", chat_id);
            bot.send_message(chat_id, "День рождения успешно добавлен! 🎉")
                .await?;
        }
    } else {
        log::warn!("Invalid input format");
        bot.send_message(chat_id, "Неверный формат ввода 😔 Попробуйте ещё раз")
            .await?;
    }

    Ok(())
}

/// Handles document messages for the bot.
/// This function processes the received document as JSON file with birthdays and updates the bot
/// state accordingly if the input is valid.
///
/// # Arguments
///
/// * `doc` - The reference to the received document.
/// * `bot` - The bot instance.
/// * `msg` - The message containing the document.
/// * `cfg` - Configuration parameters for the bot.
///
/// # Returns
///
/// A `ResponseResult` indicating the success or failure of the command.
pub async fn document_handler(
    doc: &Document,
    bot: Bot,
    chat_id: ChatId,
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    log::info!("Document received from chat id {}", chat_id);

    let mut b_map = cfg.b_map.write().await;

    let file = doc.file.clone();
    log::info!("Downloading file {} from chat id {}", file.id, chat_id);

    let file_info = bot.get_file(file.id).send().await?;
    let mut temp_file = TempFile::new().await.unwrap();
    bot.download_file(&file_info.path, &mut temp_file).await?;

    let file_content: String = tokio::fs::read_to_string(temp_file.file_path()).await?;

    match serde_json::from_str(&file_content) {
        Ok(birthdays) => {
            if let Err(err) = b_map.extend_birthdays(&chat_id, birthdays) {
                log::error!("Birthdays not added for chat id {}: {:?}", chat_id, err);
                bot.send_message(chat_id, BUSY_MSG).await?;
            } else {
                bot.send_message(chat_id, "Дни рождения успешно загружены! 🎉")
                    .await?;
            }
        }
        Err(e) => {
            log::error!("Failed to parse the file content: {}", e);
            bot.send_message(
                chat_id,
                "К сожалению, отправленный файл не корректный или содержит ошибки😔 \
                    Проверьте его и отправьте ещё раз",
            )
            .await?;
        }
    }

    Ok(())
}

/// Handles removing birthdays for the bot.
/// This function processes the received text as an index of the birthday to remove and updates the
/// bot state accordingly if the input is valid.
///
/// # Arguments
///
/// * `text` - The reference to the received text.
/// * `bot` - The bot instance.
/// * `chat_id` - The chat ID.
/// * `cfg` - Configuration parameters for the bot.
///
/// # Returns
///
/// A `ResponseResult` indicating the success or failure of the command.
pub async fn remove_handler(
    text: &str,
    bot: Bot,
    chat_id: ChatId,
    cfg: ConfigParameters,
) -> ResponseResult<()> {
    log::info!("Birthday index received from chat id {}", chat_id);

    if let Some(index) = crate::utils::parse_birthday_index(text) {
        let mut b_map = cfg.b_map.write().await;

        if let Some(birthday) = b_map.remove_birthday(&chat_id, index) {
            log::info!("Birthday {:?} removed for chat id {}", birthday, chat_id);
            bot.send_message(
                chat_id,
                format!(
                    "День рождение, Имя: {}, Дата: {} успешно удалён!",
                    birthday.name, birthday.date
                ),
            )
            .await?;
        } else {
            log::error!(
                "Birthday with index {} not found for chat id {}",
                index,
                chat_id
            );
            bot.send_message(
                chat_id,
                "День рождение не найден по указанному индексу 😔 Попробуйте ещё раз",
            )
            .await?;
        }
    } else {
        log::warn!("Invalid input format");
        bot.send_message(chat_id, "Неверный формат ввода 😔 Попробуйте ещё раз")
            .await?;
    }

    Ok(())
}
