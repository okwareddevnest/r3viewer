use anyhow::{Result, anyhow};
use keyring::Entry;
use oauth2::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenResponse, AuthUrl, TokenUrl, basic::BasicClient,
    reqwest::async_http_client, StandardTokenResponse, EmptyExtraTokenFields,
    RefreshToken, AccessToken,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://www.googleapis.com/oauth2/v4/token";
const GOOGLE_CLIENT_ID: &str = "your-google-client-id"; // This should be configurable
const GOOGLE_REDIRECT_URI: &str = "http://localhost:8080/auth/callback";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCredentials {
    pub google_access_token: Option<String>,
    pub google_refresh_token: Option<String>,
    pub github_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub google_authenticated: bool,
    pub github_authenticated: bool,
    pub user_email: Option<String>,
    pub github_username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleAuthUrl {
    pub auth_url: String,
    pub csrf_token: String,
    pub pkce_verifier: String,
}

pub struct AuthService {
    keyring_service: String,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            keyring_service: "r3viewer".to_string(),
        }
    }

    // Google OAuth2 Flow
    pub fn generate_google_auth_url(&self) -> Result<GoogleAuthUrl> {
        let client = self.create_google_oauth_client()?;
        
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        
        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://www.googleapis.com/auth/spreadsheets.readonly".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        Ok(GoogleAuthUrl {
            auth_url: auth_url.to_string(),
            csrf_token: csrf_token.secret().clone(),
            pkce_verifier: pkce_verifier.secret().clone(),
        })
    }

    pub async fn exchange_google_code(
        &self,
        code: String,
        csrf_token: String,
        pkce_verifier: String,
    ) -> Result<()> {
        let client = self.create_google_oauth_client()?;
        
        let token_result = client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(oauth2::PkceCodeVerifier::new(pkce_verifier))
            .request_async(async_http_client)
            .await
            .map_err(|e| anyhow!("Failed to exchange authorization code: {}", e))?;

        // Store tokens securely
        self.store_google_tokens(
            token_result.access_token().secret(),
            token_result.refresh_token().map(|t| t.secret()),
        )?;

        Ok(())
    }

    // GitHub Token Management
    pub async fn validate_github_token(&self, token: &str) -> Result<String> {
        let client = reqwest::Client::new();
        
        let response = client
            .get("https://api.github.com/user")
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", "r3viewer")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Invalid GitHub token"));
        }

        let user_info: serde_json::Value = response.json().await?;
        let username = user_info["login"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to get username from GitHub"))?
            .to_string();

        // Store GitHub token securely
        self.store_github_token(token)?;

        Ok(username)
    }

    // Credential Storage (OS Keychain)
    fn store_google_tokens(&self, access_token: &str, refresh_token: Option<&str>) -> Result<()> {
        let access_entry = Entry::new(&self.keyring_service, "google_access_token")?;
        access_entry.set_password(access_token)?;

        if let Some(refresh_token) = refresh_token {
            let refresh_entry = Entry::new(&self.keyring_service, "google_refresh_token")?;
            refresh_entry.set_password(refresh_token)?;
        }

        Ok(())
    }

    fn store_github_token(&self, token: &str) -> Result<()> {
        let entry = Entry::new(&self.keyring_service, "github_token")?;
        entry.set_password(token)?;
        Ok(())
    }

    pub fn get_stored_credentials(&self) -> Result<AuthCredentials> {
        let google_access_token = self.get_credential("google_access_token").ok();
        let google_refresh_token = self.get_credential("google_refresh_token").ok();
        let github_token = self.get_credential("github_token").ok();

        Ok(AuthCredentials {
            google_access_token,
            google_refresh_token,
            github_token,
        })
    }

    fn get_credential(&self, key: &str) -> Result<String> {
        let entry = Entry::new(&self.keyring_service, key)?;
        entry.get_password().map_err(|e| anyhow!("Failed to get credential: {}", e))
    }

    pub async fn refresh_google_token(&self) -> Result<String> {
        let credentials = self.get_stored_credentials()?;
        
        let refresh_token = credentials.google_refresh_token
            .ok_or_else(|| anyhow!("No refresh token available"))?;

        let client = self.create_google_oauth_client()?;
        
        let token_result = client
            .exchange_refresh_token(&RefreshToken::new(refresh_token))
            .request_async(async_http_client)
            .await
            .map_err(|e| anyhow!("Failed to refresh token: {}", e))?;

        let new_access_token = token_result.access_token().secret().clone();
        
        // Store new access token
        self.store_google_tokens(&new_access_token, None)?;
        
        Ok(new_access_token)
    }

    pub async fn get_auth_status(&self) -> Result<AuthStatus> {
        let credentials = self.get_stored_credentials()?;
        
        let google_authenticated = credentials.google_access_token.is_some();
        let github_authenticated = credentials.github_token.is_some();
        
        // Get user email from Google if authenticated
        let user_email = if google_authenticated {
            self.get_google_user_email().await.ok()
        } else {
            None
        };

        // Get GitHub username if authenticated
        let github_username = if github_authenticated {
            self.get_github_username().await.ok()
        } else {
            None
        };

        Ok(AuthStatus {
            google_authenticated,
            github_authenticated,
            user_email,
            github_username,
        })
    }

    async fn get_google_user_email(&self) -> Result<String> {
        let credentials = self.get_stored_credentials()?;
        let access_token = credentials.google_access_token
            .ok_or_else(|| anyhow!("No Google access token"))?;

        let client = reqwest::Client::new();
        let response = client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get user info"));
        }

        let user_info: serde_json::Value = response.json().await?;
        let email = user_info["email"]
            .as_str()
            .ok_or_else(|| anyhow!("No email in user info"))?
            .to_string();

        Ok(email)
    }

    async fn get_github_username(&self) -> Result<String> {
        let credentials = self.get_stored_credentials()?;
        let token = credentials.github_token
            .ok_or_else(|| anyhow!("No GitHub token"))?;

        let client = reqwest::Client::new();
        let response = client
            .get("https://api.github.com/user")
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", "r3viewer")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get GitHub user info"));
        }

        let user_info: serde_json::Value = response.json().await?;
        let username = user_info["login"]
            .as_str()
            .ok_or_else(|| anyhow!("No login in user info"))?
            .to_string();

        Ok(username)
    }

    pub fn logout(&self) -> Result<()> {
        // Remove all stored credentials
        let _ = Entry::new(&self.keyring_service, "google_access_token")?.delete_password();
        let _ = Entry::new(&self.keyring_service, "google_refresh_token")?.delete_password();
        let _ = Entry::new(&self.keyring_service, "github_token")?.delete_password();
        Ok(())
    }

    fn create_google_oauth_client(&self) -> Result<BasicClient> {
        let client_id = ClientId::new(GOOGLE_CLIENT_ID.to_string());
        let client_secret = ClientSecret::new("".to_string()); // PKCE doesn't require client secret
        let auth_url = AuthUrl::new(GOOGLE_AUTH_URL.to_string())
            .map_err(|e| anyhow!("Invalid auth URL: {}", e))?;
        let token_url = TokenUrl::new(GOOGLE_TOKEN_URL.to_string())
            .map_err(|e| anyhow!("Invalid token URL: {}", e))?;
        let redirect_url = RedirectUrl::new(GOOGLE_REDIRECT_URI.to_string())
            .map_err(|e| anyhow!("Invalid redirect URL: {}", e))?;

        Ok(BasicClient::new(
            client_id,
            Some(client_secret),
            auth_url,
            Some(token_url),
        ).set_redirect_uri(redirect_url))
    }
} 