use tokio::task::JoinError;

#[derive(Debug)]
pub enum AgentError {
    ReadFile(std::io::Error),
    Http(reqwest::Error),
    JoinError(JoinError),
    TokioError(tokio_tungstenite::tungstenite::Error),
    FrameworkError(ws_com_framework::Error),
    BadFrame(String),
    Other(Box<dyn std::error::Error + 'static + Send + Sync>),
}

impl From<tokio_tungstenite::tungstenite::Error> for AgentError {
    fn from(t: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::TokioError(t)
    }
}

impl From<std::io::Error> for AgentError {
    fn from(e: std::io::Error) -> AgentError {
        AgentError::ReadFile(e)
    }
}

impl From<reqwest::Error> for AgentError {
    fn from(e: reqwest::Error) -> AgentError {
        AgentError::Http(e)
    }
}

impl From<ws_com_framework::Error> for AgentError {
    fn from(e: ws_com_framework::Error) -> Self {
        AgentError::FrameworkError(e)
    }
}

impl From<JoinError> for AgentError {
    fn from(j: JoinError) -> Self {
        Self::JoinError(j)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for AgentError {
    fn from(e: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self::Other(e)
    }
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::ReadFile(e) => write!(f, "Unable to read file: {}", e),
            AgentError::Http(e) => write!(f, "Unable to establish http connection: {}", e),
            AgentError::JoinError(e) => {
                write!(f, "Unable to join process, should never happen: {}", e)
            }
            AgentError::Other(e) => write!(f, "Unknown: {}", e),
            AgentError::TokioError(e) => write!(f, "tokio error occured: {}", e),
            AgentError::FrameworkError(e) => write!(f, "erorr occured in framework: {}", e),
            AgentError::BadFrame(e) => write!(f, "program got a bad or unexpected ws frame: {}", e),
        }
    }
}

impl std::error::Error for AgentError {}
