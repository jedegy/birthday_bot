# Telegram Birthday Reminder Bot

This is a Telegram bot designed to help users remember the birthdays of their friends and loved ones.

## Installation

1. Clone this repository.
2. Create a bot in Telegram using `@BotFather` and obtain a token.
3. Add the token to the environment variable named `BIRTHDAY_REMINDER_BOT_TOKEN`, or create a file named `token.txt` and
   paste the token there.
4. Run the following command:

`cargo build --release`

## Usage

To run the bot, use the following command:

`cargo run --release`

The following parameters can be used:

- token_path — Path to the file with the bot telegram token. If not specified, the bot will try to read the token from
  the environment variable `BIRTHDAY_REMINDER_BOT_TOKEN`.
- backup_path — Path to the file with the backup of the HashMap with birthdays. If not specified, the bot will try to
  read the backup from the file `backup.json` and save it to the same file.
- maintainer_user_id — Telegram user ID of the maintainer. If not specified, the bot will use the default maintainer
  user ID `437067064`.

## Features

The bot can be configured by administrators of groups or channels, or when the bot is added to a chat. To set up the
bot, you need to send it a JSON file with the birthdays filled in. The bot can send a sample JSON file, or you can
request one using the `/file` command. The bot checks and sends notifications at **7:00 AM UTC**. Unfortunately, this
time cannot be adjusted.

Main commands:

- `/start` — Displays a welcome message.
- `/help` — Displays this help message.
- `/checkcontrol` — Initiates a permissions check.
- `/file` — Request a sample filled JSON file.
- `/add` — Enable adding mode to add a birthday to the list.
- `/addmany` - Enable adding mode to add multiple birthdays to the list.
- `/remove` — Enable removing mode to remove a birthday from the list.
- `/cancel` — Disables adding or removing modes.
- `/active` — Enables birthday notifications in this chat.
- `/disable` — Disables birthday notifications in this chat.
- `/list` — Displays the list of birthdays.
- `/stats` — Displays bot statistics. Only for maintainers.

Also, bot makes daily backups of the HashMap with birthdays every day at **12:00 PM UTC**.
