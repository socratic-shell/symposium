use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Complete state of a synthetic pull request review.
///
/// Contains all information needed to recreate and manage a PR-like review interface,
/// including file changes, AI insight comments, and review status tracking.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReviewState {
    pub review_id: String,
    pub title: String,
    pub description: serde_json::Value,
    pub commit_range: String,
    pub status: ReviewStatus,
    pub files_changed: Vec<crate::git::FileChange>,
    pub comment_threads: Vec<crate::git::CommentThread>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Current status of a synthetic pull request review workflow.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ReviewStatus {
    Pending,
    ChangesRequested,
    Approved,
    Merged,
}

/// A user response to an AI insight comment in a review thread.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserResponse {
    pub author: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl ReviewState {
    /// Save the review state to a JSON file for persistence across sessions.
    ///
    /// # Arguments
    /// * `directory` - Optional directory where to create the file. Uses current directory if None.
    ///
    /// # Returns
    /// * `Ok(())` - Successfully saved to `.socratic-shell-review.json`
    /// * `Err(std::io::Error)` - File write or serialization error
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, directory: Option<P>) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)?;
        let file_path = match directory {
            Some(dir) => dir.as_ref().join(".socratic-shell-review.json"),
            None => std::path::PathBuf::from(".socratic-shell-review.json"),
        };
        std::fs::write(file_path, json)?;
        Ok(())
    }

    /// Load review state from the JSON persistence file.
    ///
    /// # Arguments
    /// * `directory` - Optional directory where to look for the file. Uses current directory if None.
    ///
    /// # Returns
    /// * `Ok(ReviewState)` - Successfully loaded from `.socratic-shell-review.json`
    /// * `Err(Box<dyn std::error::Error>)` - File not found, read error, or parse error
    pub fn load_from_file<P: AsRef<std::path::Path>>(directory: Option<P>) -> Result<Self, Box<dyn std::error::Error>> {
        let file_path = match directory {
            Some(dir) => dir.as_ref().join(".socratic-shell-review.json"),
            None => std::path::PathBuf::from(".socratic-shell-review.json"),
        };
        let content = std::fs::read_to_string(file_path)?;
        let review: ReviewState = serde_json::from_str(&content)?;
        Ok(review)
    }
}
