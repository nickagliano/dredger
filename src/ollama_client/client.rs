use crate::github_client::data::RepoNode;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: String,
    examples: Vec<(String, String)>,
}

#[derive(Deserialize)]
struct OllamaChunk {
    response: Option<String>, // Some chunks might not contain "response"
}

pub async fn query_ollama(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = "http://localhost:11434/api/generate";

    let req_body = OllamaRequest {
        model: "llama3.1".to_string(),
        prompt: prompt.to_string(),
        system: "You are an AI that generates Rust doc comments using `//!` style.
                 Given a function, struct, or module, write concise, idiomatic Rust documentation
                 that explains its purpose, usage, and important details."
                 .to_string(),
        examples: vec![
            (
                "fn calculate_area(radius: f64) -> f64 { std::f64::consts::PI * radius * radius }".to_string(),
                "//! Computes the area of a circle.\n//! \n//! # Arguments\n//! * `radius` - The radius of the circle.\n//! \n//! # Returns\n//! The computed area.".to_string()
            ),
            (
                "struct Config { timeout: u32, verbose: bool }".to_string(),
                "//! Holds configuration settings for the application.\n//! \n//! Includes parameters for timeout and verbosity.".to_string()
            )
        ],
    };

    let mut res = client.post(url).json(&req_body).send().await?;

    let mut full_response = String::new();

    // Read the response as a stream
    while let Some(chunk) = res.chunk().await? {
        let chunk_str = String::from_utf8_lossy(&chunk);

        for line in chunk_str.lines() {
            if let Ok(parsed) = serde_json::from_str::<OllamaChunk>(line) {
                if let Some(text) = parsed.response {
                    full_response.push_str(&text);
                }
            }
        }
    }

    Ok(full_response)
}

pub async fn process_repo(root_node: &RepoNode) -> Result<(), Box<dyn Error>> {
    // Use a stack to hold directories to process
    let mut stack: Vec<&RepoNode> = vec![&root_node];

    // Iteratively process the stack
    while let Some(node) = stack.pop() {
        match node {
            RepoNode::File { path, content, .. } => {
                println!("📄 Sending file: {}", path);

                match query_ollama(content).await {
                    Ok(response) => println!("LLM Response for {}: {}", path, response),
                    Err(e) => eprintln!("Error querying Ollama for {}: {}", path, e),
                }
            }
            RepoNode::Directory { children, .. } => {
                // Push all children (files and directories) to the stack
                for child in children {
                    stack.push(child);
                }
            }
        }
    }

    Ok(())
}
