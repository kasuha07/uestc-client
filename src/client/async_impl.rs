use crate::Result;
use reqwest::Client;

pub struct UestcClient {
    client: Client,
}

impl UestcClient {
    pub fn new() -> Self {
        unimplemented!("Not implemented yet")
    }

    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    pub async fn login(&self, username: &str, password: &str, service_url: &str) -> Result<()> {
        unimplemented!("Not implemented yet")
    }
}

impl Default for UestcClient {
    fn default() -> Self {
        Self::new()
    }
}

