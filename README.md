# Telegram Bot LLM

A Telegram bot that integrates with OpenAI's API to respond to messages when the bot is mentioned (@bot).

## Features

- Responds when users mention the bot in a message
- Uses OpenAI's API for natural language understanding and generation
- Configurable greeting message and AI model
- Docker support for easy deployment

## Requirements

- Telegram Bot Token (from BotFather)
- OpenAI API Key

## Configuration

The bot uses the following environment variables:

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| `TELOXIDE_TOKEN` | Your Telegram bot token | Yes | - |
| `OPENAI_API_KEY` | Your OpenAI API key | Yes | - |
| `OPENAI_API_BASE` | OpenAI API base URL | No | https://api.openai.com/v1 |
| `OPENAI_MODEL_NAME` | OpenAI model to use | No | gpt-3.5-turbo |
| `BOT_GREETING_MESSAGE` | Custom greeting message for /start and /help commands | No | Auto-generated with model name |

## Running with Docker

1. Copy `.env.example` to `.env` and fill in your Telegram and OpenAI API tokens:

```bash
cp .env.example .env
```

2. Edit the `.env` file with your actual API keys and configuration.

