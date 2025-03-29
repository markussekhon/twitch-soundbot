use dirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;
use twitch_oauth2::{
    tokens::UserTokenBuilder, AccessToken, ClientId, ClientSecret,
    RefreshToken, Scope, TwitchToken, UserToken,
};
use url::Url;

#[derive(Deserialize, Serialize)]
pub struct StoredToken {
    access_token: String,
    refresh_token: String,
}

impl StoredToken {
    fn write(&self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn read(path: &PathBuf) -> Result<StoredToken, Box<dyn Error>> {
        let data = fs::read_to_string(path)?;
        let stored: StoredToken = serde_json::from_str(&data)?;
        Ok(stored)
    }

    fn token_path() -> Result<PathBuf, Box<dyn Error>> {
        let mut path =
            dirs::config_dir().ok_or("Could not find config directory.")?;
        path.push("twitch-soundbot");
        path.push("token.json");
        Ok(path)
    }

    async fn create_twitch_token() -> Result<UserToken, Box<dyn Error>> {
        let client_id = ClientId::new(env::var("CLIENT_ID").unwrap());
        let client_secret =
            ClientSecret::new(env::var("CLIENT_SECRET").unwrap());
        let redirect = Url::parse(&env::var("REDIRECT_URI").unwrap())?;

        println!("This is the redirect url we have generated: {redirect}\n\n");

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let mut builder =
            UserTokenBuilder::new(client_id, client_secret, redirect)
                .set_scopes(vec![Scope::ChannelReadRedemptions])
                .force_verify(true);

        let (url, _csrf) = builder.generate_url();
        println!("Open this URL in your browser and login:\n\n{}", url);

        println!(
            "\nAfter logging in, paste the full URL you were redirected to:"
        );
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let u = Url::parse(input.trim())?;

        let params: HashMap<_, _> = u.query_pairs().into_owned().collect();
        let code = params.get("code").ok_or("Missing code")?;
        let state = params.get("state").ok_or("Missing state")?;

        let user_token = builder.get_user_token(&client, state, code).await?;

        let token = StoredToken {
            access_token: user_token.token().secret().to_string(),
            refresh_token: user_token
                .clone()
                .refresh_token
                .unwrap()
                .secret()
                .to_string(),
        };

        token.write(&StoredToken::token_path()?)?;

        Ok(user_token)
    }

    async fn check_twitch_token(self) -> Result<UserToken, Box<dyn Error>> {
        let client_id = ClientId::new(env::var("CLIENT_ID").unwrap());
        let client_secret =
            ClientSecret::new(env::var("CLIENT_SECRET").unwrap());

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let access = AccessToken::new(self.access_token);
        let refresh = RefreshToken::new(self.refresh_token);

        let user_token = UserToken::from_existing_or_refresh_token(
            &client,
            access,
            refresh,
            client_id,
            Some(client_secret),
        )
        .await?;

        Ok(user_token)
    }

    pub async fn ensure_twitch_token() -> Result<UserToken, Box<dyn Error>> {
        let token: UserToken =
            match StoredToken::read(&StoredToken::token_path()?) {
                Ok(token) => token.check_twitch_token().await.unwrap(),
                Err(_) => StoredToken::create_twitch_token().await.unwrap(),
            };
        Ok(token)
    }
}
