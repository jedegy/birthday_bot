use serde::{Deserialize, Serialize};

/// Represents the global state of the bot.
#[derive(Clone, PartialEq)]
pub enum GlobalState {
    Normal,
    Busy,
}

/// Represents the state of the bot in the chat/group.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum State {
    Active,
    Disabled,
    WaitingJson,
}
