use std::path::PathBuf;

use chrono::{Duration, Utc};
use teloxide::prelude::{ChatId, Requester};
use teloxide::Bot;
use tokio::task::JoinHandle;

/// Constant for the birthday reminder task period in seconds.
const BIRTHDAY_REMINDER_TASK_PERIOD_SEC: i64 = 60 * 60 * 24;

/// Constant for the backup task period in seconds.
const BACKUP_TASK_PERIOD_SEC: i64 = 60 * 60 * 24;

/// Constant for the health check task period in seconds.
const HEALTH_CHECK_TASK_PERIOD_SEC: i64 = 60 * 60 * 24;

/// The task manager for the bot.
pub struct Manager {
    /// The birthday reminder task.
    birthday_reminder: JoinHandle<()>,
    /// The health check task.
    health_check: JoinHandle<()>,
    /// The daily backup task.
    daily_backup: JoinHandle<()>,
}

impl Manager {
    /// Creates a new `Manager` instance.
    ///
    /// # Arguments
    ///
    /// * `birthday_reminder` - The birthday reminder task.
    /// * `health_check` - The health check task.
    /// * `daily_backup` - The daily backup task.
    ///
    /// # Returns
    ///
    /// A new `Manager` instance.
    pub fn new(
        birthday_reminder: JoinHandle<()>,
        health_check: JoinHandle<()>,
        daily_backup: JoinHandle<()>,
    ) -> Self {
        Self {
            birthday_reminder,
            health_check,
            daily_backup,
        }
    }

    /// Returns whether the birthday reminder task is active.
    pub fn is_birthday_reminder_active(&self) -> bool {
        !self.birthday_reminder.is_finished()
    }

    /// Returns whether the health check task is active.
    pub fn is_health_check_active(&self) -> bool {
        !self.health_check.is_finished()
    }

    /// Returns whether the daily backup task is active.
    pub fn is_daily_backup_active(&self) -> bool {
        !self.daily_backup.is_finished()
    }
}

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

/// This function saves the birthdays map to a JSON file on a daily basis at 12:00 PM UTC.
///
/// # Arguments
///
/// * `map` - The thread-safe map of chat IDs to bot states and birthdays.
/// * `path` - The path to the JSON file.
///
/// # Returns
///
/// A `Result` indicating the data was saved or not.
pub async fn daily_backup_task(map: super::BirthdaysMapThreadSafe, backup_path: PathBuf) {
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
        match crate::utils::save_to_json(map.clone(), &backup_path).await {
            Ok(_) => log::info!("Birthdays data successfully saved to JSON"),
            Err(e) => log::error!("Error during saving birthdays data to JSON: {}", e),
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
    birthdays_map: super::BirthdaysMapThreadSafe,
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

            for (chat_id, (state, birthdays)) in b_map.iter() {
                if super::State::Active == *state {
                    for birthday in birthdays.iter() {
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
