use super::AUTH_SERVER_URL;
use crate::{Result, UestcClientError, core};
use reqwest::{Client, IntoUrl, Method, RequestBuilder};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use cookie_store::CookieStore;
use reqwest_cookie_store::CookieStoreMutex;

const DEFAULT_COOKIE_FILE: &str = "uestc_cookies.json";

pub struct UestcClient {
    client: Client,
    cookie_store: Arc<CookieStoreMutex>,
    cookie_file: PathBuf,
}

impl UestcClient {
    pub fn new() -> Self {
        Self::with_cookie_file(DEFAULT_COOKIE_FILE)
    }

    pub fn with_cookie_file<P: AsRef<Path>>(path: P) -> Self {
        let cookie_file = path.as_ref().to_path_buf();

        // Try to load existing cookies
        let cookie_store = if cookie_file.exists() {
            Self::load_cookie_store(&cookie_file).unwrap_or_else(|_| {
                Arc::new(CookieStoreMutex::new(CookieStore::default()))
            })
        } else {
            Arc::new(CookieStoreMutex::new(CookieStore::default()))
        };

        let client = Client::builder()
            .default_headers(super::default_headers())
            .cookie_provider(cookie_store.clone())
            .build()
            .expect("Failed to build client");

        Self {
            client,
            cookie_store,
            cookie_file,
        }
    }

    pub fn with_client(client: Client) -> Self {
        let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
        Self {
            client,
            cookie_store,
            cookie_file: PathBuf::from(DEFAULT_COOKIE_FILE),
        }
    }

    fn load_cookie_store(path: &Path) -> Result<Arc<CookieStoreMutex>> {
        let json = fs::read_to_string(path)
            .map_err(|e| UestcClientError::CookieError(format!("Failed to read cookie file: {}", e)))?;

        let store: CookieStore = serde_json::from_str(&json)
            .map_err(|e| UestcClientError::CookieError(format!("Failed to deserialize cookies: {}", e)))?;

        Ok(Arc::new(CookieStoreMutex::new(store)))
    }

    fn save_cookie_store(&self) -> Result<()> {
        let store = self.cookie_store.lock().unwrap();
        let json = serde_json::to_string_pretty(&*store)
            .map_err(|e| UestcClientError::CookieError(format!("Failed to serialize cookies: {}", e)))?;

        fs::write(&self.cookie_file, json)
            .map_err(|e| UestcClientError::CookieError(format!("Failed to write cookie file: {}", e)))?;

        Ok(())
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<()> {
        // Check if session is already active
        if self.is_session_active().await {
            return Ok(());
        }

        // Perform password login
        let login_url = format!("{}/login", AUTH_SERVER_URL);

        // Get login page without service parameter
        let resp = self.client.get(&login_url).send().await?;
        let html = resp.text().await?;

        // Parse login page
        let info = core::parser::parse_login_page(&html)?;

        // Encrypt password
        let encrypted_password = core::crypto::encrypt_password(password, &info.pwd_encrypt_salt)?;

        // Prepare form data
        let mut form_data = info.form_data;
        form_data
            .entry("username".to_string())
            .and_modify(|v| *v = username.to_string())
            .or_insert(username.to_string());
        form_data
            .entry("password".to_string())
            .and_modify(|v| *v = encrypted_password.to_string())
            .or_insert(encrypted_password.to_string());

        // Submit login form
        let resp = self.client.post(&login_url).form(&form_data).send().await?;

        // Check for redirect (302) or success status
        let status = resp.status();
        let final_url = resp.url().to_string();

        // Login is successful if we're not on the login page
        if status.is_redirection() || status.is_success() {
            if !final_url.contains("/authserver/login") {
                // Save cookies after successful login
                let _ = self.save_cookie_store();
                return Ok(());
            }
        }

        // If we're still on login page, extract error message
        let html = resp.text().await?;
        let error_msg = core::parser::extract_error_message(&html)
            .unwrap_or_else(|| format!("Login failed with status: {}", status));

        Err(UestcClientError::LoginFailed(error_msg))
    }

    pub async fn logout(&self) -> Result<()> {
        let logout_url = format!("{}/logout", AUTH_SERVER_URL);
        let resp = self.client.get(&logout_url).send().await?;

        if resp.status().is_success() {
            // Clear cookies after logout
            let _ = fs::remove_file(&self.cookie_file);
            return Ok(());
        }

        Err(UestcClientError::LogoutFailed(format!(
            "Error code: {}",
            resp.status()
        )))
    }

    /// Check if the current session is still active
    /// Returns true if logged in, false otherwise
    pub async fn is_session_active(&self) -> bool {
        let login_url = format!("{}/login", AUTH_SERVER_URL);
        let expected_redirect = "https://idas.uestc.edu.cn/personalInfo/personCenter/index.html";

        match self.client.get(&login_url).send().await {
            Ok(resp) => {
                let final_url = resp.url().to_string();
                // If we're redirected to personal center, session is active
                if final_url == expected_redirect {
                    // Save cookies when session is confirmed active
                    let _ = self.save_cookie_store();
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        self.client.request(method, url)
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::GET, url)
    }

    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::POST, url)
    }

    pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PUT, url)
    }

    pub fn patch<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PATCH, url)
    }

    pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }

    pub fn head<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::HEAD, url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new() {
        let _client = UestcClient::new();
        assert!(true);
    }

    #[tokio::test]
    async fn test_with_client() {
        use reqwest::Client;
        let req_client = Client::new();
        let _client = UestcClient::with_client(req_client);
        assert!(true);
    }

    #[tokio::test]
    async fn test_login_failed() {
        let client = UestcClient::new();
        let result = client.login("1234567890", "password123").await;
        println!("result: {:?}", result);
        assert!(result.is_err());
    }
}
