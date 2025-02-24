use crate::github_client::data::RepoNode;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

// TODO: Use this! Keep track of context window size by model, and current prompt. Actually use tokenizer.
// const MAX_TOKENS: usize = 128000; // Estimated... (maybe set this lower, keep a buffer..)

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

#[derive(Debug)]
pub struct DredgerDoc {
    pub file_path: String,
    pub comments: String, // Only extracted comments
}

// FIXME: Consolidate with  query_ollama_for_project_overview, share some abstractions
async fn query_ollama_for_doc(
    project_context: &str,
    file_path: &str, // FIXME: Use the file path in the prompt
    prompt: &str,
) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let url = "http://localhost:11434/api/generate";

    let system_prompt = format!("You are an AI that generates Rust doc comments using `//!` style. It's very important that you use //! for comments.
             Given a file or section of a file, write concise, idiomatic Rust documentation that explains its purpose, usage, and important details.
             This file is found at {}.\n
             Lastly, here is a project overview to help you generate docs. DO NOT include this summary, or any variation, in your docs!: {}", file_path, project_context);

    let req_body = OllamaRequest {
        model: "llama3.1".to_string(),
        prompt: prompt.to_string(),
        system: system_prompt,
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

// FIXME: Consolidate with  query_ollama_for_doc, share some abstractions
async fn query_ollama_for_project_overview(prompt: &str) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let url = "http://localhost:11434/api/generate";

    let req_body = OllamaRequest {
        model: "llama3.1".to_string(),
        prompt: prompt.to_string(),
        system: "You are an impersonal AI that summarizes the first 10 lines of a GitHub project's README. You should be very succinct and to the point, and just return a brief summary."
                 .to_string(),
        examples: vec![
            (
                "# RustyWeb - A Minimal Rust Web Server\n\nRustyWeb is a simple, lightweight web server built using Actix Web.  \nIt serves HTTP requests efficiently and is designed for ease of use.\n\n## Features\n- 🚀 Fast and lightweight\n- 🔧 Built with Actix Web\n- 📦 Supports JSON API responses\n\n## Installation\nTo install dependencies, run:".to_string(),
                "A lightweight web server built with Actix Web, designed for fast HTTP handling and JSON API support.".to_string()
            ),
            (
                "# RustyTodo - A Simple CLI Todo List\n\nRustyTodo is a minimalistic command-line todo list manager written in Rust.\nIt saves tasks to a file and allows easy task management.\n## Features\n- 📝 Add, remove, and list tasks\n- 💾 Persistent storage in a text file\n- 🦀 Built in Rust for speed and safety  \n\n## Usage\nRun the following command to start:".to_string(),
                "A simple CLI-based todo list manager in Rust with persistent text file storage and task management commands.".to_string()
            )
        ],
    };

    let mut res = client.post(url).json(&req_body).send().await?;

    let mut full_response = String::new();

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

// FIXME: This is sort of a mess in terms of abstractions.
pub async fn process_repo(root_node: &RepoNode) -> Result<Vec<DredgerDoc>, Box<dyn Error>> {
    let mut stack: Vec<&RepoNode> = vec![root_node];
    let mut doc_results = Vec::new();
    let mut project_context = String::new();

    // Step 1: Extract Project-Level Context from README
    // FIXME: Abstract this out
    while let Some(node) = stack.pop() {
        match node {
            RepoNode::File { path, content, .. } => {
                if path.ends_with("README") || path.ends_with("README.md") {
                    project_context = extract_project_context(content);
                    break; // Stop after finding the README
                }
            }
            RepoNode::Directory { children, .. } => {
                for child in children {
                    stack.push(child);
                }
            }
        }
    }

    // Step 1.5: Summarize the project context
    // FIXME: Abstract this out!
    let project_summary = if !project_context.is_empty() {
        match query_ollama_for_project_overview(&project_context).await {
            Ok(summary) => summary,
            Err(e) => {
                eprintln!("Error summarizing project context: {}", e);
                String::new()
            }
        }
    } else {
        String::new()
    };

    if !project_summary.is_empty() {
        println!("📄 Project Summary:\n{}", project_summary);
        // Replace project_context with summary if present; otherwise we will just
        // use the raw first N lines of the README
        project_context = project_summary
    }

    // Reinitialize stack
    stack = vec![root_node];

    // Step 2: Process Rust files with project context
    while let Some(node) = stack.pop() {
        match node {
            RepoNode::File { path, content, .. } => {
                // Skip non-Rust files
                // TODO: Could probably learn invaluable info if we read non-language files
                // TODO: Handle non-rust repo (.rb files)

                if !path.ends_with(".rs") {
                    println!("\n⏩ Skipping non-Rust file: {}", path);
                    continue;
                }

                match query_ollama_for_doc(&project_context, &path, content).await {
                    Ok(response) => {
                        let comments = extract_comments(&response);
                        if !comments.is_empty() {
                            println!("\n\nFound comments for {}:\n{}", path.clone(), comments);
                            doc_results.push(DredgerDoc {
                                file_path: path.clone(),
                                comments,
                            });
                        }
                    }
                    Err(e) => eprintln!("Error querying Ollama for {}: {}", path, e),
                }
            }
            RepoNode::Directory { children, .. } => {
                for child in children {
                    stack.push(child);
                }
            }
        }
    }

    Ok(doc_results)
}

fn extract_comments(output: &str) -> String {
    output
        .lines()
        .filter(|line| line.trim().starts_with("//")) // Only keep comments (FIXME: Once prompt is better change this to //!)
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_project_context(readme: &str) -> String {
    readme
        .lines()
        .take(10) // Extract first 10 lines (adjust as needed)
        .collect::<Vec<&str>>()
        .join("\n")
}
