use clap::{Arg, Command};
use colored::*;
use dotenv::dotenv;
use dredger::client::client;
use std::path::Path;
use std::{
    env,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    process::exit,
};
use tokio;

fn load_env() {
    let env = env::var("ENV").unwrap_or_else(|_| "production".to_string());

    if env == "test" {
        dotenv::from_filename(".env.test").ok(); // Load .env.test if in test mode
    } else {
        dotenv().ok(); // Load default .env file for production
    }
}

fn setup(quiet: bool) {
    if quiet {
        return;
    }

    println!("{}", "\nSetting up your GitHub token...\n".bold().yellow());

    // Determine the correct .env file based on ENV
    let env_var = env::var("ENV").unwrap_or_else(|_| "production".to_string());
    let env_file = if env_var == "test" {
        ".env.test"
    } else {
        ".env"
    };

    // Read existing file content if it exists
    let mut file_content = String::new();
    if Path::new(env_file).exists() {
        if let Ok(mut file) = File::open(env_file) {
            file.read_to_string(&mut file_content).ok();
        }
    }

    println!(
        "{}",
        "Please enter your GitHub personal access token:"
            .bold()
            .blue()
    );

    let mut token = String::new();
    io::stdin()
        .read_line(&mut token)
        .expect("Failed to read line");

    if token.is_empty() {
        eprintln!("Token cannot be empty.");
        return;
    }

    let token = token.trim();

    // Update the token in the file content or append if not present
    let new_content = if file_content.contains("GITHUB_PAT=") {
        // Replace the existing token line
        file_content
            .lines()
            .map(|line| {
                if line.starts_with("GITHUB_PAT=") {
                    format!("GITHUB_PAT={}", token)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
    } else {
        // Append token to the end of the file
        if file_content.is_empty() {
            format!("GITHUB_PAT={}\n", token)
        } else {
            format!("{}\nGITHUB_PAT={}\n", file_content, token)
        }
    };

    // Write updated content back to the file
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(env_file)
        .expect("Failed to open .env file for writing");
    file.write_all(new_content.as_bytes())
        .expect("Failed to write token to .env");

    // Update the running environment variable
    env::set_var("GITHUB_PAT", token);

    println!("{}", "Token saved successfully\n".yellow());
}

#[tokio::main]
async fn main() {
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

    load_env();

    if !quiet {
        println!("{}", "\nRunning Dredger...\n".bold().cyan());
    }

    loop {
        // Check for existing token setup
        if let Err(_) = check_and_setup(None) {
            if quiet {
                eprintln!("Error: No valid GitHub token found.");
                exit(1);
            } else {
                setup(quiet); // Setup the token if it isn't found
            }
        }

        // Validate token
        if let Err(_) = client::validate_token().await {
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
                setup(quiet); // Prompt user to enter a new token if invalid
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

    if let Err(e) = client::open_test_pr().await {
        if quiet {
            eprintln!("Could not open pull request");
            exit(1);
        } else {
            println!(
                "{} {}",
                "\n❌ Could not open pull request.\n".bold().red(),
                e
            );
        }
    } else {
        println!("Success! Opened PR!")
    }

    client::read_repo().await.unwrap();
}

fn check_and_setup(suffix: Option<&str>) -> Result<(), &'static str> {
    // Determine which .env file to load based on the ENV variable
    let env = env::var("ENV").unwrap_or_else(|_| "production".to_string());
    let env_file = if env == "test" {
        // Use a random suffix if provided, otherwise the default .env.test
        if let Some(suffix) = suffix {
            format!(".env.test.{}", suffix)
        } else {
            ".env.test".to_string()
        }
    } else {
        ".env".to_string()
    };

    // Check if the correct .env file exists
    if !Path::new(&env_file).exists() {
        return Err("Missing .env file");
    }

    // Read the .env file content
    let mut file_content = String::new();
    let mut file = File::open(&env_file).expect("Unable to open .env file");
    file.read_to_string(&mut file_content)
        .expect("Unable to read .env file");

    // Check if the GITHUB_PAT is set in the file
    if !file_content.contains("GITHUB_PAT=") {
        return Err("Missing GITHUB_PAT in .env file");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;
    use std::env;
    use std::fs::{remove_file, write};

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
        assert_eq!(check_and_setup(Some(&suffix)), Ok(()));

        cleanup_env_test_file(&suffix);
    }

    #[test]
    fn test_check_and_setup_missing_env_file() {
        let suffix = random_suffix(); // Generate a random suffix

        // Set ENV to "test" so the correct file gets loaded
        env::set_var("ENV", "test");

        // Should return error because the random .env.test file doesn't exist
        assert_eq!(check_and_setup(Some(&suffix)), Err("Missing .env file"));

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
            check_and_setup(Some(&suffix)),
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
        assert!(client::validate_token().await.is_err());
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
        assert!(client::validate_token().await.is_err());
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
        let result = client::validate_token().await;

        match &result {
            Ok(_) => println!("Token validated successfully"),
            Err(e) => println!("Token validation failed with error: {}", e),
        }

        assert!(result.is_ok(), "Token validation failed");
    }
}
