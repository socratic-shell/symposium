use symposium_mcp::git::{GitService, CommentParser};
use test_utils::TestRepo;

#[test]
fn test_comment_parsing_from_real_git_diffs() {
    // Create a test repository with commits and AI comments
    let temp_dir = TestRepo::new()
        // Initial commit with basic file
        .overwrite_and_add("src/auth.rs", r#"
pub fn authenticate(token: &str) -> bool {
    token == "valid"
}
"#)
        .commit("Initial auth module")
        
        // Add file with AI comments (some added, some context)
        .overwrite_and_add("src/auth.rs", r#"
pub fn authenticate(token: &str) -> bool {
    // 💡 Using simple string comparison for demo - in production would use JWT validation
    if token.is_empty() {
        return false;
    }
    
    // ❓ Should we add rate limiting here to prevent brute force attacks?
    token == "valid"
}

pub fn validate_session(session_id: &str) -> bool {
    // TODO: Implement proper session validation with database lookup
    session_id.len() > 10
}
"#)
        .commit("Add AI insights to auth module")
        
        // Add new file with comments (all added lines)
        .overwrite_and_add("src/payment.rs", r#"
// 💡 Using Stripe API for PCI compliance instead of handling cards directly
pub fn process_payment(amount: u64) -> Result<String, PaymentError> {
    // ❓ What should be the timeout for payment processing?
    stripe::charge(amount)
}
"#)
        .commit("Add payment module with AI insights")
        
        // Modify existing file - remove some comments, add others
        .overwrite("src/auth.rs", r#"
pub fn authenticate(token: &str) -> bool {
    // 💡 Updated comment that should appear (context line)
    if token.is_empty() {
        return false;
    }
    
    // 🔧 New comment on added line
    token == "valid_token"  // Changed validation
}

pub fn validate_session(session_id: &str) -> bool {
    // 💡 This comment should appear (added line)
    session_id.len() > 15  // Increased requirement
}
"#)
        .create();

    let repo_path = temp_dir.path().to_str().unwrap();
    let git_service = GitService::new(repo_path).unwrap();
    let comment_parser = CommentParser::new();

    // Test 1: Comments from payment.rs addition (latest commit)
    let (base_oid, head_oid) = git_service.parse_commit_range("HEAD~1..HEAD").unwrap();
    let file_changes = git_service.generate_diff(base_oid, head_oid).unwrap();
    let comment_threads = comment_parser.parse_file_changes(&file_changes).unwrap();

    // Should find comments from payment.rs
    assert!(comment_threads.len() >= 2, "Should find at least 2 comments from payment.rs");
    
    // Verify we have comments from payment.rs
    let payment_comments: Vec<_> = comment_threads.iter()
        .filter(|t| t.file_path.contains("payment.rs"))
        .collect();
    
    assert!(!payment_comments.is_empty(), "Should have comments from payment.rs");

    // Test 2: Comments from auth.rs enhancement (HEAD~2..HEAD~1)
    let (base_oid, head_oid) = git_service.parse_commit_range("HEAD~2..HEAD~1").unwrap();
    let file_changes = git_service.generate_diff(base_oid, head_oid).unwrap();
    let comment_threads = comment_parser.parse_file_changes(&file_changes).unwrap();

    // Should find the AI comments that were added to auth.rs
    assert!(comment_threads.len() >= 2, "Should find AI comments from auth enhancement");
    
    let comment_contents: Vec<&str> = comment_threads.iter()
        .map(|t| t.content.as_str())
        .collect();
    
    assert!(comment_contents.iter().any(|c| c.contains("JWT validation")), 
            "Should find JWT validation comment");
    assert!(comment_contents.iter().any(|c| c.contains("rate limiting")), 
            "Should find rate limiting comment");
}

#[test]
fn test_removed_lines_ignored() {
    // Create repo where we remove lines with comments
    let temp_dir = TestRepo::new()
        .overwrite_and_add("test.rs", r#"
// 💡 This comment will be removed
fn old_function() {}

// 💡 This comment will stay
fn kept_function() {}
"#)
        .commit("Initial with comments")
        
        .overwrite_and_add("test.rs", r#"
// 💡 This comment will stay
fn kept_function() {}

// 💡 New comment on added line
fn new_function() {}
"#)
        .commit("Remove old function, add new one")
        .create();

    let repo_path = temp_dir.path().to_str().unwrap();
    let git_service = GitService::new(repo_path).unwrap();
    let comment_parser = CommentParser::new();

    let (base_oid, head_oid) = git_service.parse_commit_range("HEAD~1..HEAD").unwrap();
    let file_changes = git_service.generate_diff(base_oid, head_oid).unwrap();
    let comment_threads = comment_parser.parse_file_changes(&file_changes).unwrap();

    // Should only find comments from kept context lines and new added lines
    // Should NOT find the "will be removed" comment
    let comment_contents: Vec<&str> = comment_threads.iter()
        .map(|t| t.content.as_str())
        .collect();
    
    assert!(!comment_contents.iter().any(|c| c.contains("will be removed")), 
            "Should not find comments from removed lines");
    assert!(comment_contents.iter().any(|c| c.contains("will stay")), 
            "Should find comments from context lines");
    assert!(comment_contents.iter().any(|c| c.contains("added line")), 
            "Should find comments from added lines");
}
