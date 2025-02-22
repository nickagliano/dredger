// use crate::ollama_client::client as ollama_client;
use crate::github_client::client as github_client;
use crate::github_client::data::RepoNode;
use crate::utils::errors::DredgerError;
use colored::*;

/// This is the most important function of dredger
///
/// Resposibilities:
/// - Calls github client to get repo structure, content, and an
///   estimated # of language model tokens required to parse the content
/// - Passes parsed repo content to the ollama client, which will
///   chunk up the content into LLM-digestible sizes
pub async fn dredge_repo(
    quiet: bool,
    repo_owner: String,
    repo_name: String,
    tokenizer_path: String,
) -> Result<RepoNode, Box<DredgerError>> {
    // First, read the repo into dredger RepoNode structure
    // - root node (dir node)
    //   - dir node
    //     - file node
    //     - file node
    //   - dir node
    //     - file node
    //
    // Each node, whether dir or file, will have a "token_count".
    //
    // FIXME: Define the tokenizer here, then pass it around instead of re-creating it each
    //        call to parse_repo_recursive
    let root_node = github_client::read_repo(repo_owner, repo_name, tokenizer_path).await;

    // TODO: run Ollama, based on the root node
    // ollama_client::process_root_node();
    //
    // ... this is where we would really iterate on the ollama stuff...
    // ... try and get self-improvement loop, self-rating/self-judging on the docs...
    // ... branching LLM calls in, like 10 equal prompts, and choosing best response...
    // ... if it thinks the docs are good enough, then we can open PR.
    //

    // TODO: If ollama generated good docs that are different enough
    //       from current docs, open PR.
    let open_new_pr_flag = false;

    if open_new_pr_flag {
        // TODO: If there's already a Dredger PR open, edit that PR!
        if let Err(e) = github_client::open_test_pr().await {
            if quiet {
                eprintln!("Could not open pull request");
                // TODO: How to handle this...?
                //       - Return partial success...?
                //       - Don't return root_node, but instead just
                //         a DredgerError?
            } else {
                println!(
                    "{} {}",
                    "\n❌ Could not open pull request.\n".bold().red(),
                    e
                );
                // TODO: How to handle this...?
                //       - Return partial success...?
                //       - Don't return root_node, but instead just
                //         a DredgerError?
            }
        } else {
            println!("Success! Opened PR!")
        }
    }

    root_node
}
