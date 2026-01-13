use super::AUTH_SERVER_URL;
use crate::{Result, UestcClientError, core};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::{IntoUrl, Method};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use cookie_store::CookieStore;
use reqwest_cookie_store::CookieStoreMutex;
use serde::{Deserialize, Serialize};

const DEFAULT_COOKIE_FILE: &str = "uestc_cookies.json";

#[derive(Serialize, Deserialize, Debug)]
struct SerializableCookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    expires: Option<i64>,
    secure: bool,
    http_only: bool,
}

pub struct UestcBlockingClient {
    client: Client,
    cookie_store: Arc<CookieStoreMutex>,
    cookie_file: PathBuf,
}

impl UestcBlockingClient {
    pub fn new() -> Self {
        Self::with_cookie_file(DEFAULT_COOKIE_FILE)
    }

    pub fn with_cookie_file<P: AsRef<Path>>(path: P) -> Self {
        let cookie_file = path.as_ref().to_path_buf();

        // Try to load existing cookies
        let cookie_store = if cookie_file.exists() {
            log::debug!("发现 cookie 文件: {:?}", cookie_file);
            match Self::load_cookie_store(&cookie_file) {
                Ok(store) => {
                    let count = store.lock().unwrap().iter_any().count();
                    log::debug!("成功加载 {} 个 cookies", count);
                    store
                }
                Err(e) => {
                    log::warn!("加载 cookie 失败: {}", e);
                    Arc::new(CookieStoreMutex::new(CookieStore::default()))
                }
            }
        } else {
            log::debug!("cookie 文件不存在: {:?}", cookie_file);
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
        let json = fs::read_to_string(path).map_err(|e| UestcClientError::CookieError {
            operation: "read".to_string(),
            file_path: Some(path.display().to_string()),
            message: format!("Failed to read cookie file: {}", e),
            source: Some(Box::new(e)),
        })?;

        let cookies: Vec<SerializableCookie> =
            serde_json::from_str(&json).map_err(|e| UestcClientError::CookieError {
                operation: "deserialize".to_string(),
                file_path: Some(path.display().to_string()),
                message: format!("Failed to deserialize cookies: {}", e),
                source: Some(Box::new(e)),
            })?;

        let mut store = CookieStore::default();

        // Convert SerializableCookie back to cookie_store format
        for sc in cookies {
            // Skip cookies with empty domain
            if sc.domain.is_empty() {
                log::debug!("跳过空 domain 的 cookie: {}", sc.name);
                continue;
            }

            let mut cookie_str = format!("{}={}", sc.name, sc.value);
            cookie_str.push_str(&format!("; Domain={}", sc.domain));
            cookie_str.push_str(&format!("; Path={}", sc.path));

            if sc.secure {
                cookie_str.push_str("; Secure");
            }
            if sc.http_only {
                cookie_str.push_str("; HttpOnly");
            }
            if let Some(expires) = sc.expires {
                cookie_str.push_str(&format!("; Max-Age={}", expires));
            }

            // Parse and insert the cookie
            if let Ok(cookie) = cookie_str.parse::<cookie_store::RawCookie>() {
                if let Ok(url) = url::Url::parse(&format!("https://{}", sc.domain)) {
                    if let Err(e) = store.insert_raw(&cookie, &url) {
                        log::debug!("插入 cookie 失败: {:?}", e);
                    }
                } else {
                    log::debug!("无法解析 domain: {}", sc.domain);
                }
            }
        }

        Ok(Arc::new(CookieStoreMutex::new(store)))
    }

    fn save_cookie_store(&self) -> Result<()> {
        let store = self.cookie_store.lock().unwrap();

        // Convert cookies to SerializableCookie format
        let cookies: Vec<SerializableCookie> = store
            .iter_any()
            .map(|c| {
                // Use a default domain if the cookie doesn't have one
                let domain = c.domain()
                    .filter(|d| !d.is_empty())
                    .unwrap_or("idas.uestc.edu.cn");

                SerializableCookie {
                    name: c.name().to_string(),
                    value: c.value().to_string(),
                    domain: domain.to_string(),
                    path: c.path().unwrap_or("/").to_string(),
                    expires: None, // Treat all as session cookies for simplicity
                    secure: c.secure().unwrap_or(false),
                    http_only: c.http_only().unwrap_or(false),
                }
            })
            .collect();

        let count = cookies.len();
        log::debug!("保存 {} 个 cookies 到: {:?}", count, self.cookie_file);

        let json = serde_json::to_string_pretty(&cookies).map_err(|e| {
            UestcClientError::CookieError {
                operation: "serialize".to_string(),
                file_path: Some(self.cookie_file.display().to_string()),
                message: format!("Failed to serialize cookies: {}", e),
                source: Some(Box::new(e)),
            }
        })?;

        fs::write(&self.cookie_file, json).map_err(|e| UestcClientError::CookieError {
            operation: "write".to_string(),
            file_path: Some(self.cookie_file.display().to_string()),
            message: format!("Failed to write cookie file: {}", e),
            source: Some(Box::new(e)),
        })?;

        log::debug!("cookies 已成功保存");
        Ok(())
    }

    pub fn login(&self, username: &str, password: &str) -> Result<()> {
        log::info!("Starting login for user: {}", username);

        // Check if session is already active
        if self.is_session_active() {
            log::info!("Session already active, skipping login");
            return Ok(());
        }

        // Perform password login
        let login_url = format!("{}/login", AUTH_SERVER_URL);

        log::debug!("Fetching login page");
        // Get login page without service parameter
        let resp = self.client.get(&login_url).send()?;
        let html = resp.text()?;

        log::debug!("Parsing login page");
        // Parse login page
        let info = core::parser::parse_login_page(&html)?;

        log::debug!("Encrypting password");
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

        log::debug!("Submitting login form");
        // Submit login form
        let resp = self.client.post(&login_url).form(&form_data).send()?;

        // Check for redirect (302) or success status
        let status = resp.status();
        let final_url = resp.url().to_string();

        log::debug!("Login response status: {}, URL: {}", status, final_url);

        // Login is successful if we're not on the login page
        if status.is_redirection() || status.is_success() {
            if !final_url.contains("/authserver/login") {
                log::info!("Login successful for user: {}", username);
                // Save cookies after successful login
                if let Err(e) = self.save_cookie_store() {
                    log::warn!("Failed to save cookies after login: {}", e);
                }
                return Ok(());
            }
        }

        // If we're still on login page, extract error message
        let html = resp.text()?;
        let error_msg = core::parser::extract_error_message(&html)
            .unwrap_or_else(|| format!("Login failed with status: {}", status));

        log::error!("Login failed for user {}: {}", username, error_msg);

        Err(UestcClientError::LoginFailed {
            message: error_msg,
            username: Some(username.to_string()),
        })
    }

    pub fn logout(&self) -> Result<()> {
        log::info!("Attempting to logout");

        let logout_url = format!("{}/logout", AUTH_SERVER_URL);
        let resp = self.client.get(&logout_url).send()?;

        if resp.status().is_success() {
            log::info!("Logout successful");
            // Clear cookies after logout
            if let Err(e) = fs::remove_file(&self.cookie_file) {
                log::warn!(
                    "Failed to delete cookie file after logout: {}",
                    e
                );
            }
            return Ok(());
        }

        let error_msg = format!("Logout failed with status: {}", resp.status());
        log::error!("{}", error_msg);

        Err(UestcClientError::LogoutFailed {
            message: error_msg,
        })
    }

    /// Login using WeChat QR code
    /// This will display a QR code in the terminal for scanning
    pub fn wechat_login(&self) -> Result<()> {
        use crate::core::wechat;

        // Check if session is already active
        log::debug!("检查已存储的会话");
        if self.is_session_active() {
            log::info!("已经登录，无需重新登录");
            return Ok(());
        }
        log::debug!("未检测到有效会话，开始微信登录流程");

        log::debug!("正在连接 CAS 初始化参数");

        // Step 1: Get WeChat OAuth parameters
        let cas_login_url = format!("{}/combinedLogin.do?type=weixin", AUTH_SERVER_URL);
        let resp = self.client.get(&cas_login_url).send()?;

        // Extract WeChat OAuth parameters from the final URL
        let wechat_auth_url = resp.url().to_string();

        // Verify we got redirected to WeChat OAuth page
        if !wechat_auth_url.contains("open.weixin.qq.com") {
            return Err(UestcClientError::WeChatError {
                message: format!(
                    "Failed to redirect to WeChat login page, current URL: {}",
                    wechat_auth_url
                ),
            });
        }

        let params = wechat::WechatAuthParams::from_url(&wechat_auth_url)?;
        log::debug!("Target AppID: {}", params.appid);

        // Step 2: Get QR code UUID
        log::debug!("正在获取二维码 UUID");
        let xml_url = params.build_qr_xml_url();
        let resp = self.client.get(&xml_url).send()?;
        let xml_text = resp.text()?;
        let uuid = wechat::parse_qr_uuid_from_xml(&xml_text)?;

        // Step 3: Display QR code in terminal
        wechat::display_qr_in_terminal(&uuid)?;

        // Step 4: Poll for scan status
        log::debug!("等待扫码");
        let mut last_code: Option<String> = None;
        let wx_code = loop {
            let poll_url = wechat::build_poll_url(&uuid, last_code.as_deref());
            let resp = self.client
                .get(&poll_url)
                .timeout(std::time::Duration::from_secs(30))
                .send()?;
            let text = resp.text()?;
            let result = wechat::parse_scan_status(&text)?;

            match result.status {
                wechat::ScanStatus::Confirmed => {
                    log::debug!("登录成功 (405)");
                    if let Some(code) = result.wx_code {
                        log::debug!("获取到 wx_code");
                        break code;
                    } else {
                        return Err(UestcClientError::WeChatError {
                            message: "Received 405 status but wx_code not found".to_string(),
                        });
                    }
                }
                wechat::ScanStatus::Scanned => {
                    log::info!("已扫码，请在手机上点击确认");
                    last_code = Some("404".to_string());
                }
                wechat::ScanStatus::Expired => {
                    return Err(UestcClientError::WeChatError {
                        message: "QR code expired, please run again".to_string(),
                    });
                }
                wechat::ScanStatus::Waiting => {
                    // Keep waiting silently
                }
                wechat::ScanStatus::Unknown(code) => {
                    log::warn!("未知状态码: {}", code);
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(500));
        };

        // Step 5: Complete login
        log::debug!("正在验证登录");
        let callback_url = params.build_callback_url(&wx_code);
        let resp = self.client.get(&callback_url).send()?;
        let final_url = resp.url().to_string();

        // Consume the response body to ensure cookies are properly captured
        let _ = resp.bytes()?;

        // Check if login succeeded by examining the final URL
        if !final_url.contains("/authserver/login") {
            // Save cookies after successful login
            if let Err(e) = self.save_cookie_store() {
                log::warn!("Failed to save cookies after WeChat login: {}", e);
            }
            log::info!("微信登录成功");
            Ok(())
        } else {
            Err(UestcClientError::WeChatError {
                message: "WeChat login failed, still on login page".to_string(),
            })
        }
    }

    /// Check if the current session is still active
    /// Returns true if logged in, false otherwise
    pub fn is_session_active(&self) -> bool {
        let login_url = format!("{}/login", AUTH_SERVER_URL);
        let expected_redirect = "https://idas.uestc.edu.cn/personalInfo/personCenter/index.html";

        log::debug!("Checking session status");

        match self.client.get(&login_url).send() {
            Ok(resp) => {
                let final_url = resp.url().to_string();
                // If we're redirected to personal center, session is active
                if final_url == expected_redirect {
                    log::debug!("Session is active");
                    // Save cookies when session is confirmed active
                    if let Err(e) = self.save_cookie_store() {
                        log::warn!("Failed to save cookies during session check: {}", e);
                    }
                    true
                } else {
                    log::debug!("Session is not active (URL: {})", final_url);
                    false
                }
            }
            Err(e) => {
                log::debug!("Session check failed: {}", e);
                false
            }
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
        let result = client.login("1234567890", "password123");
        assert!(result.is_err());
    }
}
