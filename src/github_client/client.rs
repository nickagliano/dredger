use super::data::{RepoContent, RepoNode};
use crate::utils::errors::DredgerError;
use crate::utils::tokens::{count_tokens, TokenizerError};
use base64::prelude::*;
use base64::Engine;
use futures::future::BoxFuture;
use reqwest::Client;
use serde_json::json;
use std::env;
use std::error::Error;
use std::path::Path;
use tokenizers::Tokenizer;

async fn fetch_file_content(
    client: &Client,
    repo_owner: &str,
    repo_name: &str,
    file_path: &str,
    github_token: &str,
) -> Result<String, Box<dyn Error>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        repo_owner, repo_name, file_path
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", github_token))
        .header("User-Agent", "dredger")
        .send()
        .await?;

    if response.status().is_success() {
        let file_info: RepoContent = response.json().await?;

        if let Some(encoded_content) = file_info.content {
            let decoded_bytes = BASE64_STANDARD.decode(encoded_content.replace("\n", ""))?;
            return Ok(String::from_utf8_lossy(&decoded_bytes).to_string());
        }
    }

    Err("Failed to fetch file content".into())
}

// Recursive function to fetch repo structure
fn read_repo_recursive(
    client: Client,
    repo_owner: String,
    repo_name: String,
    tokenizer_path: String,
    path: String,
    github_token: String,
) -> BoxFuture<'static, Result<RepoNode, Box<DredgerError>>> {
    Box::pin(async move {
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            repo_owner, repo_name, path
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", github_token))
            .header("User-Agent", "my-rust-app")
            .send()
            .await
            .map_err(|e| Box::new(DredgerError::ReqwestError(e)))?;

        if response.status().is_success() {
            let repo_contents: Vec<RepoContent> = response
                .json()
                .await
                .map_err(|e| Box::new(DredgerError::ReqwestError(e)))?;

            let mut children = Vec::new();

            for file in repo_contents {
                if file.r#type == "file" {
                    let content = fetch_file_content(
                        &client,
                        &repo_owner,
                        &repo_name,
                        &file.path,
                        &github_token,
                    )
                    .await
                    .unwrap_or_else(|_| "Failed to fetch content".to_string());

                    // let tokenizer_path = "tokenizers/llama.json";
                    //
                    let copy_of_tokenizer_path = tokenizer_path.clone();

                    if !Path::new(&copy_of_tokenizer_path).exists() {
                        return Err(Box::new(DredgerError::TokenizerError(
                            TokenizerError::FileNotFound(tokenizer_path.to_string()),
                        )));
                    }

                    let tokenizer = Tokenizer::from_file(copy_of_tokenizer_path).map_err(|e| {
                        Box::new(DredgerError::TokenizerError(TokenizerError::LoadError(
                            e.to_string(),
                        )))
                    })?;

                    let token_count = count_tokens(&content, &tokenizer).unwrap();

                    children.push(RepoNode::File {
                        name: file.name,
                        path: file.path,
                        content,
                        token_count,
                    });
                } else if file.r#type == "dir" {
                    let subdir_node = read_repo_recursive(
                        client.clone(),
                        repo_owner.clone(),
                        repo_name.clone(),
                        tokenizer_path.clone(),
                        file.path.clone(),
                        github_token.clone(),
                    )
                    .await?;

                    children.push(subdir_node);
                }
            }

            // Sum the token counts from all children (files and directories)
            let total_token_count = children
                .iter()
                .map(|child| child.token_count())
                .sum::<usize>();

            Ok(RepoNode::Directory {
                name: path.clone(),
                path,
                children,
                token_count: total_token_count,
            })
        } else {
            eprintln!("Failed to fetch repository contents: {}", response.status());
            Err(Box::new(DredgerError::GithubClientError(format!(
                "Failed to fetch repository contents: {}",
                response.status()
            ))))
        }
    })
}

/// This method calls read_repo_recursive in order to extract info from
/// the code repository at github.com/{repo_owner}/{repo_name}
///
/// It parses GitHub file-trees into `RepoNode`s, which are a core
/// data structure in Dredger.
///
/// Although it might be a little bit unclear, for efficiency sake,
/// we're also calculating the # of language model tokens in this
/// GitHub client, in the read_repo / read_repo_recursive functions.
///
// TODO: Add branch name?
pub async fn read_repo(
    repo_owner: String,
    repo_name: String,
    tokenizer_path: String,
) -> Result<RepoNode, Box<DredgerError>> {
    let client = Client::new();

    // At this point the github_token was already validated,
    // so we don't check again here--we just load the token
    let github_token =
        std::env::var("GITHUB_PAT").map_err(|e| Box::new(DredgerError::VarError(e)))?;

    let root_node = read_repo_recursive(
        client,
        repo_owner,
        repo_name,
        tokenizer_path,
        "".to_string(), // Indicates root, start of recursion
        github_token,
    )
    .await?;

    Ok(root_node)
}

pub async fn validate_token() -> Result<(), DredgerError> {
    let client = Client::new();

    // Get the GitHub token from the environment variable
    let token = env::var("GITHUB_PAT").map_err(|e| DredgerError::VarError(e))?;

    // Determine the environment (default to production)
    let current_env = env::var("ENV").unwrap_or_else(|_| "production".to_string());

    // Choose the correct URL based on the environment
    let url = if current_env == "test" {
        // Use mockito's server URL and append "/user"
        format!("{}/user", mockito::server_url())
    } else {
        "https://api.github.com/user".to_string()
    };

    // Make the GET request with the necessary headers
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "dredger") // GitHub requires a User-Agent header
        .send()
        .await;

    // Process the response
    match res {
        Ok(response) => {
            if response.status().is_success() {
                Ok(())
            } else {
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                Err(DredgerError::GithubClientError(format!(
                    "Request failed with status {}: {}",
                    status, body
                )))
            }
        }
        Err(e) => Err(DredgerError::ReqwestError(e)),
    }
}

pub async fn make_request<T>(
    client: &Client,
    url: &str,
    method: reqwest::Method,
    body: Option<serde_json::Value>,
    token: &str,
) -> Result<T, Box<dyn Error>>
where
    T: serde::de::DeserializeOwned,
{
    let mut request = client
        .request(method, url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "dredger");

    if let Some(body) = body {
        request = request.json(&body);
    }

    let response = request.send().await?;
    let status = response.status();

    // Capture the response text to handle errors
    let error_text = &response.text().await.unwrap_or_default();

    if !status.is_success() {
        // Use the captured error_text for error handling
        eprintln!("Request failed: {}: {}", status, error_text);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Request failed",
        )));
    }

    // Parse the successful response into the expected result type
    let response_json: T = serde_json::from_str(&error_text)?;
    Ok(response_json)
}

pub async fn create_branch(
    client: &Client,
    owner: &str,
    repo: &str,
    base_sha: &str,
    new_branch: &str,
    token: &str,
) -> Result<(), Box<dyn Error>> {
    let create_ref_url = format!("https://api.github.com/repos/{}/{}/git/refs", owner, repo);
    let new_ref_body = json!({
        "ref": format!("refs/heads/{}", new_branch),
        "sha": base_sha,
    });

    let _: serde_json::Value = make_request(
        client,
        &create_ref_url,
        reqwest::Method::POST,
        Some(new_ref_body),
        token,
    )
    .await?;
    Ok(())
}

async fn add_file_to_repo(
    client: &Client,
    owner: &str,
    repo: &str,
    file_path: &str,
    file_content: &str,
    new_branch: &str,
    token: &str,
) -> Result<(), Box<dyn Error>> {
    let create_file_url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner, repo, file_path
    );
    let encoded_content = base64::engine::general_purpose::STANDARD.encode(file_content);

    let create_file_body = json!({
        "message": format!("Add {}", file_path),
        "content": encoded_content,
        "branch": new_branch
    });

    let _: serde_json::Value = make_request(
        client,
        &create_file_url,
        reqwest::Method::PUT,
        Some(create_file_body),
        token,
    )
    .await?;
    Ok(())
}

pub async fn create_pull_request(
    client: &Client,
    owner: &str,
    repo: &str,
    base_branch: &str,
    new_branch: &str,
    title: &str,
    body: &str,
    token: &str,
) -> Result<String, Box<dyn Error>> {
    let create_pr_url = format!("https://api.github.com/repos/{}/{}/pulls", owner, repo);
    let create_pr_body = json!({
        "title": title,
        "head": new_branch,
        "base": base_branch,
        "body": body
    });

    let pr_response_json: serde_json::Value = make_request(
        client,
        &create_pr_url,
        reqwest::Method::POST,
        Some(create_pr_body),
        token,
    )
    .await?;
    let pr_url = pr_response_json["html_url"]
        .as_str()
        .ok_or("PR URL not found")?
        .to_string();

    Ok(pr_url)
}

pub async fn open_test_pr() -> Result<(), Box<dyn Error>> {
    let token = env::var("GITHUB_PAT").map_err(|_| "Missing GITHUB_PAT environment variable")?;
    let client = Client::new();

    let owner = "nickagliano";
    let repo = "tbg-rust";
    let base_branch = "master";
    let new_branch = "hello-world-test-1";

    // Get the SHA of the base branch
    let base_ref_url = format!(
        "https://api.github.com/repos/{}/{}/git/ref/heads/{}",
        owner, repo, base_branch
    );
    let base_ref_resp: serde_json::Value =
        make_request(&client, &base_ref_url, reqwest::Method::GET, None, &token).await?;
    let base_sha = base_ref_resp["object"]["sha"]
        .as_str()
        .ok_or("Could not find base SHA")?;

    // 1. Create a new branch
    create_branch(&client, owner, repo, base_sha, new_branch, &token).await?;

    // 2. Add a file
    add_file_to_repo(
        &client,
        owner,
        repo,
        "hello.txt",
        "hello world",
        new_branch,
        &token,
    )
    .await?;

    // 3. Open a pull request
    let pr_url = create_pull_request(
        &client,
        owner,
        repo,
        base_branch,
        new_branch,
        "Test PR: Hello World",
        "This PR adds a hello world file.",
        &token,
    )
    .await?;

    println!("Pull request created: {}", pr_url);

    Ok(())
}
