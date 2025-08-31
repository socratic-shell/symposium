use std::fs;
use tempfile::TempDir;

/// Action-based test repository builder
pub struct TestRepo {
    actions: Vec<RepoAction>,
}

#[derive(Debug, Clone)]
enum RepoAction {
    /// Overwrite file content
    Overwrite { path: String, content: String },
    /// Append to file content
    Append { path: String, content: String },
    /// Add file changes to Git index
    Add { path: String },
    /// Create a commit with current staged changes
    Commit { message: String },
}

impl TestRepo {
    /// Create a new test repository builder
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }
    
    /// Overwrite file content (file remains unstaged)
    pub fn overwrite(mut self, path: &str, content: &str) -> Self {
        self.actions.push(RepoAction::Overwrite {
            path: path.to_string(),
            content: content.to_string(),
        });
        self
    }
    
    /// Append to file content (file remains unstaged)
    pub fn append(mut self, path: &str, content: &str) -> Self {
        self.actions.push(RepoAction::Append {
            path: path.to_string(),
            content: content.to_string(),
        });
        self
    }
    
    /// Add file changes to Git index
    pub fn add(mut self, path: &str) -> Self {
        self.actions.push(RepoAction::Add {
            path: path.to_string(),
        });
        self
    }
    
    /// Overwrite file and immediately add to index
    pub fn overwrite_and_add(mut self, path: &str, content: &str) -> Self {
        self.actions.push(RepoAction::Overwrite {
            path: path.to_string(),
            content: content.to_string(),
        });
        self.actions.push(RepoAction::Add {
            path: path.to_string(),
        });
        self
    }
    
    /// Append to file and immediately add to index
    pub fn append_and_add(mut self, path: &str, content: &str) -> Self {
        self.actions.push(RepoAction::Append {
            path: path.to_string(),
            content: content.to_string(),
        });
        self.actions.push(RepoAction::Add {
            path: path.to_string(),
        });
        self
    }
    
    /// Create a commit with current staged changes
    pub fn commit(mut self, message: &str) -> Self {
        self.actions.push(RepoAction::Commit {
            message: message.to_string(),
        });
        self
    }
    
    /// Execute all actions and create the temporary repository
    pub fn create(self) -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();
        
        // Initialize Git repository
        let repo = git2::Repository::init(repo_path).expect("Failed to init git repo");
        
        // Configure git user
        let mut config = repo.config().expect("Failed to get repo config");
        config.set_str("user.name", "Test User").expect("Failed to set user name");
        config.set_str("user.email", "test@example.com").expect("Failed to set user email");
        
        let signature = git2::Signature::now("Test User", "test@example.com")
            .expect("Failed to create signature");
        
        // Execute actions in sequence
        for action in self.actions {
            match action {
                RepoAction::Overwrite { path, content } => {
                    let file_path = repo_path.join(&path);
                    if let Some(parent) = file_path.parent() {
                        fs::create_dir_all(parent).expect("Failed to create parent directories");
                    }
                    fs::write(&file_path, &content).expect("Failed to write file");
                }
                RepoAction::Append { path, content } => {
                    let file_path = repo_path.join(&path);
                    if let Some(parent) = file_path.parent() {
                        fs::create_dir_all(parent).expect("Failed to create parent directories");
                    }
                    
                    let existing = if file_path.exists() {
                        fs::read_to_string(&file_path).unwrap_or_default()
                    } else {
                        String::new()
                    };
                    
                    fs::write(&file_path, format!("{}{}", existing, content))
                        .expect("Failed to append to file");
                }
                RepoAction::Add { path } => {
                    let mut index = repo.index().expect("Failed to get index");
                    index.add_path(std::path::Path::new(&path)).expect("Failed to add file to index");
                    index.write().expect("Failed to write index");
                }
                RepoAction::Commit { message } => {
                    let mut index = repo.index().expect("Failed to get index");
                    let tree_id = index.write_tree().expect("Failed to write tree");
                    let tree = repo.find_tree(tree_id).expect("Failed to find tree");
                    
                    let parent_commit = repo.head()
                        .ok()
                        .and_then(|head| head.target())
                        .and_then(|oid| repo.find_commit(oid).ok());
                    
                    let parents: Vec<&git2::Commit> = if let Some(ref parent) = parent_commit {
                        vec![parent]
                    } else {
                        vec![]
                    };
                    
                    repo.commit(
                        Some("HEAD"),
                        &signature,
                        &signature,
                        &message,
                        &tree,
                        &parents,
                    ).expect("Failed to create commit");
                }
            }
        }
        
        temp_dir
    }
}
