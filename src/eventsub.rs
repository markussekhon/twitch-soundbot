use futures_util::StreamExt;
use reqwest;
use serde::Serialize;
use serde_json::Value;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio_tungstenite::connect_async;
use twitch_oauth2::TwitchToken; // for token().secret()

/// Struct for the condition in the subscription payload.
#[derive(Serialize)]
struct Condition {
    broadcaster_user_id: String,
}

/// Struct for the websocket transport in the subscription payload.
/// Twitch requires a session_id when using the websocket transport.
#[derive(Serialize)]
struct WsTransport {
    method: String,     // should be "websocket"
    session_id: String, // the session_id from the welcome message
}

/// Subscription payload for websocket-based subscriptions.
#[derive(Serialize)]
struct WsSubscriptionPayload {
    #[serde(rename = "type")]
    event_type: String,
    version: String,
    condition: Condition,
    transport: WsTransport,
}

/// Helper: Given a JSON text, attempt to extract a session_id
/// This expects the message structure to have:
/// - "metadata.message_type" == "session_welcome"
/// - The session ID at "payload.session.id"
pub fn extract_session_id(text: &str) -> Option<String> {
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        if v.get("metadata")
            .and_then(|m| m.get("message_type"))
            .and_then(|t| t.as_str())
            == Some("session_welcome")
        {
            if let Some(session) =
                v.get("payload").and_then(|p| p.get("session"))
            {
                return session
                    .get("id")
                    .and_then(|id| id.as_str())
                    .map(|s| s.to_string());
            }
        }
    }
    None
}

/// Connects to Twitch’s EventSub WebSocket endpoint and waits.
/// Returns the WebSocket stream and the extracted session_id.
pub async fn connect_eventsub_ws() -> Result<
    (
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        String,
    ),
    Box<dyn Error>,
> {
    // Use the production WebSocket endpoint per Twitch docs.
    let ws_url = "wss://eventsub.wss.twitch.tv/ws";
    let (ws_stream, _) = connect_async(ws_url).await?;
    println!("Connected to Twitch EventSub WebSocket endpoint.");

    let mut stream = ws_stream;
    let mut session_id = None;
    // Wait up to 10 seconds for a welcome message.
    for _ in 0..10 {
        if let Some(msg) = stream.next().await {
            let msg = msg?;
            if msg.is_text() {
                let text = msg.to_text()?;
                //TODO: Create a format message function.
                //println!("Received message: {}", text);
                if let Some(sid) = extract_session_id(text) {
                    session_id = Some(sid);
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    if let Some(sid) = session_id {
        Ok((stream, sid))
    } else {
        Err("Failed to receive session welcome message".into())
    }
}

/// Looks up the numeric broadcaster ID from Twitch given a username.
/// This calls the Get Users API and returns the numeric user ID.
async fn get_numeric_broadcaster_id(
    username: &str,
    token: &str,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let client_id = env::var("CLIENT_ID")?;
    let url = format!("https://api.twitch.tv/helix/users?login={}", username);
    let res = client
        .get(&url)
        .header("Client-ID", client_id)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    if !res.status().is_success() {
        return Err(format!(
            "Failed to fetch broadcaster id: {}",
            res.status()
        )
        .into());
    }
    let json: Value = res.json().await?;
    if let Some(data) = json.get("data") {
        if let Some(user) = data.as_array().and_then(|arr| arr.get(0)) {
            if let Some(id) = user.get("id").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }
    }
    Err("No broadcaster id found".into())
}

/// Registers a websocket subscription with Twitch using the provided session_id.
pub async fn register_ws_subscription(
    token: &str,
    broadcaster_numeric_id: &str,
    session_id: &str,
) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let payload = WsSubscriptionPayload {
        event_type: "channel.channel_points_custom_reward_redemption.add"
            .to_string(),
        version: "1".to_string(),
        condition: Condition {
            broadcaster_user_id: broadcaster_numeric_id.to_string(),
        },
        transport: WsTransport {
            method: "websocket".to_string(),
            session_id: session_id.to_string(),
        },
    };

    let response = client
        .post("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Client-ID", env::var("CLIENT_ID")?)
        .header("Authorization", format!("Bearer {}", token))
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Successfully registered websocket subscription.");
        Ok(())
    } else {
        let text = response.text().await?;
        Err(format!("Failed to register subscription: {}", text).into())
    }
}

/// Connects to the Twitch EventSub WebSocket, registers a subscription using
/// the session_id, and processes incoming messages. Redemption events are
/// delegated to the redemption handler.
pub async fn run_eventsub_ws_service(
    token: &twitch_oauth2::UserToken,
) -> Result<(), Box<dyn Error>> {
    // Get the broadcaster identifier from env vars.
    let provided_broadcaster = env::var("BROADCASTER_ID")?;
    let token_str = token.token().secret();

    // Look up the numeric broadcaster ID from Twitch.
    let numeric_broadcaster_id =
        get_numeric_broadcaster_id(&provided_broadcaster, token_str).await?;
    println!("Numeric broadcaster ID: {}", numeric_broadcaster_id);

    // Connect to the WebSocket endpoint and obtain the session_id.
    let (ws_stream, session_id) = connect_eventsub_ws().await?;
    println!("Obtained session_id: {}", session_id);

    // Register the websocket subscription using the numeric broadcaster id.
    register_ws_subscription(token_str, &numeric_broadcaster_id, &session_id)
        .await?;

    println!("Running WebSocket message loop...");
    let (_write, mut read) = ws_stream.split();

    while let Some(message) = read.next().await {
        let message = message?;
        if message.is_text() {
            let text = message.to_text()?;
            //TODO: Create a format message function.
            //println!("Received message: {}", text);
            let event: Value = serde_json::from_str(text)?;
            // Look inside the "payload" object
            if let Some(payload) = event.get("payload") {
                if let Some(subscription) = payload.get("subscription") {
                    if let Some(event_type) =
                        subscription.get("type").and_then(|v| v.as_str())
                    {
                        if event_type == "channel.channel_points_custom_reward_redemption.add" {
                            let payload = payload.clone();
                            std::thread::spawn(|| {
                                crate::redemption::handle_redemption(payload).ok();
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_id_from_welcome() {
        let welcome_msg = r#"
        {
            "metadata": {
                "message_id": "dummy",
                "message_type": "session_welcome",
                "message_timestamp": "2025-04-01T00:00:00Z"
            },
            "payload": {
                "session": {
                    "id": "TestSessionID123",
                    "status": "connected",
                    "connected_at": "2025-04-01T00:00:00Z",
                    "keepalive_timeout_seconds": 10
                }
            }
        }
        "#;
        let session_id = extract_session_id(welcome_msg);
        assert_eq!(session_id, Some("TestSessionID123".to_string()));
    }

    #[tokio::test]
    async fn test_get_numeric_broadcaster_id_invalid() {
        let result =
            get_numeric_broadcaster_id("nonexistentuser", "fake_token").await;
        assert!(result.is_err());
    }
}
