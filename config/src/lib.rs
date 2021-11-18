#[derive(Debug, Clone)]
pub struct Config<'r> {
    agent_id: Option<Id<'r>>,
    websocket_address: &'r str,
    server_address: &'r str,
    file_store_location: &'r str,
    max_upload_attempts: usize,
    size_limit: usize,
    default_share_time_hours: usize,
    reconnect_delay_minutes: usize,
}

/// Information required to connect to central api
#[derive(Debug, Clone)]
pub struct Id<'r> {
    public_id: &'r str,
    private_key: &'r str,
}

impl<'r> Config<'r> {
    pub fn load_config() -> Config<'r> {
        //TODO load from disk
        //TODO validate that the file_store_location exists, if it doesn't we should create it.
        Config {
            agent_id: None,
            websocket_address: "ws://localhost:8000/api/v1",
            server_address: "http://localhost:8000/api/v1",
            file_store_location: "/home/josiah/.riptide",
            max_upload_attempts: 10,
            size_limit: 2147483648,
            default_share_time_hours: 48,
            reconnect_delay_minutes: 15,
        }
    }

    pub fn public_id(&self) -> Option<&'r str> {
        if let Some(id) = &self.agent_id {
            Some(id.public_id)
        } else {
            None
        }
    }

    pub fn private_id(&self) -> Option<&'r str> {
        if let Some(id) = &self.agent_id {
            Some(id.private_key)
        } else {
            None
        }
    }

    //XXX generate these getters using a crate https://docs.rs/getset/0.1.1/getset/index.html
    pub fn websocket_address(&self) -> &'r str {
        self.websocket_address
    }

    pub fn server_address(&self) -> &'r str {
        self.server_address
    }

    pub fn file_store_location(&self) -> &'r str {
        self.file_store_location
    }

    pub fn max_upload_attempts(&self) -> usize {
        self.max_upload_attempts
    }

    pub fn size_limit(&self) -> usize {
        self.size_limit
    }

    pub fn default_share_time_hours(&self) -> usize {
        self.default_share_time_hours
    }

    pub fn reconnect_delay_minutes(&self) -> usize {
        self.reconnect_delay_minutes
    }
}