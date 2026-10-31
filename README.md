# uestc-client

A minimal reqwest client for UESTC (University of Electronic Science and Technology of China).

这是一个电子科技大学（UESTC）的最小化 reqwest 客户端封装，用于模拟登录 UESTC 统一身份认证系统，并维持会话以便进行后续的 HTTP 请求。

## Features

- **Async & Blocking**: Supports both asynchronous (tokio) and blocking APIs.
- **Multiple Login Methods**:
  - Username/password login with automatic password encryption
  - WeChat QR code login via terminal
- **Encrypted Cookie Persistence**: Optionally saves and loads cookies encrypted at rest (AES-256-GCM).
- **Session Management**: Reuses an active session only when it belongs to the account being logged in.
- **Reqwest Wrapper**: Exposes `reqwest`'s request builder for full flexibility.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
uestc-client = "0.3.0"
tokio = { version = "1", features = ["full"] }
```

To use the blocking client, enable the `blocking` feature:

```toml
[dependencies]
uestc-client = { version = "0.2.1", features = ["blocking"] }
```

## Usage

### Async Client (Default)

#### Username/Password Login

```rust
use uestc_client::UestcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client whose cookies are persisted encrypted at rest
    let client = UestcClient::with_encrypted_cookie_file("uestc_cookies.json", "a-strong-secret");

    // Login - cookies are encrypted, saved and reused.
    // If the saved session belongs to this account, login is skipped automatically.
    client.login("your_student_id", "your_password").await?;

    // Now you can make authenticated requests
    let resp = client.get("https://eportal.uestc.edu.cn/new/index.html")
        .send()
        .await?;

    println!("Response status: {}", resp.status());

    // Logout when done: ends the server session and clears local session cookies
    // (only the trusted-device fingerprint cookie is kept).
    client.logout().await?;

    Ok(())
}
```

#### WeChat QR Code Login

```rust
use uestc_client::UestcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new client
    let client = UestcClient::new();

    // Login using WeChat QR code
    // A QR code will be displayed in the terminal for you to scan
    client.wechat_login().await?;

    // Session is now active
    let resp = client.get("https://eportal.uestc.edu.cn/new/index.html")
        .send()
        .await?;

    println!("Response status: {}", resp.status());

    Ok(())
}
```

#### Cookie persistence

- `UestcClient::new()` / `with_cookie_file(path)` keep cookies **in memory only**. An existing
  file at `path` is never read, written or deleted.
- `with_encrypted_cookie_file(path, secret)` persists cookies encrypted at rest. If the file
  cannot be decrypted, the client starts with an empty session and **leaves the file untouched**.
- `try_with_encrypted_cookie_file(path, secret)` does the same but returns an error instead,
  so you can decide how to recover.
- Plaintext cookie files written by uestc-client 0.3.0 and earlier are no longer loaded
  implicitly. Migrate them once, explicitly:

```rust
uestc_client::migrate_plaintext_cookie_file("uestc_cookies.json", "a-strong-secret")?;
let client = UestcClient::try_with_encrypted_cookie_file("uestc_cookies.json", "a-strong-secret")?;
```

#### Session ownership and logout

- `login(username, ..)` reuses a saved session only if it was created by a password login
  (or reauth) for the same `username`. Sessions of another account, or of unknown ownership
  (WeChat login, files saved by older versions), are discarded and a fresh login is performed.
- `logout()` ends the server session and clears all local session cookies, in memory and on
  disk, keeping only the `MULTIFACTOR_BROWSER_FINGERPRINT` trusted-device cookie so the next
  login is not asked for reauth again.
- `logout_and_forget_device()` also removes that fingerprint cookie.
- If the WeChat QR login lands on a multi-factor reauth page, `wechat_login()` returns
  `UestcClientError::ReauthRequired` instead of reporting success.

### Blocking Client

Enable the `blocking` feature in your `Cargo.toml`.

#### Username/Password Login

```rust
use uestc_client::UestcBlockingClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = UestcBlockingClient::new();

    // Login - cookies are automatically managed
    client.login("your_student_id", "your_password")?;

    // Authenticated request
    let resp = client.get("https://eportal.uestc.edu.cn/new/index.html")
        .send()?;

    println!("Response status: {}", resp.status());

    // Logout (clears local session cookies, keeps the trusted-device fingerprint)
    client.logout()?;

    Ok(())
}
```

#### WeChat QR Code Login

```rust
use uestc_client::UestcBlockingClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = UestcBlockingClient::new();

    // Login using WeChat QR code
    // A QR code will be displayed in the terminal for you to scan
    client.wechat_login()?;

    // Session is now active
    let resp = client.get("https://eportal.uestc.edu.cn/new/index.html")
        .send()?;

    println!("Response status: {}", resp.status());

    Ok(())
}
```

## Examples

The repository includes working examples in the `examples/` directory:

- `async_wechat_login.rs` - Async WeChat QR code login
- `blocking_wechat_login.rs` - Blocking WeChat QR code login

Run them with:

```bash
# Async WeChat login (default features)
cargo run --example async_wechat_login

# Blocking WeChat login
cargo run --no-default-features --features blocking --example blocking_wechat_login
```

## License

This project is licensed under the MIT License.
