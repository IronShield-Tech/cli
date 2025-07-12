use reqwest::{Client, header::HeaderMap};

use ironshield_api::handler::{error, error::ErrorHandler, result::ResultHandler};

use crate::config::ClientConfig;

const USER_AGENT: &str = "curl/8.4.0";

pub struct IronShieldClient {
    config:      ClientConfig,
    http_client: Client,
}

impl IronShieldClient {
    pub fn new(config: ClientConfig) -> ResultHandler<Self> {
        if config.endpoint.is_empty() {
            return Err(ErrorHandler::config_error("Endpoint cannot be empty."));
        }

        if !config.endpoint.starts_with("https://") {
            return Err(ErrorHandler::config_error(error::INVALID_ENDPOINT));
        }

        let http_client = Client::builder()
            .timeout(config.timeout)
            .user_agent(USER_AGENT) // I picked random numbers
            .danger_accept_invalid_certs(false)
            .build()
            .map_err(ErrorHandler::from_network_error)?;

        Ok(Self {
            config,
            http_client
        })
    }
}