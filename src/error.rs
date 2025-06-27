use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;
pub type ConfigError = Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Feed parsing error: {0}")]
    FeedParse(String),
    
    #[error("HTTP error: {0}")]
    HttpError(String),
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    
    #[error("Request timeout: {0}")]
    Timeout(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("FUSE error: {0}")]
    Fuse(String),
    
    // #[error("Database error: {0}")]
    // Database(#[from] rusqlite::Error),
    
    #[error("Cache error: {0}")]
    Cache(String),
    
    #[error("Content extraction error: {0}")]
    ContentExtraction(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Already exists: {0}")]
    AlreadyExists(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
    
    #[error("Invalid: {0}")]
    Invalid(String),
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Config(err.to_string())
    }
}

impl Error {
    pub fn is_temporary(&self) -> bool {
        matches!(
            self,
            Error::HttpError(_) | Error::Timeout(_) | Error::Io(_)
        )
    }
    
    pub fn is_user_error(&self) -> bool {
        matches!(
            self,
            Error::InvalidUrl(_) | Error::Config(_) | Error::PermissionDenied(_)
        )
    }
    
    pub fn error_code(&self) -> &'static str {
        match self {
            Error::FeedParse(_) => "FEED_PARSE",
            Error::HttpError(_) => "HTTP_ERROR",
            Error::InvalidUrl(_) => "INVALID_URL",
            Error::Timeout(_) => "TIMEOUT",
            Error::Io(_) => "IO_ERROR",
            Error::Serialization(_) => "SERIALIZATION",
            Error::Config(_) => "CONFIG",
            Error::Fuse(_) => "FUSE",
            // Error::Database(_) => "DATABASE",
            Error::Cache(_) => "CACHE",
            Error::ContentExtraction(_) => "CONTENT_EXTRACTION",
            Error::Storage(_) => "STORAGE",
            Error::PermissionDenied(_) => "PERMISSION_DENIED",
            Error::NotFound(_) => "NOT_FOUND",
            Error::AlreadyExists(_) => "ALREADY_EXISTS",
            Error::InvalidState(_) => "INVALID_STATE",
            Error::ResourceExhausted(_) => "RESOURCE_EXHAUSTED",
            Error::Unknown(_) => "UNKNOWN",
            Error::Invalid(_) => "INVALID",
        }
    }
}