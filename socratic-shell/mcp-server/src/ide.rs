use std::{future::Future, pin::Pin};

use pulldown_cmark::Event;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::dialect::{DialectFunction, DialectInterpreter};

pub mod ambiguity;
pub mod test;

// IPC client trait that the userdata must implement
pub trait IpcClient: Send {
    async fn resolve_symbol_by_name(&mut self, name: &str) -> anyhow::Result<Vec<SymbolDef>>;
    async fn find_all_references(&mut self, symbol: &SymbolDef) -> anyhow::Result<Vec<FileRange>>;
    fn generate_uuid(&self) -> String;
}

/// The "symbols" file is used as the expected argument
/// for a number of other functions. It is intentionally
/// flexible to enable LLM shorthands -- it can receive
/// a string, an array with other symbols, or an explicit
/// symbol definition. In all cases the [`Symbols::resolve`][]
/// will canonicalize to a list of [`SymbolDef`][] structures.
///
/// Note that `Symbols` is not actually a [`DialectFunction`][].
/// It is only intended for use as the value of a *function argument*
/// -- it doesn't have a canonical function format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Symbols {
    Name(String),
    Array(Vec<Symbols>),
    SymbolDef(SymbolDef),
}

// Symbol implementation
impl Symbols {
    pub fn resolve<U: IpcClient>(
        &self,
        interpreter: &mut DialectInterpreter<U>,
    ) -> Pin<Box<impl Future<Output = anyhow::Result<Vec<SymbolDef>>>>> {
        Box::pin(async move {
            match self {
                Symbols::Name(name) => {
                    // Call IPC: resolve-symbol-by-name (using Deref to access userdata directly)
                    interpreter.resolve_symbol_by_name(name).await
                }

                Symbols::Array(symbols) => {
                    let mut output = vec![];
                    for s in symbols {
                        output.extend(s.resolve(interpreter).await?);
                    }
                    Ok(output)
                }

                Symbols::SymbolDef(symbol_def) => Ok(vec![symbol_def.clone()]),
            }
        })
    }
}

/// A symbol definition representing where a symbol is defined.
///
/// Corresponds loosely to LSP SymbolInformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDef {
    /// The symbol name (e.g., "User", "validateToken")
    pub name: String,

    /// The "kind" of symbol (this is a string that the LLM hopefully knows how to interpret)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    /// Location where this symbol is defined
    #[serde(rename = "definedAt")]
    pub defined_at: FileRange,
}

crate::dialect_value!(SymbolDef {
    name,
    kind,
    defined_at
});

/// A *reference* to a symbol -- includes the information about the symbol itself.
/// A [`SymbolRef`][] can therefore be seen as a subtype of [`SymbolDef`][].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRef {
    /// Symbol being referenced
    #[serde(flatten)]
    pub definition: SymbolDef,

    /// Location where this symbol is referenced from
    #[serde(rename = "referencedAt")]
    pub referenced_at: FileRange,
}

crate::dialect_value!(SymbolRef {
    name,
    kind,
    defined_at,
    referenced_at
});

/// Represents a range of bytes in a file (or URI, etc).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lines {
    /// File path, relative to workspace root
    pub path: String,

    /// Start line of range (always <= end)
    pub start: usize,

    /// End line of range (always >= start)
    pub end: usize,
}

impl<U: Send> DialectFunction<U> for Lines {
    type Output = FileRange;

    const PARAMETER_ORDER: &'static [&'static str] = &["path", "start", "end"];

    async fn execute(
        self,
        _interpreter: &mut DialectInterpreter<U>,
    ) -> anyhow::Result<Self::Output> {
        let Lines { path, start, end } = self;

        // Find the length of the end line.
        let content = std::fs::read_to_string(&path)?;
        let lines = content
            .lines()
            .skip(start - 1)
            .take(end - start + 1)
            .collect::<Vec<_>>();

        let last_column = match lines.last() {
            Some(l) => l.len(),
            None => 0,
        };

        Ok(FileRange {
            path,
            start: FileLocation {
                line: start as u32,
                column: 1,
            },
            end: FileLocation {
                line: end as u32,
                column: last_column as u32,
            },
            content: Some(lines.join("\n")),
        })
    }
}

/// Represents a range of bytes in a file (or URI, etc).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRange {
    /// File path, relative to workspace root
    pub path: String,

    /// Start of range (always <= end)
    pub start: FileLocation,

    /// End of range (always >= start)
    pub end: FileLocation,

    /// Enclosing text (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

crate::dialect_value!(FileRange {
    path,
    start,
    end,
    content
});

/// A line/colum index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLocation {
    /// Line number (1-based)
    pub line: u32,

    /// Column number (1-based)
    pub column: u32,
}

crate::dialect_value!(FileLocation { line, column });

// IDE Functions
#[derive(Deserialize)]
pub struct FindDefinitions {
    of: Symbols,
}

impl<U: IpcClient> DialectFunction<U> for FindDefinitions {
    type Output = Vec<SymbolDef>;

    const PARAMETER_ORDER: &'static [&'static str] = &["of"];

    async fn execute(
        self,
        interpreter: &mut DialectInterpreter<U>,
    ) -> anyhow::Result<Self::Output> {
        self.of.resolve(interpreter).await
    }
}

#[derive(Deserialize)]
pub struct FindReferences {
    pub to: Symbols,
}

impl<U: IpcClient> DialectFunction<U> for FindReferences {
    type Output = Vec<SymbolRef>;

    const PARAMETER_ORDER: &'static [&'static str] = &["to"];

    async fn execute(
        self,
        interpreter: &mut DialectInterpreter<U>,
    ) -> anyhow::Result<Self::Output> {
        let definitions = self.to.resolve(interpreter).await?;
        let mut output = vec![];
        for definition in definitions {
            let locations = interpreter.find_all_references(&definition).await?;
            output.extend(locations.into_iter().map(|loc| SymbolRef {
                definition: definition.clone(),
                referenced_at: loc,
            }));
        }
        Ok(output)
    }
}

/// Search for regex patterns in files, respecting gitignore rules.
///
/// Examples:
/// - `{"search": {"path": "src/auth.rs", "regex": "fn\\s+\\w+"}}` - Find functions in specific file
/// - `{"search": {"path": "src/", "regex": "TODO|FIXME", "extension": ".rs"}}` - Find todos in Rust files
/// - `{"search": {"path": ".", "regex": "struct User\\b", "extension": "rs"}}` - Find User struct in Rust files
#[derive(Deserialize)]
pub struct Search {
    pub path: String,
    pub regex: String,
    pub extension: Option<String>,
}

impl<U: IpcClient> DialectFunction<U> for Search {
    type Output = Vec<FileRange>;

    const PARAMETER_ORDER: &'static [&'static str] = &["path", "regex", "extension"];

    async fn execute(
        self,
        _interpreter: &mut DialectInterpreter<U>,
    ) -> anyhow::Result<Self::Output> {
        use ignore::Walk;
        use regex::Regex;
        use std::path::Path;

        let regex = Regex::new(&self.regex)?;
        let mut results = Vec::new();
        let search_path = Path::new(&self.path);

        // Normalize extension (add dot if missing)
        let extension_filter = self.extension.as_ref().map(|ext| {
            if ext.starts_with('.') {
                ext.clone()
            } else {
                format!(".{}", ext)
            }
        });

        // If it's a specific file, search just that file
        if search_path.is_file() {
            results.extend(process_file(&self.path, &extension_filter, &regex));
        } else if search_path.is_dir() {
            // Directory search with gitignore support
            for result in Walk::new(&self.path) {
                let entry = result?;
                if entry.file_type().map_or(false, |ft| ft.is_file()) {
                    let path_str = entry.path().to_string_lossy().to_string();
                    results.extend(process_file(&path_str, &extension_filter, &regex));
                }
            }
        }
        // If path doesn't exist, just return empty results

        Ok(results)
    }
}

/// Generate git diffs for commit ranges, respecting exclude options.
///
/// Examples:
/// - `{"gitdiff": {"commit_range": "HEAD^.."}}` - Changes in last commit
/// - `{"gitdiff": {"commit_range": "HEAD~3..HEAD~1"}}` - Changes between specific commits  
/// - `{"gitdiff": {"commit_range": "HEAD", "exclude_unstaged": true}}` - Only staged changes
#[derive(Deserialize)]
pub struct GitDiff {
    pub commit_range: String,

    #[expect(dead_code)]
    pub exclude_unstaged: Option<bool>,

    #[expect(dead_code)]
    pub exclude_staged: Option<bool>,
}

impl<U: IpcClient> DialectFunction<U> for GitDiff {
    type Output = GitDiffElement;

    const PARAMETER_ORDER: &'static [&'static str] =
        &["commit_range", "exclude_unstaged", "exclude_staged"];

    async fn execute(
        self,
        _interpreter: &mut DialectInterpreter<U>,
    ) -> anyhow::Result<Self::Output> {
        use crate::git::GitService;

        // Use current directory as repo path (could be made configurable)
        let git_service = GitService::new(".")?;
        let (base_oid, head_oid) = git_service.parse_commit_range(&self.commit_range)?;
        let file_changes = git_service.generate_diff(base_oid, head_oid)?;

        // TODO: Apply exclude filters for staged/unstaged changes
        // For now, return all changes wrapped in GitDiffElement
        Ok(GitDiffElement {
            files: file_changes,
        })
    }
}

/// Create a comment at a specific location with optional icon and content.
///
/// Normalizes different location types (FileRange, SymbolDef, SymbolRef) into FileRange.
///
/// Examples:
/// - `{"comment": {"location": {"path": "src/main.rs", "start": {"line": 10, "column": 1}, "end": {"line": 10, "column": 20}}, "content": ["This needs refactoring"]}}`
/// - `{"comment": {"location": {"search": {"path": "src/", "regex": "fn main"}}, "icon": "warning", "content": ["Entry point"]}}`
#[derive(Deserialize)]
pub struct Comment {
    /// Location for the comment.
    pub location: ResolvedLocation,

    /// Optional icon.
    pub icon: Option<String>,

    /// Optional content.
    ///
    /// FIXME: These should be content elements.
    pub content: Vec<serde_json::Value>, // Will be resolved to ResolvedWalkthroughElement
}

/// We accept either symbols or file ranges.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum ResolvedLocation {
    FileRange(FileRange),
    SearchResults(Vec<FileRange>),
    SymbolDefs(Vec<SymbolDef>),
}

/// Resolved comment output from the [`Comment`] dialect function.
///
/// This is the processed result after normalizing different location types
/// (FileRange, SymbolDef, SymbolRef) into a consistent Vec<FileRange> format.
/// The fully normalized struct that we send over IPC.
#[derive(Serialize, Deserialize, Debug)]
pub struct ResolvedComment {
    pub id: String,
    pub locations: Vec<FileRange>,
    pub icon: Option<String>,
    pub comment: Vec<ResolvedWalkthroughElement>,
}

impl<U: IpcClient> DialectFunction<U> for Comment {
    type Output = ResolvedComment;

    const PARAMETER_ORDER: &'static [&'static str] = &["location", "icon", "content"];

    async fn execute(
        self,
        interpreter: &mut DialectInterpreter<U>,
    ) -> anyhow::Result<Self::Output> {
        // Normalize different location types to a Vec<FileRange>
        let locations = match self.location {
            ResolvedLocation::FileRange(range) => vec![range],
            ResolvedLocation::SymbolDefs(def) => def.iter().map(|d| d.defined_at.clone()).collect(),
            ResolvedLocation::SearchResults(results) => results,
        };

        if locations.is_empty() {
            return Err(anyhow::anyhow!("Location resolved to empty search results"));
        }

        // Process content elements - for now, convert strings to Markdown elements
        // TODO: Execute Dialect programs in content elements
        let mut resolved_content = Vec::new();
        for content_item in self.content {
            match content_item {
                serde_json::Value::String(text) => {
                    resolved_content.push(ResolvedWalkthroughElement::Markdown(
                        ResolvedMarkdownElement { markdown: text },
                    ));
                }
                _ => {
                    // For now, convert other types to string and treat as markdown
                    // TODO: Execute Dialect programs here
                    resolved_content.push(ResolvedWalkthroughElement::Markdown(
                        ResolvedMarkdownElement {
                            markdown: content_item.to_string(),
                        },
                    ));
                }
            }
        }

        Ok(ResolvedComment {
            id: interpreter.user_data().generate_uuid(),
            locations,
            icon: self.icon,
            comment: resolved_content,
        })
    }
}

fn search_file_content(file_path: &str, content: &str, regex: &regex::Regex) -> Vec<FileRange> {
    let mut results = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        if let Some(mat) = regex.find(line) {
            results.push(FileRange {
                path: file_path.to_string(),
                start: FileLocation {
                    line: (line_num + 1) as u32,
                    column: (mat.start() + 1) as u32,
                },
                end: FileLocation {
                    line: (line_num + 1) as u32,
                    column: (mat.end() + 1) as u32,
                },
                content: Some(line.to_string()),
            });
        }
    }
    results
}

fn matches_extension(file_path: &str, extension_filter: &Option<String>) -> bool {
    match extension_filter {
        Some(ext) => file_path.ends_with(ext),
        None => true,
    }
}

fn process_file(
    file_path: &str,
    extension_filter: &Option<String>,
    regex: &regex::Regex,
) -> Vec<FileRange> {
    if matches_extension(file_path, extension_filter) {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            return search_file_content(file_path, &content, regex);
        }
    }
    Vec::new()
}

/// Create an interactive action button for walkthroughs.
///
/// Examples:
/// - `{"action": {"button": "Run Tests"}}`
/// - `{"action": {"button": "Generate", "tell_agent": "Generate user authentication boilerplate"}}`
#[derive(Deserialize)]
pub struct Action {
    /// Button text
    pub button: String,

    /// Optional text to send to agent when clicked
    pub tell_agent: Option<String>,
}

/// Resolved action output from the [`Action`] dialect function.
///
/// This is the processed result with button text and optional agent instructions.
#[derive(Serialize, Deserialize, Debug)]
pub struct ResolvedAction {
    pub button: String,
    pub tell_agent: Option<String>,
}

impl<U: IpcClient> DialectFunction<U> for Action {
    type Output = ResolvedAction;

    const PARAMETER_ORDER: &'static [&'static str] = &["button", "tell_agent"];

    async fn execute(
        self,
        _interpreter: &mut DialectInterpreter<U>,
    ) -> anyhow::Result<Self::Output> {
        // Action is already resolved, just pass through
        Ok(ResolvedAction {
            button: self.button,
            tell_agent: self.tell_agent,
        })
    }
}

/// Resolved walkthrough types for IPC communication with VSCode extension

/// Resolved walkthrough output from the `present_walkthrough` MCP tool.
///
/// Contains HTML content with resolved XML elements and Dialect expressions.
#[derive(Serialize, Debug)]
pub struct ResolvedWalkthrough {
    /// HTML content with resolved XML elements (comment, gitdiff, action, mermaid)
    pub content: String,
    /// Base directory path for resolving relative file references
    pub base_uri: String,
}

/// Resolved markdown element from plain string input in walkthrough sections.
///
/// This represents the processed result when a walkthrough element is a plain string.
/// Markdown content with processed file references converted to dialectic: URLs.
///
/// This type has a custom `Deserialize` implementation that automatically processes
/// markdown during deserialization, converting file references like:
/// - `[text](src/file.ts?pattern)` → `[text](dialectic:src/file.ts?regex=pattern)`
/// - `[text](src/file.ts#L42)` → `[text](dialectic:src/file.ts?line=42)`
/// - `[text](src/file.ts)` → `[text](dialectic:src/file.ts)`
///
/// This ensures the extension receives properly formatted dialectic: URLs without
/// needing client-side conversion logic.
#[derive(Debug)]
pub struct ResolvedMarkdownElement {
    pub markdown: String,
}

impl Serialize for ResolvedMarkdownElement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as just the string content, not as an object
        self.markdown.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ResolvedMarkdownElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_markdown = String::deserialize(deserializer)?;
        let processed_content = process_markdown_links(raw_markdown);
        Ok(ResolvedMarkdownElement {
            markdown: processed_content,
        })
    }
}

pub fn process_markdown_links(markdown: String) -> String {
    use pulldown_cmark::{Event, Parser, Tag};

    let parser = Parser::new(&markdown);
    let mut events: Vec<Event> = parser.collect();

    // Pass 1: Coalesce adjacent Text events first
    events = coalesce_text_events(events);

    // Pass 2: Process malformed links in Text events
    events = process_malformed_links_in_events(events);

    // Pass 3: Convert well-formed Link events (but skip ones already processed)
    for event in &mut events {
        if let Event::Start(Tag::Link { dest_url, .. }) = event {
            // Only convert if it doesn't already start with dialectic:
            if !dest_url.starts_with("dialectic:") {
                let converted_url = convert_url_to_dialectic(dest_url);
                *dest_url = converted_url.into();
            }
        }
    }

    // Convert events back to markdown
    let mut output = String::new();
    pulldown_cmark_to_cmark::cmark(events.into_iter(), &mut output).unwrap();
    output
}

fn coalesce_text_events(events: Vec<Event>) -> Vec<Event> {
    use pulldown_cmark::Event;

    let mut result = Vec::new();
    let mut accumulated_text = String::new();

    for event in events {
        match event {
            Event::Text(text) => {
                accumulated_text.push_str(&text);
            }
            _ => {
                if !accumulated_text.is_empty() {
                    result.push(Event::Text(accumulated_text.clone().into()));
                    accumulated_text.clear();
                }
                result.push(event);
            }
        }
    }

    // Don't forget any remaining text
    if !accumulated_text.is_empty() {
        result.push(Event::Text(accumulated_text.into()));
    }

    result
}

fn process_malformed_links_in_events(events: Vec<Event>) -> Vec<Event> {
    use pulldown_cmark::Event;

    let mut result = Vec::new();

    for event in events {
        match event {
            Event::Text(text) => {
                process_malformed_links_in_text(&text, &mut result);
            }
            _ => {
                result.push(event);
            }
        }
    }

    result
}

fn process_malformed_links_in_text(text: &str, events: &mut Vec<Event>) {
    use pulldown_cmark::{Event, LinkType, Tag, TagEnd};

    // Combined regex with named captures
    let combined_regex = regex::Regex::new(
        r"(?P<malformed>\[(?P<malformed_text>[^\]]+)\]\((?P<malformed_url>[^)]*[ \{\[\(][^)]*)\))|(?P<reference>\[(?P<reference_text>[^\]]+)\]\[\])"
    ).unwrap();

    process_regex_matches(text, &combined_regex, events, |caps, events| {
        if caps.name("malformed").is_some() {
            // Malformed link: [text](url with spaces)
            let link_text = caps.name("malformed_text").unwrap().as_str().to_string();
            let url = caps.name("malformed_url").unwrap().as_str().to_string();

            // Generate proper link events
            events.push(Event::Start(Tag::Link {
                link_type: LinkType::Inline,
                dest_url: url.into(),
                title: "".into(),
                id: "".into(),
            }));
            events.push(Event::Text(link_text.into()));
            events.push(Event::End(TagEnd::Link));
        } else if caps.name("reference").is_some() {
            // Reference link: [text][]
            let link_text = caps.name("reference_text").unwrap().as_str().to_string();

            // Determine URL based on pattern
            let url = if let Some(line_caps) = regex::Regex::new(r"^([^:]+\.[a-z]+):(\d+)$")
                .unwrap()
                .captures(&link_text)
            {
                let filename = &line_caps[1];
                let line_num = &line_caps[2];
                format!("dialectic:{}#L{}", filename, line_num)
            } else if regex::Regex::new(r"^[^:]+\.[a-z]+$")
                .unwrap()
                .is_match(&link_text)
            {
                format!("dialectic:{}", link_text)
            } else {
                // For other reference links, leave as-is for now
                events.push(Event::Text(format!("[{}][]", link_text).into()));
                return;
            };

            // Generate proper link events
            events.push(Event::Start(Tag::Link {
                link_type: LinkType::Inline,
                dest_url: url.into(),
                title: "".into(),
                id: "".into(),
            }));
            events.push(Event::Text(link_text.into()));
            events.push(Event::End(TagEnd::Link));
        }
    });
}

fn process_regex_matches<F>(
    text: &str,
    regex: &regex::Regex,
    events: &mut Vec<Event>,
    mut handle_match: F,
) where
    F: FnMut(&regex::Captures, &mut Vec<Event>),
{
    let mut last_end = 0;

    for m in regex.find_iter(text) {
        // Add text before the match
        if m.start() > last_end {
            events.push(Event::Text(text[last_end..m.start()].to_string().into()));
        }

        if let Some(caps) = regex.captures(&text[m.start()..m.end()]) {
            handle_match(&caps, events);
        }

        last_end = m.end();
    }

    // Add any remaining text
    if last_end < text.len() {
        events.push(Event::Text(text[last_end..].to_string().into()));
    }
}

fn convert_url_to_dialectic(url: &str) -> String {
    // Handle path?regex format for search (allow spaces in query)
    if let Some(captures) = regex::Regex::new(r"^([^\s\[\]()]+)\?(.+)$")
        .unwrap()
        .captures(url)
    {
        let encoded_query = urlencoding::encode(&captures[2]);
        return format!("dialectic:{}?regex={}", &captures[1], encoded_query);
    }

    // Handle path#L42-L50 format for line ranges
    if let Some(captures) = regex::Regex::new(r"^([^\s\[\]()]+)#L(\d+)-L(\d+)$")
        .unwrap()
        .captures(url)
    {
        return format!(
            "dialectic:{}?line={}-{}",
            &captures[1], &captures[2], &captures[3]
        );
    }

    // Handle path#L42 format for single lines
    if let Some(captures) = regex::Regex::new(r"^([^\s\[\]()]+)#L(\d+)$")
        .unwrap()
        .captures(url)
    {
        return format!("dialectic:{}?line={}", &captures[1], &captures[2]);
    }

    // Handle bare filenames (including those with spaces or special chars)
    if !url.contains("://") && !url.starts_with("dialectic:") {
        return format!("dialectic:{}", url);
    }

    // Return unchanged if no patterns match
    url.to_string()
}

/// Resolved git diff output from the [`GitDiff`] dialect function.
///
/// This is the processed result containing file changes from a git commit range,
/// with each file's additions, deletions, and diff hunks.
#[derive(Serialize, Deserialize, Debug)]
pub struct GitDiffElement {
    pub files: Vec<crate::git::FileChange>,
}

/// Resolved walkthrough element output from various dialect functions.
///
/// This enum represents the processed results from executing Dialect programs
/// in walkthrough sections. Each variant corresponds to a different type of
/// input that can be resolved:
/// - Plain strings → [`ResolvedMarkdownElement`]
/// - [`Comment`] dialect function → [`ResolvedComment`]
/// - [`GitDiff`] dialect function → [`GitDiffElement`]
/// - [`Action`] dialect function → [`ResolvedAction`]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ResolvedWalkthroughElement {
    /// Plain markdown text with processed links
    Markdown(ResolvedMarkdownElement),
    /// Comment placed at specific locations
    Comment(ResolvedComment),
    /// Git diff display
    GitDiff(GitDiffElement),
    /// Action button
    Action(ResolvedAction),
}
#[cfg(test)]
mod url_conversion_tests {
    use super::*;
    use expect_test::{Expect, expect};
    use pulldown_cmark::{Event, Parser, Tag};

    fn check_extracted_urls(input: &str, expected: Expect) {
        let processed = process_markdown_links(input.to_string());

        // Extract URLs using pulldown-cmark parser
        let parser = Parser::new(&processed);
        let mut urls = Vec::new();

        for event in parser {
            if let Event::Start(Tag::Link { dest_url, .. }) = event {
                urls.push(dest_url.to_string());
            }
        }

        expected.assert_debug_eq(&urls);
    }

    #[test]
    fn test_markdown_url_conversion() {
        let markdown = r#"
Check out [this function](src/auth.ts?validateToken) and 
[this line](src/auth.ts#L42) or [this range](src/auth.ts#L42-L50).
Also see [the whole file](src/auth.ts) and [this function with spaces](src/auth.rs?fn foo).
"#;

        check_extracted_urls(
            markdown,
            expect![[r#"
            [
                "dialectic:src/auth.ts?regex=validateToken",
                "dialectic:src/auth.ts?line=42",
                "dialectic:src/auth.ts?line=42-50",
                "dialectic:src/auth.ts",
                "dialectic:src/auth.rs?regex=fn%20foo",
            ]
        "#]],
        );
    }

    #[test]
    fn test_pulldown_cmark_respects_code_blocks() {
        let markdown = r#"
Here's a real link: [check this](src/real.ts?pattern)

But this should be ignored:
```
// This is just example code, not a real link
[fake link](src/fake.ts?pattern)
```

And this inline code too: `[another fake](src/inline.ts)`
"#;

        check_extracted_urls(
            markdown,
            expect![[r#"
            [
                "dialectic:src/real.ts?regex=pattern",
            ]
        "#]],
        );
    }

    #[test]
    fn test_malformed_and_reference_links() {
        let markdown = r#"
Check [file with spaces](src/auth.rs?fn foo) and [file with bracket](src/auth.rs?fn{bar).
Also [main.rs][] and [utils.ts:42][].
"#;

        check_extracted_urls(
            markdown,
            expect![[r#"
            [
                "dialectic:src/auth.rs?regex=fn%20foo",
                "dialectic:src/auth.rs?regex=fn%7Bbar",
                "dialectic:main.rs",
                "dialectic:utils.ts#L42",
            ]
        "#]],
        );
    }

    #[test]
    fn test_mixed_link_types_in_single_text() {
        let markdown = r#"
Check [foo.rs][], [foo](foo.rs?a b), [bar.rs][].
"#;

        check_extracted_urls(
            markdown,
            expect![[r#"
            [
                "dialectic:foo.rs",
                "dialectic:foo.rs?regex=a%20b",
                "dialectic:bar.rs",
            ]
        "#]],
        );
    }

    #[test]
    fn test_resolved_comment_deserialization() {
        // Test each part separately

        // 1. Test the content array element
        let content_json = r#""This should find exactly one location with no icon!""#;
        let content_result: Result<ResolvedWalkthroughElement, _> =
            serde_json::from_str(content_json);
        match content_result {
            Ok(_) => println!("✅ Content element deserialized successfully"),
            Err(e) => println!("❌ Content element failed: {}", e),
        }

        // 1b. Test the content array element
        let content_json = r#""This should find exactly one location with no icon!""#;
        let content_result: Result<ResolvedMarkdownElement, _> = serde_json::from_str(content_json);
        match content_result {
            Ok(_) => println!("✅ Markdown element deserialized successfully"),
            Err(e) => println!("❌ Markdown element failed: {}", e),
        }

        // 2. Test the location object
        let location_json = r#"{
            "content": "ResolvedLocation::FileRange(range) => vec![range],",
            "end": {"column": 63, "line": 325},
            "path": "server/src/ide.rs",
            "start": {"column": 13, "line": 325}
        }"#;
        let location_result: Result<FileRange, _> = serde_json::from_str(location_json);
        match location_result {
            Ok(_) => println!("✅ Location deserialized successfully"),
            Err(e) => println!("❌ Location failed: {}", e),
        }

        // 3. Test the full ResolvedComment
        let json = r#"{
            "id": "test-comment-id",
            "content": ["This should find exactly one location with no icon!"],
            "icon": null,
            "locations": [{
                "content": "ResolvedLocation::FileRange(range) => vec![range],",
                "end": {"column": 63, "line": 325},
                "path": "server/src/ide.rs",
                "start": {"column": 13, "line": 325}
            }]
        }"#;

        let comment_result: Result<ResolvedComment, _> = serde_json::from_str(json);
        match comment_result {
            Ok(comment) => println!(
                "✅ ResolvedComment deserialized with {} locations",
                comment.locations.len()
            ),
            Err(e) => println!("❌ ResolvedComment failed: {}", e),
        }

        // 4. Test as ResolvedWalkthroughElement
        let result: Result<ResolvedWalkthroughElement, _> = serde_json::from_str(json);
        match result {
            Ok(_) => println!("✅ ResolvedWalkthroughElement deserialized successfully"),
            Err(e) => println!("❌ ResolvedWalkthroughElement failed: {}", e),
        }
    }
}
