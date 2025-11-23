use super::{AUTH_SERVER_URL, DEFAULT_SERVICE_URL};
use crate::{Result, UestcClientError, core};
use reqwest::blocking::Client;

pub struct UestcBlockingClient {
    client: Client,
}

impl UestcBlockingClient {
    pub fn new() -> Self {
        // build client
        let client = Client::builder()
            .default_headers(super::default_headers())
            .cookie_store(true)
            .build()
            .expect("Failed to build client");

        Self { client }
    }

    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    pub fn login<'a>(
        &self,
        username: &str,
        password: &str,
        service_url: impl Into<Option<&'a str>>,
    ) -> Result<()> {
        let login_url = format!("{}/login", AUTH_SERVER_URL);
        let service_url = service_url.into().unwrap_or(DEFAULT_SERVICE_URL);

        // Get login page
        let resp = self
            .client
            .get(&login_url)
            .query(&[("service", service_url)])
            .send()?;
        let html = resp.text()?;

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
        let resp = self
            .client
            .post(&login_url)
            .query(&[("service", service_url)])
            .form(&form_data)
            .send()?;

        // Verify login
        if resp.status().is_success() {
            return Ok(());
        }

        Err(UestcClientError::LoginFailed(format!(
            "Error code: {}",
            resp.status()
        )))
    }

    pub fn logout(&self) -> Result<()> {
        let logout_url = format!("{}/logout", AUTH_SERVER_URL);
        let resp = self
            .client
            .get(&logout_url)
            .query(&[("service", DEFAULT_SERVICE_URL)])
            .send()?;

        if resp.status().is_success() {
            return Ok(());
        }

        Err(UestcClientError::LogoutFailed(format!(
            "Error code: {}",
            resp.status()
        )))
    }
}

impl Default for UestcBlockingClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let _client = UestcBlockingClient::new();
        assert!(true);
    }

    #[test]
    fn test_with_client() {
        use reqwest::blocking::Client;
        let req_client = Client::new();
        let _client = UestcBlockingClient::with_client(req_client);
        assert!(true);
    }

    #[test]
    fn test_login_failed() {
        let client = UestcBlockingClient::new();
        let result = client.login(
            "1234567890",
            "password123",
            "https://eportal.uestc.edu.cn/new/index.html?browser=no",
        );
        println!("result: {:?}", result);
        assert!(result.is_err());
    }
}
