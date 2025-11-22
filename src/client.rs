#[cfg(feature = "async")]
pub mod async_impl;

#[cfg(feature = "blocking")]
pub mod blocking_impl;

#[cfg(feature = "async")]
pub use async_impl::UestcClient;

#[cfg(feature = "blocking")]
pub use blocking_impl::UestcBlockingClient;
