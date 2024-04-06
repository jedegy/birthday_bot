use serde::{Deserialize, Serialize};

/// Represents the state of the bot in the chat/group.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum State {
    Active,
    Disabled,
    WaitingJson,
    WaitingBirthday,
    WaitingRemoving,
}
