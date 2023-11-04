# Telegram Birthday Reminder Bot

This is a Telegram bot designed to help users remember the birthdays of their friends and loved ones.

## Installation
1. Clone this repository.
2. Create a bot in Telegram using `@BotFather` and obtain a token.
3. Add the token to the environment variable named `BIRTHDAY_REMINDER_BOT_TOKEN`, or create a file named `token.txt` and paste the token there.
4. Run the following command:

`cargo build --release`

## Features

The bot can be configured by administrators of groups or channels, or when the bot is added to a chat. To set up the bot, you need to send it a JSON file with the birthdays filled in. The bot can send a sample JSON file, or you can request one using the `/file` command. The bot checks and sends notifications at **9:00 AM UTC**. Unfortunately, this time cannot be adjusted.

Main commands:
- `/start` — Displays a welcome message.
- `/help` — Displays this help message.
- `/checkcontrol` — Initiates a permissions check.
- `/file` — Request a sample filled JSON file.
- `/active` — Enables birthday notifications in this chat.
- `/disable` — Disables birthday notifications in this chat.
