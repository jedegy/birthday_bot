use serde::{Deserialize, Serialize};
use teloxide::prelude::ChatId;
use tokio::sync::RwLock;

use std::collections::HashMap;
use std::sync::Arc;

/// The limit size of the birthdays map in bytes.
pub const BIRTHDAY_MAP_LIMIT: usize = 256 * 1024 * 1024;

/// A thread-safe map of chat IDs to bot states and birthdays.
pub type BirthdaysMap = Arc<RwLock<HashMap<ChatId, (super::State, Birthdays)>>>;

/// Represents a birthday with a name, date, and username.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Birthday {
    /// The name of the person.
    pub name: String,
    /// The date of the birthday.
    pub date: String,
    /// The username of the person.
    pub username: String,
}

/// Represents a list of birthdays.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Birthdays {
    /// The list of birthdays.
    birthdays: Vec<Birthday>,
}

impl Birthdays {
    /// Returns the number of birthdays in the list.
    pub fn len(&self) -> usize {
        self.birthdays.len()
    }

    /// Returns a reference to the list of birthdays.
    pub fn get_birthdays(&self) -> &Vec<Birthday> {
        &self.birthdays
    }
}
