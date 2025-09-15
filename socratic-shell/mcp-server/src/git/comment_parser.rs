use regex::Regex;
use schemars::JsonSchema;
use crate::git::{FileChange, DiffLineType};

/// Type of AI insight comment found in source code
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
pub enum CommentType {
    Explanation,
    Question,
    Todo,
    Fixme,
}

/// Parsed AI insight comment with type and content
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct ParsedComment {
    pub comment_type: CommentType,
    pub content: String,
}

/// Comment thread representing a discussion around a specific line of code
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct CommentThread {
    pub thread_id: String,
    pub file_path: String,
    pub line_number: u32,
    pub comment_type: CommentType,
    pub content: String,
    pub responses: Vec<String>,
}

/// Parses AI insight comments from source code files.
///
/// Extracts structured comments that provide context and explanations for code changes:
/// - 💡 Explanations: Why code was implemented a certain way
/// - ❓ Questions: Areas where the AI seeks feedback or clarification  
/// - TODO: Future work items identified during implementation
/// - FIXME: Known issues that need addressing
///
/// Supports multiple comment syntaxes (// # <!-- -->) for cross-language compatibility.
pub struct CommentParser {
    lightbulb_regex: Regex,
    question_regex: Regex,
    todo_regex: Regex,
    fixme_regex: Regex,
}

impl CommentParser {
    /// Creates a new CommentParser with pre-compiled regex patterns.
    ///
    /// Initializes regex patterns for detecting AI insight comments across
    /// multiple programming languages and comment syntaxes.
    pub fn new() -> Self {
        Self {
            // Match various comment styles: //, #, <!-- -->, etc.
            lightbulb_regex: Regex::new(r"(?://|#|<!--)\s*💡\s*(.+?)(?:-->)?$").unwrap(),
            question_regex: Regex::new(r"(?://|#|<!--)\s*❓\s*(.+?)(?:-->)?$").unwrap(),
            todo_regex: Regex::new(r"(?://|#|<!--)\s*TODO:\s*(.+?)(?:-->)?$").unwrap(),
            fixme_regex: Regex::new(r"(?://|#|<!--)\s*FIXME:\s*(.+?)(?:-->)?$").unwrap(),
        }
    }

    /// Parse all AI insight comments from a single source file.
    ///
    /// Scans each line for AI insight comment patterns and creates structured
    /// comment threads with file location and content.
    ///
    /// # Arguments
    /// * `file_path` - Path to the source file to analyze
    ///
    /// # Returns
    /// * `Ok(Vec<CommentThread>)` - List of comment threads found
    /// * `Err(std::io::Error)` - File read error
    pub fn parse_file(&self, file_path: &str) -> Result<Vec<CommentThread>, std::io::Error> {
        let content = std::fs::read_to_string(file_path)?;
        let mut threads = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(comment) = self.extract_comment(line) {
                threads.push(CommentThread {
                    thread_id: uuid::Uuid::new_v4().to_string(),
                    file_path: file_path.to_string(),
                    line_number: line_num as u32 + 1, // 1-indexed
                    comment_type: comment.comment_type,
                    content: comment.content,
                    responses: vec![],
                });
            }
        }

        Ok(threads)
    }

    /// Parse AI comments from multiple files with error resilience.
    ///
    /// Processes a list of file paths, continuing even if individual files fail.
    /// Automatically skips deleted files and provides warnings for parse errors.
    ///
    /// # Arguments
    /// * `file_paths` - List of file paths to analyze (typically from Git diff)
    ///
    /// # Returns
    /// * `Ok(Vec<CommentThread>)` - Combined comment threads from all files
    /// * `Err(std::io::Error)` - Critical error preventing processing
    pub fn parse_files(&self, file_paths: &[String]) -> Result<Vec<CommentThread>, std::io::Error> {
        let mut all_threads = Vec::new();

        for file_path in file_paths {
            // Skip deleted files and binary files
            if std::path::Path::new(file_path).exists() {
                match self.parse_file(file_path) {
                    Ok(mut threads) => all_threads.append(&mut threads),
                    Err(e) => {
                        // Log error but continue with other files
                        eprintln!("Warning: Failed to parse comments in {}: {}", file_path, e);
                    }
                }
            }
        }

        Ok(all_threads)
    }

    /// Extract a single AI insight comment from a line of source code.
    ///
    /// Matches against pre-compiled regex patterns for different comment types
    /// and extracts the comment content while preserving type information.
    ///
    /// # Arguments
    /// * `line` - Single line of source code to analyze
    ///
    /// # Returns
    /// * `Some(ParsedComment)` - AI insight comment found with type and content
    /// * `None` - No AI insight comment detected on this line
    fn extract_comment(&self, line: &str) -> Option<ParsedComment> {
        if let Some(caps) = self.lightbulb_regex.captures(line) {
            Some(ParsedComment {
                comment_type: CommentType::Explanation,
                content: caps[1].trim().to_string(),
            })
        } else if let Some(caps) = self.question_regex.captures(line) {
            Some(ParsedComment {
                comment_type: CommentType::Question,
                content: caps[1].trim().to_string(),
            })
        } else if let Some(caps) = self.todo_regex.captures(line) {
            Some(ParsedComment {
                comment_type: CommentType::Todo,
                content: caps[1].trim().to_string(),
            })
        } else if let Some(caps) = self.fixme_regex.captures(line) {
            Some(ParsedComment {
                comment_type: CommentType::Fixme,
                content: caps[1].trim().to_string(),
            })
        } else {
            None
        }
    }

    /// Parse AI insight comments from FileChange structures with diff hunks
    ///
    /// Only extracts comments from lines that were actually changed (added or context),
    /// making synthetic PRs focused on the specific modifications.
    ///
    /// # Arguments
    /// * `file_changes` - Array of FileChange with diff hunks
    ///
    /// # Returns
    /// * `Ok(Vec<CommentThread>)` - Comment threads from changed lines only
    /// * `Err(Box<dyn std::error::Error>)` - Error parsing hunks
    pub fn parse_file_changes(&self, file_changes: &[FileChange]) -> Result<Vec<CommentThread>, Box<dyn std::error::Error>> {
        let mut all_threads = Vec::new();
        
        for file_change in file_changes {
            for hunk in &file_change.hunks {
                for line in &hunk.lines {
                    // Only parse added or context lines (not removed lines)
                    if matches!(line.line_type, DiffLineType::Added | DiffLineType::Context) {
                        if let Some(comment) = self.extract_comment(&line.content) {
                            all_threads.push(CommentThread {
                                thread_id: format!("{}:{}", file_change.path, line.new_line_number.unwrap_or(0)),
                                file_path: file_change.path.clone(),
                                line_number: line.new_line_number.unwrap_or(0) as u32,
                                comment_type: comment.comment_type,
                                content: comment.content,
                                responses: vec![],
                            });
                        }
                    }
                }
            }
        }
        
        Ok(all_threads)
    }
}

impl Default for CommentParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_lightbulb_comment() {
        let parser = CommentParser::new();
        
        let comment = parser.extract_comment("// 💡 Using JWT instead of sessions for stateless design");
        assert!(comment.is_some());
        let comment = comment.unwrap();
        assert!(matches!(comment.comment_type, CommentType::Explanation));
        assert_eq!(comment.content, "Using JWT instead of sessions for stateless design");
    }

    #[test]
    fn test_extract_question_comment() {
        let parser = CommentParser::new();
        
        let comment = parser.extract_comment("# ❓ Should we add rate limiting here?");
        assert!(comment.is_some());
        let comment = comment.unwrap();
        assert!(matches!(comment.comment_type, CommentType::Question));
        assert_eq!(comment.content, "Should we add rate limiting here?");
    }

    #[test]
    fn test_extract_todo_comment() {
        let parser = CommentParser::new();
        
        let comment = parser.extract_comment("// TODO: Add error handling for invalid tokens");
        assert!(comment.is_some());
        let comment = comment.unwrap();
        assert!(matches!(comment.comment_type, CommentType::Todo));
        assert_eq!(comment.content, "Add error handling for invalid tokens");
    }

    #[test]
    fn test_no_comment() {
        let parser = CommentParser::new();
        
        let comment = parser.extract_comment("let x = 42; // Regular comment");
        assert!(comment.is_none());
    }
}
