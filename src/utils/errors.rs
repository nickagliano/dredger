use super::tokens::TokenizerError;
use reqwest::Error as ReqwestError;
use std::env::VarError;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum DredgerError {
    GithubClientError(String),
    OllamaClientError(String),
    TokenizerError(TokenizerError),
    ReqwestError(ReqwestError),
    IoError(io::Error),
    JsonError(serde_json::Error),
    OtherError(String),
    VarError(VarError),
}

// Implement fmt::Display for the general DredgerError
impl fmt::Display for DredgerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DredgerError::TokenizerError(e) => write!(f, "Tokenizer Error: {}", e),
            DredgerError::ReqwestError(e) => write!(f, "Network Error: {}", e),
            DredgerError::IoError(e) => write!(f, "IO Error: {}", e),
            DredgerError::JsonError(e) => write!(f, "JSON Error: {}", e),
            DredgerError::OtherError(msg) => write!(f, "Other Error: {}", msg),
            DredgerError::GithubClientError(msg) => write!(f, "GitHub Client Error: {}", msg),
            DredgerError::OllamaClientError(msg) => write!(f, "Ollama Client Error: {}", msg),
            DredgerError::VarError(msg) => write!(f, "Environment Variable Error: {}", msg),
        }
    }
}

// Implement the std::error::Error trait for the general DredgerError
impl std::error::Error for DredgerError {}
