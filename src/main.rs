use clap::{Arg, Command};
use colored::*;
use dotenv::dotenv;
use dredger::core;
use dredger::github_client::client as github_client;
use dredger::utils::cli::{get_token_from_env, setup_token};
use std::{env, process::exit};
use tokio;

// TODO: Constantize/enum-ize the environments (prod, test) and .env file paths
fn load_env() {
    let env = env::var("ENV").unwrap_or_else(|_| "production".to_string());

    if env == "test" {
        dotenv::from_filename(".env.test").ok(); // Load .env.test if in test mode
    } else {
        dotenv().ok(); // Load default .env file for production
    }
}

#[tokio::main]
async fn main() {
    load_env();

    // Parse CLI arguments
    let matches = Command::new("Dredger")
        .version("1.0")
        .author("Nick Agliano <nickagliano@gmail.com>")
        .about("GitHub Token Validator & Setup Tool")
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Run in quiet mode (minimal output)")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let quiet = matches.get_flag("quiet");

    if !quiet {
        println!("{}", "\nRunning Dredger...\n".bold().cyan());
    }

    loop {
        // Check for existing GitHub token setup
        if let Err(_) = get_token_from_env(None) {
            if quiet {
                eprintln!("Error: No valid GitHub token found.");
                exit(1);
            } else {
                setup_token(quiet); // Setup the token if it isn't found
            }
        }

        // Validate token
        if let Err(_) = github_client::validate_token().await {
            if quiet {
                eprintln!("Error: Invalid GitHub token.");
                exit(1);
            } else {
                println!(
                    "{}",
                    "\n❌ Invalid GitHub token. Please try again.\n"
                        .bold()
                        .red()
                );
                setup_token(quiet); // Prompt user to enter a new token if invalid
                continue; // Retry the validation after new token entry
            }
        }

        if !quiet {
            println!(
                "{}",
                "\n✅ GitHub Token verified. Proceeding...\n".bold().green()
            );
        }

        break; // Exit loop once token is valid
    }

    // TODO: Let users set this via CLI or via some other UI
    let repo_owner = "nickagliano".to_string();
    let repo_name = "dredger".to_string();

    // TODO: Implement multiple models, update this based on selected open source model
    let tokenizer_path = "tokenizers/llama.json".to_string(); // or "deepseek-tokenizer.json"

    core::actions::dredge_repo(quiet, repo_owner, repo_name, tokenizer_path)
        .await
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;
    use std::env;
    use std::fs::{remove_file, write, File};
    use std::io::Write;
    use std::path::Path;

    fn random_suffix() -> String {
        let random_number: u32 = rand::random_range(1000..9999);
        format!("{}", random_number)
    }

    fn cleanup_env_test_file(suffix: &str) {
        let test_file_name = format!(".env.test.{}", suffix);
        if Path::new(&test_file_name).exists() {
            remove_file(test_file_name).expect("Unable to remove test file");
        }
    }

    #[test]
    fn test_check_and_setup_success() {
        let suffix = random_suffix(); // Generate a random suffix

        // Simulate .env.test file with a token
        let _ = write(format!(".env.test.{}", suffix), "GITHUB_PAT=test_token");

        // Set ENV to "test" so the correct file gets loaded
        env::set_var("ENV", "test");

        // Should return Ok because the token is present
        assert_eq!(get_token_from_env(Some(&suffix)), Ok(()));

        cleanup_env_test_file(&suffix);
    }

    #[test]
    fn test_check_and_setup_missing_env_file() {
        let suffix = random_suffix(); // Generate a random suffix

        // Set ENV to "test" so the correct file gets loaded
        env::set_var("ENV", "test");

        // Should return error because the random .env.test file doesn't exist
        assert_eq!(get_token_from_env(Some(&suffix)), Err("Missing .env file"));

        cleanup_env_test_file(&suffix);
    }

    #[test]
    fn test_check_and_setup_missing_token() {
        let suffix = random_suffix(); // Generate a random suffix

        // Create the unique .env.test file for this test
        let test_file_name = format!(".env.test.{}", suffix);
        let mut file = File::create(&test_file_name).expect("Unable to create test file");
        file.write_all(b"").expect("Unable to write to .env.test");

        // Set ENV to "test" so the correct file gets loaded
        env::set_var("ENV", "test");

        // Should return error because the token is missing
        assert_eq!(
            get_token_from_env(Some(&suffix)),
            Err("Missing GITHUB_PAT in .env file")
        );

        // Clean up after the test
        remove_file(test_file_name).expect("Unable to remove test file");

        cleanup_env_test_file(&suffix);
    }

    #[tokio::test]
    async fn test_validate_token_invalid() {
        let _m = mock("GET", "/user")
            .with_header("Authorization", "Bearer invalid_token")
            .with_status(401)
            .create();

        // Set the GITHUB_PAT in environment variables for the test
        env::set_var("GITHUB_PAT", "invalid_token");
        env::set_var("ENV", "test"); // Set ENV to test

        // Should return Err for invalid token
        assert!(github_client::validate_token().await.is_err());
    }

    #[tokio::test]
    async fn test_validate_token_api_error() {
        let _m = mock("GET", "/user")
            .with_header("Authorization", "Bearer test_token")
            .with_status(500)
            .create();

        // Set the GITHUB_PAT in environment variables for the test
        env::set_var("GITHUB_PAT", "test_token");
        env::set_var("ENV", "test"); // Set ENV to test

        // Should return Err for API communication failure
        assert!(github_client::validate_token().await.is_err());
    }

    #[tokio::test]
    async fn test_validate_token_valid() {
        // Create a mock for GET /user that returns a 200 OK response
        let _m = mockito::mock("GET", "/user")
            .with_header("Authorization", "Bearer test_token")
            .with_status(200)
            .with_body("{\"login\": \"test_user\"}")
            .create();

        // Set environment variables for the test
        env::set_var("GITHUB_PAT", "test_token");
        env::set_var("ENV", "test");

        // Call the function with the full mock server URL
        let result = github_client::validate_token().await;

        match &result {
            Ok(_) => println!("Token validated successfully"),
            Err(e) => println!("Token validation failed with error: {}", e),
        }

        assert!(result.is_ok(), "Token validation failed");
    }
}
