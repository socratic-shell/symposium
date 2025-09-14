use std::path::PathBuf;

/// Get the current working directory
pub fn current_dir() -> std::io::Result<PathBuf> {
    std::env::current_dir()
}
