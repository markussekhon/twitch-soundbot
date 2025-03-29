use axum::http::StatusCode;
use serde_json::Value;

/// Handles a channel point redemption event.
pub fn handle_redemption(payload: Value) -> Result<(), StatusCode> {
    // Attempt to extract the "event" object from the payload.
    if let Some(event) = payload.get("event") {
        // Extract the reward title (if available).
        let reward_title = event
            .get("reward")
            .and_then(|r| r.get("title"))
            .and_then(|t| t.as_str())
            .unwrap_or("unknown reward");

        // Extract the user name (if available).
        let user_name = event
            .get("user_name")
            .and_then(|u| u.as_str())
            .unwrap_or("unknown user");

        crate::sound::play_sound_for_redemption(user_name, reward_title);
    } else {
        println!("No event details found in the payload.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_handle_redemption() {
        // Create a test payload simulating a redemption event.
        let payload = json!({
            "event": {
                "reward": {
                    "title": "CoolSound"
                },
                "user_name": "TestUser"
            }
        });
        let result = handle_redemption(payload);
        assert!(result.is_ok());
    }
}
