mod auth;
mod config;
mod eventsub;
mod redemption;
mod sound;

use auth::StoredToken;
use config::ensure_config;
use eventsub::run_eventsub_ws_service;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration (interactive if missing)
    ensure_config()?;

    // Obtain a Twitch token (using your existing user token flow)
    let user_token = StoredToken::ensure_twitch_token().await?;

    // Run the EventSub WebSocket service using the obtained token.
    run_eventsub_ws_service(&user_token).await?;

    Ok(())
}
