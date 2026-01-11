# uestc-client

A minimal reqwest client for UESTC (University of Electronic Science and Technology of China).

这是一个电子科技大学（UESTC）的最小化 reqwest 客户端封装，用于模拟登录 UESTC 统一身份认证系统，并维持会话以便进行后续的 HTTP 请求。

## Features

- **Async & Blocking**: Supports both asynchronous (tokio) and blocking APIs.
- **Login/Logout**: Handles the login flow (including password encryption) and logout.
- **Automatic Cookie Persistence**: Transparently saves and loads cookies, just like a browser.
- **Session Management**: Automatically checks if session is active before logging in.
- **Reqwest Wrapper**: Exposes `reqwest`'s request builder for full flexibility.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
uestc-client = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

To use the blocking client, enable the `blocking` feature:

```toml
[dependencies]
uestc-client = { version = "0.1.0", features = ["blocking"] }
```

## Usage

### Async Client (Default)

```rust
use uestc_client::UestcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new client with automatic cookie persistence
    let client = UestcClient::new();

    // Login - cookies are automatically saved and reused
    // If valid cookies exist, login is skipped automatically
    client.login("your_student_id", "your_password").await?;

    // Now you can make authenticated requests
    let resp = client.get("https://eportal.uestc.edu.cn/new/index.html")
        .send()
        .await?;

    println!("Response status: {}", resp.status());

    // Logout when done (clears cookies)
    client.logout().await?;

    Ok(())
}
```

You can also specify a custom cookie file path:

```rust
let client = UestcClient::with_cookie_file("my_cookies.json");
client.login("your_student_id", "your_password").await?;
```

### Blocking Client

Enable the `blocking` feature in your `Cargo.toml`.

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

    // Logout
    client.logout()?;

    Ok(())
}
```

## License

This project is licensed under the MIT License.
