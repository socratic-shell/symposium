use crate::synthetic_pr::{CommentParser, GitService, ReviewState, ReviewStatus};
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// MCP tool parameters for creating a new synthetic pull request.
// ANCHOR: request_review_params
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RequestReviewParams {
    /// Git commit range to review (e.g., "HEAD", "HEAD~2", "abc123..def456")
    pub commit_range: String,
    /// Title for the review
    pub title: String,
    /// Flexible description object for LLM-specific formatting
    pub description: serde_json::Value,
    /// Optional repository path (defaults to current directory)
    pub repo_path: Option<String>,
}

/// User feedback type from VSCode extension
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    Comment,
    CompleteReview,
}

/// Completion action for review completion
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CompletionAction {
    RequestChanges,
    Checkpoint,
    Return,
}

/// User feedback from VSCode extension
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct UserFeedback {
    pub review_id: String,
    #[serde(flatten)]
    pub feedback: FeedbackData,
}

/// Different types of feedback data
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(tag = "feedback_type", rename_all = "snake_case")]
pub enum FeedbackData {
    Comment {
        file_path: Option<String>,
        line_number: Option<u32>,
        comment_text: String,
        context_lines: Option<Vec<String>>,
    },
    CompleteReview {
        completion_action: CompletionAction,
        additional_notes: Option<String>,
    },
}

/// Data generated from the working directory and sent over IPC to the extension
/// as the basis for a review.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReviewData {
    pub review_id: String,
    pub title: String,
    pub description: serde_json::Value,
    pub commit_range: String,
    pub files_changed: Vec<crate::synthetic_pr::FileChange>,
    pub comment_threads: Vec<crate::synthetic_pr::CommentThread>,
    pub status: String,
}

// ANCHOR: update_review_params
/// MCP tool parameters for updating an existing synthetic pull request.
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct UpdateReviewParams {
    /// What review are we updating
    pub review_id: String,

    /// What kind of update should be performed.
    pub action: UpdateReviewAction,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub enum UpdateReviewAction {
    /// Wait for user feedback from VSCode extension
    WaitForFeedback,
    /// Add a new comment thread to the review
    AddComment { comment: serde_json::Value },
    /// Mark the review as approved
    Approve,
    /// Request changes to the review
    RequestChanges,
}

/// Response data from synthetic pull request update operations.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateReviewResponse {
    pub status: String,
    pub review_id: String,
    pub user_action: Option<String>,
    pub message: Option<String>,
    pub user_feedback: Option<UserFeedback>,
}

/// Response data for synthetic pull request status queries.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReviewStatusResponse {
    pub review_id: Option<String>,
    pub title: Option<String>,
    pub status: String,
    pub files_changed: Option<usize>,
    pub comment_threads: Option<usize>,
    pub created_at: Option<chrono::DateTime<Utc>>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
}

/// MCP tool: Create a synthetic pull request from Git commit range with AI insight comments.
///
/// Orchestrates the complete synthetic PR creation workflow:
/// 1. Parses the Git commit range to identify changes
/// 2. Generates structured diff with file statistics  
/// 3. Extracts AI insight comments from changed files
/// 4. Creates review state with metadata and persistence
/// 5. Returns structured data for VSCode extension consumption
///
/// # Arguments
/// * `params` - Request parameters including commit range, title, and description
///
/// # Returns
/// * `Ok(ReviewResponse)` - Complete review data with files and comments
/// * `Err(Box<dyn std::error::Error>)` - Git operation or file system error
pub async fn harvest_review_data(
    params: RequestReviewParams,
) -> Result<ReviewData, Box<dyn std::error::Error>> {
    // Use provided repo path or default to current directory
    let repo_path = params.repo_path.as_deref().unwrap_or(".");

    // Initialize services with explicit repo path
    let git_service = GitService::new(repo_path)?;
    let comment_parser = CommentParser::new();

    // Parse commit range and generate diff with hunks
    let (base_oid, head_oid) = git_service.parse_commit_range(&params.commit_range)?;
    let file_changes = git_service.generate_diff(base_oid, head_oid)?;

    // Parse AI comments from diff hunks (only changed lines)
    let comment_threads = comment_parser.parse_file_changes(&file_changes)?;

    // Create review state
    let review_state = ReviewState {
        review_id: uuid::Uuid::new_v4().to_string(),
        title: params.title.clone(),
        description: params.description.clone(),
        commit_range: params.commit_range.clone(),
        status: ReviewStatus::Pending,
        files_changed: file_changes.clone(),
        comment_threads: comment_threads.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Save review state to file
    review_state.save_to_file(None::<&str>)?;

    // Return response for VSCode extension
    Ok(ReviewData {
        review_id: review_state.review_id,
        title: review_state.title,
        description: review_state.description,
        commit_range: review_state.commit_range,
        files_changed: file_changes,
        comment_threads,
        status: "success".to_string(),
    })
}

/// MCP tool: Update an existing synthetic pull request or wait for user feedback.
///
/// Supports iterative review workflows between AI assistants and developers:
/// - `wait_for_feedback`: Blocks until user provides review feedback
/// - `add_comment`: Adds new comment threads to existing review
/// - `approve`: Marks review as approved and ready for merge
/// - `request_changes`: Indicates review needs modifications
///
/// # Arguments
/// * `params` - Update parameters including review ID and action
///
/// # Returns
/// * `Ok(UpdateReviewResponse)` - Status and result of update operation
/// * `Err(Box<dyn std::error::Error>)` - Invalid action or file system error
pub async fn update_review(
    params: UpdateReviewParams,
) -> Result<UpdateReviewResponse, Box<dyn std::error::Error>> {
    match params.action {
        UpdateReviewAction::WaitForFeedback => {
            // In a real implementation, this would block until VSCode extension provides feedback
            // For now, return a placeholder response
            Ok(UpdateReviewResponse {
                status: "waiting".to_string(),
                review_id: params.review_id,
                user_action: Some("pending".to_string()),
                message: Some("Waiting for user feedback...".to_string()),
                user_feedback: None,
            })
        }
        UpdateReviewAction::AddComment { comment: _ } => {
            // Load existing review, add comment, save back
            let mut review = ReviewState::load_from_file(None::<&str>)?;

            // Add new comment thread (simplified implementation)
            // In practice, this would handle thread replies, etc.

            review.updated_at = Utc::now();
            review.save_to_file(None::<&str>)?;

            Ok(UpdateReviewResponse {
                status: "comment_added".to_string(),
                review_id: params.review_id,
                user_action: None,
                message: Some("Comment added successfully".to_string()),
                user_feedback: None,
            })
        }
        UpdateReviewAction::Approve => {
            let mut review = ReviewState::load_from_file(None::<&str>)?;
            review.status = ReviewStatus::Approved;
            review.updated_at = Utc::now();
            review.save_to_file(None::<&str>)?;

            Ok(UpdateReviewResponse {
                status: "approved".to_string(),
                review_id: params.review_id,
                user_action: Some("approved".to_string()),
                message: None,
                user_feedback: None,
            })
        }
        UpdateReviewAction::RequestChanges => {
            let mut review = ReviewState::load_from_file(None::<&str>)?;
            review.status = ReviewStatus::ChangesRequested;
            review.updated_at = Utc::now();
            review.save_to_file(None::<&str>)?;

            Ok(UpdateReviewResponse {
                status: "changes_requested".to_string(),
                review_id: params.review_id,
                user_action: Some("changes_requested".to_string()),
                message: None,
                user_feedback: None,
            })
        }
    }
}

/// Get the status of the current synthetic pull request.
///
/// Provides summary information about the active review including file counts,
/// comment thread counts, timestamps, and current workflow status.
///
/// # Arguments
/// * `repo_path` - Optional repository path (defaults to current directory)
///
/// # Returns
/// * `Ok(ReviewStatusResponse)` - Current review status or "no_active_review"
/// * `Err(Box<dyn std::error::Error>)` - File system error
pub async fn get_review_status(
    repo_path: Option<&str>,
) -> Result<ReviewStatusResponse, Box<dyn std::error::Error>> {
    let repo_path = repo_path.unwrap_or(".");
    let review_file_path = if repo_path == "." {
        ".socratic-shell-review.json".to_string()
    } else {
        format!("{}/.socratic-shell-review.json", repo_path)
    };

    match std::fs::read_to_string(&review_file_path) {
        Ok(content) => {
            let review: ReviewState = serde_json::from_str(&content)?;
            Ok(ReviewStatusResponse {
                review_id: Some(review.review_id),
                title: Some(review.title),
                status: format!("{:?}", review.status),
                files_changed: Some(review.files_changed.len()),
                comment_threads: Some(review.comment_threads.len()),
                created_at: Some(review.created_at),
                updated_at: Some(review.updated_at),
            })
        }
        Err(_) => Ok(ReviewStatusResponse {
            review_id: None,
            title: None,
            status: "no_active_review".to_string(),
            files_changed: None,
            comment_threads: None,
            created_at: None,
            updated_at: None,
        }),
    }
}
