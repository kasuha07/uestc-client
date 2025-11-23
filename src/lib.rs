mod client;
mod core;

#[cfg(feature = "async")]
pub use client::UestcClient;

#[cfg(feature = "blocking")]
pub use client::UestcBlockingClient;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum UestcClientError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Crypto error: {0}")]
    CryptoError(String),

    #[error("Login failed: {0}")]
    LoginFailed(String),

    #[error("Logout failed: {0}")]
    LogoutFailed(String),
}

pub type Result<T> = std::result::Result<T, UestcClientError>;
