use serde::Deserialize;

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
        token_count: usize,
    },
    Directory {
        name: String,
        path: String,
        children: Vec<RepoNode>,
        token_count: usize,
    },
}

// Define the recursive iterator to sum token counts
impl RepoNode {
    pub fn iter(&self) -> RepoNodeIter {
        RepoNodeIter::new(self)
    }

    pub fn token_count(&self) -> usize {
        match self {
            RepoNode::File { token_count, .. } => *token_count,
            RepoNode::Directory { token_count, .. } => *token_count,
        }
    }
}

pub struct RepoNodeIter<'a> {
    stack: Vec<&'a RepoNode>,
}

impl<'a> RepoNodeIter<'a> {
    pub fn new(root: &'a RepoNode) -> Self {
        RepoNodeIter { stack: vec![root] }
    }
}

impl<'a> Iterator for RepoNodeIter<'a> {
    type Item = &'a RepoNode;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.stack.pop() {
            match node {
                RepoNode::File { .. } => {}
                RepoNode::Directory { children, .. } => {
                    self.stack.extend(children);
                }
            }
            Some(node)
        } else {
            None
        }
    }
}

impl ToString for RepoNode {
    fn to_string(&self) -> String {
        fn format_node(node: &RepoNode, depth: usize) -> String {
            let indent = "  ".repeat(depth);
            match node {
                RepoNode::File {
                    name,
                    path,
                    content: _,
                    token_count,
                } => {
                    format!(
                        "{}📄 {} ({}) - Token count={:?}\n",
                        indent, name, path, token_count
                    )
                }
                RepoNode::Directory {
                    name,
                    path,
                    children,
                    token_count,
                } => {
                    let mut output = format!(
                        "{}📁 {} ({}) - Token count={:?}\n",
                        indent, name, path, token_count
                    );
                    for child in children {
                        output.push_str(&format_node(child, depth + 1));
                    }
                    output
                }
            }
        }
        format_node(self, 0)
    }
}
