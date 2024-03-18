use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use teloxide::prelude::ChatId;
use tokio::sync::RwLock;

use crate::State;

/// The limit size of the birthdays map in bytes.
pub const BIRTHDAY_MAP_LIMIT: usize = 256 * 1024 * 1024;

/// The type alias for thread-safe map of chat IDs to bot states and birthdays.
pub type BirthdaysMapThreadSafe = Arc<RwLock<BirthdaysMap>>;

/// Represents the kind of error that can occur when updating the birthdays map.
#[derive(Debug)]
pub enum ErrorKind {
    BirthdayMapFull,
}

/// Represents an error that can occur when updating the birthdays map.
#[derive(Debug)]
pub struct Error {
    _kind: ErrorKind,
}

impl Error {
    /// Creates a new error with the given kind.
    fn new(kind: ErrorKind) -> Self {
        Self { _kind: kind }
    }
}

/// Represents a map of chat IDs to bot states and birthdays.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BirthdaysMap {
    map: HashMap<ChatId, (State, Birthdays)>,
}

impl Default for BirthdaysMap {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl BirthdaysMap {
    /// Creates a new map of chat IDs to bot states and birthdays.
    ///
    /// # Arguments
    ///
    /// * `map` - The map of chat IDs to bot states and birthdays.
    ///
    /// # Returns
    ///
    /// A new map of chat IDs to bot states and birthdays.
    pub fn new(map: HashMap<ChatId, (State, Birthdays)>) -> Self {
        Self { map }
    }

    /// Returns an iterator over the map of chat IDs to bot states and birthdays.
    pub fn iter(&self) -> impl Iterator<Item = (&ChatId, &(State, Birthdays))> {
        self.map.iter()
    }

    /// Updates the list of birthdays for the given chat ID.
    /// If the chat ID is not present in the map, it will be added with the new birthday.
    /// If amount of memory used by the map exceeds the limit, an error will be returned.
    ///
    /// # Arguments
    ///
    /// * `chat_id` - The chat ID.
    /// * `birthday` - The new birthday.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the operation.
    pub fn update_birthdays(&mut self, chat_id: &ChatId, birthday: Birthday) -> Result<(), Error> {
        if let Some((_, birthdays)) = self.map.get_mut(chat_id) {
            birthdays.birthdays.push(birthday)
        } else {
            if self.estimate_size()
                + std::mem::size_of_val(chat_id)
                + std::mem::size_of_val(&birthday)
                + std::mem::size_of_val(&Birthdays::default())
                > BIRTHDAY_MAP_LIMIT
            {
                return Err(Error::new(ErrorKind::BirthdayMapFull));
            } else {
                let birthdays = Birthdays::new(vec![birthday]);
                self.map
                    .insert(*chat_id, (State::WaitingBirthday, birthdays));
            }
        }

        Ok(())
    }

    /// Extends the list of birthdays for the given chat ID.
    /// If the chat ID is not present in the map, it will be added with the new list of birthdays.
    /// If amount of memory used by the map exceeds the limit, an error will be returned.
    ///
    /// # Arguments
    ///
    /// * `chat_id` - The chat ID.
    /// * `birthdays` - The new list of birthdays.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the operation.
    pub fn extend_birthdays(
        &mut self,
        chat_id: &ChatId,
        birthdays: Birthdays,
    ) -> Result<(), Error> {
        if let Some((_, in_birthdays)) = self.map.get_mut(chat_id) {
            in_birthdays.extend(birthdays);
        } else {
            if self.estimate_size()
                + std::mem::size_of_val(chat_id)
                + std::mem::size_of_val(&birthdays)
                + std::mem::size_of_val(&Birthdays::default())
                > BIRTHDAY_MAP_LIMIT
            {
                return Err(Error::new(ErrorKind::BirthdayMapFull));
            } else {
                self.map.insert(*chat_id, (State::WaitingJson, birthdays));
            }
        }
        Ok(())
    }

    /// Updates the bot state for the given chat ID.
    /// If the chat ID is not present in the map, it will be added with the new state.
    /// If amount of memory used by the map exceeds the limit, an error will be returned.
    ///
    /// # Arguments
    ///
    /// * `chat_id` - The chat ID.
    /// * `state` - The new state.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the operation.
    pub fn update_state(&mut self, chat_id: &ChatId, state: State) -> Result<(), Error> {
        if let Some((in_state, _)) = self.map.get_mut(chat_id) {
            *in_state = state;
        } else {
            if self.estimate_size()
                + std::mem::size_of_val(chat_id)
                + std::mem::size_of_val(&state)
                + std::mem::size_of_val(&Birthdays::default())
                > BIRTHDAY_MAP_LIMIT
            {
                return Err(Error::new(ErrorKind::BirthdayMapFull));
            } else {
                self.map.insert(*chat_id, (state, Birthdays::default()));
            }
        }
        Ok(())
    }

    /// Inserts the given chat ID, state, and birthdays into the map.
    /// If the chat ID is already present in the map, it will be updated with the new state and birthdays.
    /// If amount of memory used by the map exceeds the limit, an error will be returned.
    ///
    /// # Arguments
    ///
    /// * `chat_id` - The chat ID.
    /// * `state` - The new state.
    /// * `birthdays` - The new list of birthdays.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the operation.
    pub fn insert(
        &mut self,
        chat_id: ChatId,
        state: State,
        birthdays: Birthdays,
    ) -> Result<(), Error> {
        if self.estimate_size()
            + std::mem::size_of_val(&chat_id)
            + std::mem::size_of_val(&state)
            + std::mem::size_of_val(&birthdays)
            > BIRTHDAY_MAP_LIMIT
        {
            return Err(Error::new(ErrorKind::BirthdayMapFull));
        }

        self.map.insert(chat_id, (state, birthdays));
        Ok(())
    }

    /// Return the reference to the tuple of bot state and birthdays for the given chat ID.
    ///
    /// # Arguments
    ///
    /// * `chat_id` - The chat ID.
    ///
    /// # Returns
    ///
    /// A reference to the tuple of bot state and birthdays for the given chat ID.
    #[inline(always)]
    pub fn get(&self, chat_id: &ChatId) -> Option<&(State, Birthdays)> {
        self.map.get(chat_id)
    }

    /// Return the mutable reference to the tuple of bot state and birthdays for the given chat ID.
    ///
    /// # Arguments
    ///
    /// * `chat_id` - The chat ID.
    ///
    /// # Returns
    ///
    /// A mutable reference to the tuple of bot state and birthdays for the given chat ID.
    #[inline(always)]
    pub fn get_mut(&mut self, chat_id: &ChatId) -> Option<&mut (State, Birthdays)> {
        self.map.get_mut(chat_id)
    }

    /// Function returns the size of the map in bytes.
    ///
    /// # Returns
    ///
    /// The size of the map in bytes.
    pub fn estimate_size(&self) -> usize {
        let mut size = 0;
        for (chat_id, (state, birthdays)) in self.map.iter() {
            size += std::mem::size_of_val(chat_id);
            size += std::mem::size_of_val(state);
            size += std::mem::size_of_val(birthdays);
        }
        size
    }
}

/// Represents a birthday with a name, date, and username.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
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
    /// Creates a new list of birthdays.
    ///
    /// # Arguments
    ///
    /// * `birthdays` - The list of birthdays.
    ///
    /// # Returns
    ///
    /// A new list of birthdays.
    pub fn new(birthdays: Vec<Birthday>) -> Self {
        Self { birthdays }
    }

    /// Returns an iterator over the list of birthdays.
    pub fn iter(&self) -> impl Iterator<Item = &Birthday> {
        self.birthdays.iter()
    }

    /// Returns the number of birthdays in the list.
    pub fn len(&self) -> usize {
        self.birthdays.len()
    }

    /// Returns whether the list of birthdays is empty.
    pub fn is_empty(&self) -> bool {
        self.birthdays.is_empty()
    }

    /// Extends the list of birthdays with the given list and removes duplicates.
    ///
    /// # Arguments
    ///
    /// * `other` - The list of birthdays to extend with.
    pub fn extend(&mut self, other: Birthdays) {
        self.birthdays.extend(other.birthdays);
        let set: std::collections::HashSet<_> = self.birthdays.drain(..).collect();
        self.birthdays.extend(set.into_iter());
    }

    /// Returns a string representation of the list of birthdays.
    pub fn list(&self) -> String {
        if self.birthdays.is_empty() {
            "Список дней рождений пуст".to_string()
        } else {
            let mut reply_text = String::from("Список дней рождений:\n");
            for (idx, birthday) in self.birthdays.iter().enumerate() {
                reply_text += format!(
                    "{}. {} - {} {}\n",
                    idx, birthday.name, birthday.date, birthday.username
                )
                .as_str();
            }
            reply_text
        }
    }
}
