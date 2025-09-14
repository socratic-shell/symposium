use git2::{Delta, DiffOptions, Oid, Repository};
use schemars::JsonSchema;

/// Git service for repository operations.
///
/// Provides Git repository analysis capabilities including commit parsing,
/// diff generation, and file change detection.
pub struct GitService {
    repo: Repository,
}

/// Represents the status of a file change in a diff
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
}

/// Represents a single line in a diff hunk
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub old_line_number: Option<usize>,
    pub new_line_number: Option<usize>,
}

/// Type of diff line (added, removed, or context)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
pub enum DiffLineType {
    Added,
    Removed,
    Context,
}

/// Represents a hunk (contiguous block of changes) in a diff
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct DiffHunk {
    pub header: String,
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub lines: Vec<DiffLine>,
}

/// Represents a single file change in a diff
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct FileChange {
    pub path: String,
    pub status: ChangeStatus,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<DiffHunk>,
}

impl GitService {
    /// Create a new GitService instance for the specified repository path.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the Git repository directory
    ///
    /// # Returns
    /// * `Ok(GitService)` - Successfully initialized service
    /// * `Err(git2::Error)` - Repository not found or invalid
    pub fn new(repo_path: &str) -> Result<Self, git2::Error> {
        let repo = Repository::open(repo_path)?;
        Ok(GitService { repo })
    }

    /// Parse a commit range string into base and head OIDs.
    ///
    /// Supports various Git commit range formats:
    /// - "HEAD" - Compare HEAD with working tree
    /// - "HEAD~2" - Compare HEAD~2 with working tree  
    /// - "abc123..def456" - Compare two specific commits
    ///
    /// # Arguments
    /// * `range` - Git commit range specification
    ///
    /// # Returns
    /// * `Ok((base_oid, head_oid))` - Parsed commit OIDs (head_oid is None for working tree)
    /// * `Err(git2::Error)` - Invalid range or commit not found
    pub fn parse_commit_range(&self, range: &str) -> Result<(Oid, Option<Oid>), git2::Error> {
        if range.contains("..") {
            // Range format: base..head
            let parts: Vec<&str> = range.split("..").collect();
            if parts.len() != 2 {
                return Err(git2::Error::from_str("Invalid range format"));
            }

            let base_oid = self.repo.revparse_single(parts[0])?.id();
            let head_oid = self.repo.revparse_single(parts[1])?.id();
            Ok((base_oid, Some(head_oid)))
        } else {
            // Single commit: compare with working tree
            let base_oid = self.repo.revparse_single(range)?.id();
            Ok((base_oid, None))
        }
    }

    /// Generate diff with file-level statistics between two commits or HEAD and working tree.
    ///
    /// # Arguments
    /// * `base_oid` - Base commit for comparison
    /// * `head_oid` - Head commit, or None to compare with working tree
    ///
    /// # Returns
    /// * `Ok(Vec<FileChange>)` - List of files with change statistics and hunks field (currently None)
    /// * `Err(git2::Error)` - Git operation failed
    pub fn generate_diff(
        &self,
        base_oid: Oid,
        head_oid: Option<Oid>,
    ) -> Result<Vec<FileChange>, git2::Error> {
        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(true);
        diff_opts.context_lines(3);

        let diff = match head_oid {
            Some(head_oid) => {
                // Compare two commits
                let base_tree = self.repo.find_commit(base_oid)?.tree()?;
                let head_tree = self.repo.find_commit(head_oid)?.tree()?;
                self.repo.diff_tree_to_tree(
                    Some(&base_tree),
                    Some(&head_tree),
                    Some(&mut diff_opts),
                )?
            }
            None => {
                // Compare HEAD with working tree
                let head_tree = self.repo.find_commit(base_oid)?.tree()?;
                self.repo
                    .diff_tree_to_workdir(Some(&head_tree), Some(&mut diff_opts))?
            }
        };

        use std::cell::RefCell;

        // Use RefCell for interior mutability since all closures are captured simultaneously
        let file_changes = RefCell::new(Vec::<FileChange>::new());

        diff.foreach(
            &mut |delta, _progress| {
                let (path, status) = match (delta.old_file().path(), delta.new_file().path()) {
                    (Some(_old_path), Some(new_path)) => (
                        new_path.to_string_lossy().to_string(),
                        match delta.status() {
                            Delta::Added => ChangeStatus::Added,
                            Delta::Deleted => ChangeStatus::Deleted,
                            Delta::Modified => ChangeStatus::Modified,
                            Delta::Renamed => ChangeStatus::Modified,
                            Delta::Copied => ChangeStatus::Added,
                            _ => ChangeStatus::Modified,
                        },
                    ),
                    (None, Some(new_path)) => {
                        (new_path.to_string_lossy().to_string(), ChangeStatus::Added)
                    }
                    (Some(old_path), None) => (
                        old_path.to_string_lossy().to_string(),
                        ChangeStatus::Deleted,
                    ),
                    (None, None) => return true,
                };

                file_changes.borrow_mut().push(FileChange {
                    path,
                    status,
                    additions: 0,
                    deletions: 0,
                    hunks: Vec::new(),
                });

                true
            },
            None, // No binary callback
            Some(&mut |_delta, hunk| {
                // Called once per hunk - finalize previous hunk, start new one
                let mut file_changes = file_changes.borrow_mut();
                let current_file = file_changes.last_mut().unwrap();

                let header = String::from_utf8_lossy(hunk.header()).trim().to_string();
                current_file.hunks.push(DiffHunk {
                    header,
                    old_start: hunk.old_start() as usize,
                    old_lines: hunk.old_lines() as usize,
                    new_start: hunk.new_start() as usize,
                    new_lines: hunk.new_lines() as usize,
                    lines: Vec::new(),
                });
                true
            }),
            Some(&mut |_delta, _hunk, line| {
                let mut file_changes = file_changes.borrow_mut();
                let current_file = file_changes.last_mut().unwrap();
                let current_hunk = current_file.hunks.last_mut().unwrap();

                // Called once per line - add to current hunk
                let line_type = match line.origin() {
                    '+' => DiffLineType::Added,
                    '-' => DiffLineType::Removed,
                    ' ' => DiffLineType::Context,
                    _ => DiffLineType::Context,
                };

                let content = String::from_utf8_lossy(line.content())
                    .trim_end()
                    .to_string();

                let (old_line_number, new_line_number) = match line_type {
                    DiffLineType::Added => (None, Some(line.new_lineno().unwrap_or(0) as usize)),
                    DiffLineType::Removed => (Some(line.old_lineno().unwrap_or(0) as usize), None),
                    DiffLineType::Context => (
                        line.old_lineno().map(|n| n as usize),
                        line.new_lineno().map(|n| n as usize),
                    ),
                };

                // Update file statistics
                match line_type {
                    DiffLineType::Added => current_file.additions += 1,
                    DiffLineType::Removed => current_file.deletions += 1,
                    DiffLineType::Context => (),
                }

                // Add line to current hunk
                current_hunk.lines.push(DiffLine {
                    line_type,
                    content,
                    old_line_number,
                    new_line_number,
                });

                true
            }),
        )?;

        Ok(file_changes.into_inner())
    }
}
