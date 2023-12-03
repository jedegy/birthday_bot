use async_tempfile::TempFile;
use teloxide::net::Download;
use teloxide::prelude::{Message, Request, Requester, ResponseResult};
use teloxide::Bot;

use crate::{Birthdays, ConfigParameters, State};

/// Handles document messages for the bot.
///
/// # Arguments
///
/// * `bot` - The bot instance.
/// * `msg` - The message containing the document.
/// * `cfg` - Configuration parameters for the bot.
///
/// # Returns
///
/// A `ResponseResult` indicating the success or failure of the command.
pub async fn document_handler(bot: Bot, msg: Message, cfg: ConfigParameters) -> ResponseResult<()> {
    if let Some(doc) = msg.document() {
        log::info!("Document received from chat id {}", msg.chat.id);

        let mut b_map = cfg.b_map.write().await;

        let download_file = b_map
            .get(&msg.chat.id)
            .map_or(false, |entry| entry.0 == State::WaitingJson);

        if download_file {
            let file = doc.file.clone();
            log::info!("Downloading file {} from chat id {}", file.id, msg.chat.id);

            let file_info = bot.get_file(file.id).send().await?;
            let mut temp_file = TempFile::new().await.unwrap();
            bot.download_file(&file_info.path, &mut temp_file).await?;

            let file_content: String = tokio::fs::read_to_string(temp_file.file_path()).await?;

            match serde_json::from_str(&file_content) {
                Ok(birthdays) => {
                    b_map.insert(msg.chat.id, (State::Active, birthdays));
                    bot.send_message(
                        msg.chat.id,
                        "Дни рождения успешно загружены🎉 \
                    Напоминания будут приходить ровно в день рождение в 7:00 UTC",
                    )
                    .await?;
                }
                Err(e) => {
                    log::error!("Failed to parse the file content: {}", e);
                    b_map.insert(msg.chat.id, (State::WaitingJson, Birthdays::default()));
                    bot.send_message(
                        msg.chat.id,
                        "К сожалению, отправленный файл не корректный или содержит ошибки😔 \
                    Проверьте его и отправьте ещё раз",
                    )
                    .await?;
                }
            }
        }
    }

    Ok(())
}
