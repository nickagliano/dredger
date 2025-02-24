use colored::*;
use std::path::Path;
use std::{env, fs::File, io::Read};
use std::{
    fs::OpenOptions,
    io::{self, Write},
};

/// Part of
pub fn setup_token(quiet: bool) {
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

/// Setup GitHub API token
pub fn get_token_from_env(suffix: Option<&str>) -> Result<(), &'static str> {
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
