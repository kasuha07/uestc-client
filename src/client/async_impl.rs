use crate::{Result, UestcClientError, core};
use reqwest::{Client, header};

pub struct UestcClient {
    client: Client,
}

const AUTH_SERVER_URL: &str = "https://idas.uestc.edu.cn/authserver";
const DEFAULT_SERVICE_URL: &str = "https://eportal.uestc.edu.cn/new/index.html?browser=no";

impl UestcClient {
    pub fn new() -> Self {
        // global headers
        let mut headers = header::HeaderMap::new();
        // common headers
        headers.insert(header::ACCEPT, header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));
        headers.insert(
            header::ACCEPT_LANGUAGE,
            header::HeaderValue::from_static("zh-CN,zh;q=0.9"),
        );
        headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-cache"),
        );
        headers.insert(
            header::UPGRADE_INSECURE_REQUESTS,
            header::HeaderValue::from_static("1"),
        );
        headers.insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
        headers.insert(header::DNT, header::HeaderValue::from_static("1"));
        headers.insert(header::USER_AGENT, header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36"));

        // Sec-Fetch headers
        headers.insert(
            "Sec-Fetch-Dest",
            header::HeaderValue::from_static("document"),
        );
        headers.insert(
            "Sec-Fetch-Mode",
            header::HeaderValue::from_static("navigate"),
        );
        headers.insert("Sec-Fetch-Site", header::HeaderValue::from_static("none"));
        headers.insert("Sec-Fetch-User", header::HeaderValue::from_static("?1"));

        // Sec-Ch-Ua headers
        headers.insert(
            "Sec-Ch-Ua",
            header::HeaderValue::from_static(r#""Not_A Brand";v="99", "Chromium";v="142""#),
        );
        headers.insert("Sec-Ch-Ua-Mobile", header::HeaderValue::from_static("?0"));
        headers.insert(
            "Sec-Ch-Ua-Platform",
            header::HeaderValue::from_static(r#""Windows""#),
        );

        // build client
        let client = Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .build()
            .expect("Failed to build client");

        Self { client }
    }

    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
        service_url: impl Into<Option<&str>>,
    ) -> Result<()> {
        let login_url = format!("{}/login", AUTH_SERVER_URL);
        let service_url = service_url.into().unwrap_or(DEFAULT_SERVICE_URL);

        // Get login page
        let resp = self
            .client
            .get(&login_url)
            .query(&[("service", service_url)])
            .send()
            .await?;
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
        let resp = self
            .client
            .post(&login_url)
            .query(&[("service", service_url)])
            .form(&form_data)
            .send()
            .await?;

        // Verify login
        if resp.status().is_success() {
            return Ok(());
        }

        Err(UestcClientError::LoginFailed(format!(
            "Error code: {}",
            resp.status()
        )))
    }

    pub async fn logout(&self) -> Result<()> {
        let logout_url = format!("{}/logout", AUTH_SERVER_URL);
        let resp = self
            .client
            .get(&logout_url)
            .query(&[("service", DEFAULT_SERVICE_URL)])
            .send()
            .await?;

        if resp.status().is_success() {
            return Ok(());
        }

        Err(UestcClientError::LogoutFailed(format!(
            "Error code: {}",
            resp.status()
        )))
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
        let result = client
            .login(
                "1234567890",
                "password123",
                "https://eportal.uestc.edu.cn/new/index.html?browser=no",
            )
            .await;
        println!("result: {:?}", result);
        assert!(result.is_err());
    }
}
