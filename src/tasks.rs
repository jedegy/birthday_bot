use chrono::{Duration, Utc};
use std::path::PathBuf;
use teloxide::prelude::{ChatId, Requester};
use teloxide::Bot;

/// Constant for the birthday reminder task period in seconds.
const BIRTHDAY_REMINDER_TASK_PERIOD_SEC: i64 = 60 * 60 * 24;

/// Constant for the backup task period in seconds.
const BACKUP_TASK_PERIOD_SEC: i64 = 60 * 60 * 24;

/// Constant for the health check task period in seconds.
const HEALTH_CHECK_TASK_PERIOD_SEC: i64 = 60 * 5;

/// Sends a health check message to the maintainer of the bot.
///
/// # Arguments
///
/// * `bot` - The bot instance.
///
/// This function sends a health check message to the maintainer of the bot
/// at 7:10 AM UTC daily.
pub async fn health_check_task(bot: Bot) {
    loop {
        // Calculate the time for the next health check.
        let now = Utc::now().naive_utc();
        let next_run = (now + Duration::seconds(HEALTH_CHECK_TASK_PERIOD_SEC))
            .date()
            .and_hms_opt(7, 10, 0)
            .unwrap_or_default();
        let duration_until_next_run = (next_run - now).to_std().unwrap_or_default();

        // Sleep until the next health check time.
        tokio::time::sleep(duration_until_next_run).await;

        // Send a health check message.
        match bot
            .send_message(ChatId(super::MAINTAINER_USER_ID as i64), "I'm alive!")
            .await
        {
            Ok(_) => log::info!("Health check message sent successfully"),
            Err(e) => log::error!("Error during sending health check message: {}", e),
        }
    }
}

/// This function saves the data to a JSON file on a daily basis at 12:00 PM UTC.
///
/// # Arguments
///
/// * `data` - The data to save.
/// * `path` - The path to the JSON file.
///
/// # Returns
///
/// A `Result` indicating the data was saved or not.
pub async fn daily_backup_task(data: super::BirthdaysMap, backup_path: PathBuf) {
    loop {
        // Calculate the time for the next backup.
        let now = Utc::now().naive_utc();
        let next_run = (now + Duration::seconds(BACKUP_TASK_PERIOD_SEC))
            .date()
            .and_hms_opt(12, 0, 0)
            .unwrap_or_default();
        let duration_until_next_run = (next_run - now).to_std().unwrap_or_default();

        // Wait until the next backup time.
        tokio::time::sleep(duration_until_next_run).await;

        // Save data to JSON
        match crate::utils::save_to_json(data.clone(), &backup_path).await {
            Ok(_) => log::info!("Birthday data successfully saved to JSON"),
            Err(e) => log::error!("Error during saving birthday data to JSON: {}", e),
        }
    }
}

/// Sends birthday reminders on a daily basis.
///
/// # Arguments
///
/// * `bot` - The bot instance.
/// * `birthdays_map` - A thread-safe map of chat IDs to bot states and birthdays.
///
/// This function sends reminders about upcoming birthdays to chats
/// with an active bot state. The reminders are sent at 7:00 AM UTC daily.
pub async fn send_birthday_reminders(
    bot: Bot,
    birthdays_map: super::BirthdaysMap,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Calculate the time for the next reminder.
        let now = Utc::now().naive_utc();
        let next_run = (now + Duration::seconds(BIRTHDAY_REMINDER_TASK_PERIOD_SEC))
            .date()
            .and_hms_opt(7, 0, 0)
            .unwrap_or_default();
        let duration_until_next_run = (next_run - now).to_std().unwrap_or_default();

        // Sleep until the next reminder time.
        tokio::time::sleep(duration_until_next_run).await;

        let mut output = Vec::new();
        {
            let b_map = birthdays_map.read().await;

            for (chat_id, (state, vec)) in b_map.iter() {
                if super::State::Active == *state {
                    for birthday in vec.birthdays.iter() {
                        if birthday.date == Utc::now().format("%d-%m").to_string() {
                            let username_text = if !birthday.username.is_empty() {
                                format!("({})", birthday.username)
                            } else {
                                "".into()
                            };

                            let text = format!(
                                "Поздравьте сегодня замечательного человека с днем рождения {} {}!🎉",
                                birthday.name, username_text
                            );
                            output.push((*chat_id, text));
                        }
                    }
                }
            }
        }

        // Send the reminders.
        for (chat_id, text) in output {
            bot.send_message(chat_id, text).await?;
        }
    }
}
