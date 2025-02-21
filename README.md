# Dredger

**Dredger** is a Rust-based tool designed to automatically scan a codebase, generate doc-level comments, and submit pull requests with detailed insights. It works asynchronously in the background, "dredging" your code for the context that designers and developers need to understand it better.

## Features

- **Automated Documentation**: Generate doc-level comments that describe functions, logic, and design intent.
- **GitHub Integration**: Fetch code from GitHub repositories and create pull requests with documentation.
- **Rust Native**: Built with Rust for fast, efficient, and safe operation.
- **Async Processing**: Handles large codebases asynchronously, providing fast, non-blocking results.

## Getting Started

### Prerequisites

1. **Rust**: Install Rust by following the instructions on [rust-lang.org](https://www.rust-lang.org/learn/get-started).
2. **GitHub Token**: Generate a personal access token for GitHub with repo access.

### Installation

Clone the repository and build the project:

```bash
git clone https://github.com/yourusername/dredger.git
cd dredger
cargo build --release
```

### Configuration

Before running Dredger, you'll need to set your GitHub token.

1. Copy the .env.sample file to .env:

```bash
cp .env.sample .env
```

2. Open the `.env` file and fill in your GitHub personal access token:

```bash
GITHUB_TOKEN=your_personal_access_token_here
```


### Usage
To start scanning a repository:
(NOT YET IMPLEMENTED)
```bash
cargo run -- --repo https://github.com/username/repository
```

This command will fetch the repository, parse the code, generate doc-level comments, and create a pull request with the changes.


### Contribution
We welcome contributions! Feel free to fork the repository, make changes, and submit pull requests. Here are some areas you can help with:

- Improving code parsing and comment generation.
- Adding more GitHub API features.
- Writing better documentation.


### License

Coming soon...
