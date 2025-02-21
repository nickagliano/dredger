use base64::prelude::*;
use base64::Engine;
use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::error::Error;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoContent {
    pub name: String,
    pub path: String,
    pub r#type: String,          // "file" or "dir"
    pub content: Option<String>, // Only present in single file requests
}

#[derive(Debug)]
pub enum RepoNode {
    File {
        name: String,
        path: String,
        content: String,
    },
    Directory {
        name: String,
        path: String,
        children: Vec<RepoNode>,
    },
}

// Fetch file content separately
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
        .header("User-Agent", "my-rust-app")
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
    path: String,
    github_token: String,
) -> BoxFuture<'static, Result<RepoNode, Box<dyn Error>>> {
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
            .await?;

        if response.status().is_success() {
            let repo_contents: Vec<RepoContent> = response.json().await?;
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

                    children.push(RepoNode::File {
                        name: file.name,
                        path: file.path,
                        content,
                    });
                } else if file.r#type == "dir" {
                    let subdir_node = read_repo_recursive(
                        client.clone(),
                        repo_owner.clone(),
                        repo_name.clone(),
                        file.path.clone(),
                        github_token.clone(),
                    )
                    .await?;

                    children.push(subdir_node);
                }
            }

            Ok(RepoNode::Directory {
                name: path.clone(),
                path,
                children,
            })
        } else {
            eprintln!("Failed to fetch repository contents: {}", response.status());
            Err("Failed to fetch repository contents".into())
        }
    })
}

pub async fn read_repo() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let repo_owner = "nickagliano".to_string();
    let repo_name = "dredger".to_string();
    let github_token = std::env::var("GITHUB_PAT")?; // Load token from env

    println!("✅ GitHub Token verified. Proceeding...");

    let root_node =
        read_repo_recursive(client, repo_owner, repo_name, "".to_string(), github_token).await?;

    println!("{:#?}", root_node);

    Ok(())
}

pub async fn validate_token() -> Result<(), String> {
    let client = Client::new();

    // Get the GitHub token from the environment variable
    let token = env::var("GITHUB_PAT")
        .map_err(|_| "Missing GITHUB_PAT environment variable".to_string())?;

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
                Err(format!("Request failed with status {}: {}", status, body))
            }
        }
        Err(e) => Err(format!("Request failed with error: {}", e)),
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
