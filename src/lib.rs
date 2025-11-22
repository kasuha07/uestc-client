mod core;
mod client;

#[cfg(feature = "async")]
pub use client::UestcClient;

#[cfg(feature = "blocking")]
pub use client::UestcBlockingClient as UestcClient;

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
}

pub type Result<T> = std::result::Result<T, UestcClientError>;
