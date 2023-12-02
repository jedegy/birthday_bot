use clap::Parser;
use std::path::PathBuf;

/// The arguments for the bot.
#[derive(Parser, Debug)]
#[command(author, version = "0.2.0", about = "A Telegram bot that sends birthday reminders", long_about = None)]
pub struct Args {
    /// The path to the token file.
    #[arg(short, long)]
    pub token_path: Option<PathBuf>,

    /// The path to the backup file.
    #[arg(short, long)]
    pub backup_path: PathBuf,

    /// The user ID of the bot maintainer.
    #[arg(short, long)]
    pub maintainer_user_id: Option<u64>,
}
