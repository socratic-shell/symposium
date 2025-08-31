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
    pub files_changed: Vec<FileChange>,
    pub comment_threads: Vec<CommentThread>,
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

/// Type of line in a diff
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum DiffLineType {
    /// Added line
    Added,
    /// Removed line
    Removed,
    /// Context line (unchanged)
    Context,
}

/// Represents a single line in a diff hunk
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiffLine {
    /// Type of line: Added, Removed, or Context
    pub line_type: DiffLineType,
    /// Content of the line
    pub content: String,
    /// Line number in the old file (None for added lines)
    pub old_line_number: Option<usize>,
    /// Line number in the new file (None for removed lines)
    pub new_line_number: Option<usize>,
}

/// Represents a diff hunk (contiguous block of changes)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiffHunk {
    /// Header line (e.g., "@@ -10,6 +10,8 @@")
    pub header: String,
    /// Starting line number in old file
    pub old_start: usize,
    /// Number of lines in old file
    pub old_lines: usize,
    /// Starting line number in new file
    pub new_start: usize,
    /// Number of lines in new file
    pub new_lines: usize,
    /// Individual lines in this hunk
    pub lines: Vec<DiffLine>,
}

/// Represents a single file change in a synthetic pull request.
///
/// Contains file path, change type, line-level statistics, and detailed diff hunks
/// for display in PR interfaces and change summaries.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileChange {
    pub path: String,
    pub status: ChangeStatus,
    pub additions: u32,
    pub deletions: u32,
    /// Diff hunks containing line-by-line changes
    pub hunks: Vec<DiffHunk>,
}

/// Type of change made to a file in the Git diff.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
}

/// A comment thread attached to a specific line in a file.
///
/// Represents an AI insight comment with its location, type, and any user responses.
/// Forms the basis for PR-style line-by-line commenting interfaces.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommentThread {
    pub thread_id: String,
    pub file_path: String,
    pub line_number: u32,
    pub comment_type: CommentType,
    pub content: String,
    pub responses: Vec<UserResponse>,
}

/// Type of AI insight comment for categorization and display.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum CommentType {
    /// 💡 Explanations of implementation decisions and design choices
    Explanation,
    /// ❓ Questions seeking feedback or clarification from reviewers  
    Question,
    /// TODO items for future work or improvements
    Todo,
    /// FIXME items indicating known issues that need addressing
    Fixme,
}

/// A user response to an AI insight comment in a review thread.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserResponse {
    pub author: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Intermediate representation of a parsed AI insight comment.
///
/// Used during comment extraction before creating full CommentThread objects.
#[derive(Debug, Clone)]
pub struct ParsedComment {
    pub comment_type: CommentType,
    pub content: String,
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
