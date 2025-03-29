# Twitch Soundbot

A self-hosted Twitch EventSub bot that listens for custom channel point
redemptions and plays corresponding sounds locally using rodio.

## Features

- Secure Twitch OAuth2 user token flow (with twitch_oauth2)
- WebSocket connection to Twitch EventSub
- Auto-registration of channel point redemption events
- Plays matching .mp3 files from a sounds/ directory
- Interactive config setup (.env generation)
- Automatic recovery via refresh tokens
- Threaded sound playback for overlapping redemptions

## Setup

### 1. Prerequisites

- Twitch app credentials (Client ID + Client Secret)
- A local sounds/ directory containing .mp3 files
- Rust toolchain (cargo, rustc)

### 2. Build and Run
```
cargo run
```

First-time use will prompt you to log in via Twitch to authorize the bot.
Config is stored at `~/.config/twitch-soundbot/.env`.
Token is saved at `~/.config/twitch-soundbot/token.json`.

### 3. Configuration Options (.env)

| Variable        | Description                                 |
|----------------|---------------------------------------------|
| CLIENT_ID      | Twitch app Client ID                        |
| CLIENT_SECRET  | Twitch app Client Secret                    |
| REDIRECT_URI   | Where Twitch should redirect after login    |
| BROADCASTER_ID | Twitch username to monitor                  |
| BIND_ADDRESS   | Local bind address for internal use         |
| EVENTSUB_SECRET| Secret used when validating EventSub        |

### 4. Sound Matching

When a user redeems a reward titled "CoolSound", the bot looks for a file like
`sounds/CoolSound.mp3` (case-insensitive) and plays it. Drop .mp3 files into
the sounds/ folder with matching names.

## Project Structure

- auth.rs: Token storage, validation, and OAuth2 flow
- config.rs: Interactive setup and .env loading
- eventsub.rs: Twitch WebSocket handling and subscription logic
- redemption.rs: Parses incoming events and triggers sound playback
- sound.rs: Handles loading and playing audio

## Testing

Tests include:

- Unit tests for redemption handling and session parsing
- Integration tests for .env loading
- Overlapping sound playback concurrency test

Run tests with:
```
cargo test
```
