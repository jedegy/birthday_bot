use chrono::{Duration, Utc};
use teloxide::prelude::Requester;
use teloxide::Bot;

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
        let next_run = (now + Duration::days(1))
            .date()
            .and_hms_opt(7, 0, 0)
            .unwrap();
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
