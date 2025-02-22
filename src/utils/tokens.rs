use std::error::Error;
use std::fmt;
use tokenizers::Tokenizer;

#[derive(Debug)]
pub enum TokenizerError {
    FileNotFound(String),
    LoadError(String),
    TokenizationError(String),
}

// Implement fmt::Display to make the TokenizerError human-readable
impl fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenizerError::FileNotFound(path) => {
                write!(f, "Tokenizer file not found: {}", path)
            }
            TokenizerError::LoadError(msg) => write!(f, "Failed to load tokenizer: {}", msg),
            TokenizerError::TokenizationError(msg) => {
                write!(f, "Tokenization failed: {}", msg)
            }
        }
    }
}

// Implement the std::error::Error trait (needed for the `Box<dyn Error>` type)
impl Error for TokenizerError {}

/// Returns the # of large language model tokens required to parse the text content (code + documentation)
pub fn count_tokens(content: &str, tokenizer: &Tokenizer) -> Result<usize, TokenizerError> {
    let encoding = tokenizer
        .encode(content, true)
        .map_err(|e| TokenizerError::TokenizationError(format!("{}", e)))?;

    Ok(encoding.get_ids().len())
}
