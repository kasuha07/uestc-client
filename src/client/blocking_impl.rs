use crate::Result;
use reqwest::blocking::Client;

pub struct UestcBlockingClient {
    client: Client,
}

impl UestcBlockingClient {
    pub fn new() -> Self {
        unimplemented!("Not implemented yet")
    }

    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    pub fn login(&self, username: &str, password: &str, service_url: &str) -> Result<()> {
        unimplemented!("Not implemented yet")
    }
}

impl Default for UestcBlockingClient {
    fn default() -> Self {
        Self::new()
    }
}

