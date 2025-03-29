use dirs;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Returns the path to the configuration file.
fn config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = dirs::config_dir().ok_or("No config directory")?;
    path.push("twitch-soundbot");
    fs::create_dir_all(&path)?;
    path.push(".env");
    Ok(path)
}

/// Prompts the user for configuration and writes it to disk.
fn interactive_setup(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("No config found. Let's set it up.");

    fn prompt(msg: &str) -> Result<String, Box<dyn std::error::Error>> {
        print!("{}: ", msg);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    let client_id = prompt("CLIENT_ID")?;
    let client_secret = prompt("CLIENT_SECRET")?;
    let redirect_uri = prompt("REDIRECT_URI (default http://localhost/)")?;
    let broadcaster_id = prompt("BROADCASTER_ID")?;
    let bind_address = prompt("BIND_ADDRESS (default 127.0.0.1:17564)")?;

    println!(
        "To generate a 32-character EVENTSUB_SECRET, \n\
         enter a random word (this will seed the RNG):"
    );
    let seed_word = prompt("Random word")?;

    // Generate secret
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    seed_word.hash(&mut hasher);
    let seed = hasher.finish();

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let charset = b"abcdefghijklmnopqrstuvwxyz\
                    ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                    0123456789";

    let eventsub_secret: String = (0..32)
        .map(|_| {
            let idx = rng.random_range(0..charset.len());
            charset[idx] as char
        })
        .collect();

    let redirect_uri = if redirect_uri.is_empty() {
        "http://localhost/".to_string()
    } else {
        redirect_uri
    };

    let bind_address = if bind_address.is_empty() {
        "127.0.0.1:17564".to_string()
    } else {
        bind_address
    };

    let env_content = format!(
        "CLIENT_ID={}\n\
         CLIENT_SECRET={}\n\
         REDIRECT_URI={}\n\
         BROADCASTER_ID={}\n\
         BIND_ADDRESS={}\n\
         EVENTSUB_SECRET={}\n",
        client_id,
        client_secret,
        redirect_uri,
        broadcaster_id,
        bind_address,
        eventsub_secret,
    );

    fs::write(path, env_content)?;
    println!("Config written to {:?}", path);
    Ok(())
}

/// Loads the config file into the environment or creates one if missing.
pub fn ensure_config() -> Result<(), Box<dyn std::error::Error>> {
    let path = config_path()?;
    if path.exists() {
        dotenvy::from_path(&path)?;
    } else {
        interactive_setup(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod config_tests {
    #[test]
    fn test_loads_expected_env_values() {
        fn clear_env() {
            for key in [
                "CLIENT_ID",
                "CLIENT_SECRET",
                "REDIRECT_URI",
                "BROADCASTER_ID",
                "BIND_ADDRESS",
                "EVENTSUB_SECRET",
            ] {
                std::env::remove_var(key);
            }
        }

        clear_env();

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let content = "\
CLIENT_ID=abc123
CLIENT_SECRET=xyz456
REDIRECT_URI=http://localhost:9000
BROADCASTER_ID=channel_xyz
BIND_ADDRESS=0.0.0.0:9001
EVENTSUB_SECRET=another_32_char_secret_value
";

        std::fs::write(path, content).unwrap();

        dotenvy::from_path(path).unwrap();

        assert_eq!(std::env::var("CLIENT_ID").unwrap(), "abc123");
        assert_eq!(std::env::var("CLIENT_SECRET").unwrap(), "xyz456");
        assert_eq!(
            std::env::var("REDIRECT_URI").unwrap(),
            "http://localhost:9000"
        );
        assert_eq!(std::env::var("BROADCASTER_ID").unwrap(), "channel_xyz");
        assert_eq!(std::env::var("BIND_ADDRESS").unwrap(), "0.0.0.0:9001");
        assert_eq!(
            std::env::var("EVENTSUB_SECRET").unwrap(),
            "another_32_char_secret_value"
        );
    }
}
