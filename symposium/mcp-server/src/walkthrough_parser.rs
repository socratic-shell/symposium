use anyhow::Result;
use pulldown_cmark::{Event, Parser, Tag, TagEnd, html};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use uuid::Uuid;

use crate::dialect::DialectInterpreter;
use crate::ide::{FileRange, IpcClient, SymbolDef};

/// Location data that can be either a symbol definition or a file range
/// Uses untagged enum to automatically deserialize from different location formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocationData {
    /// Symbol definition (from findDefinitions, findReferences, etc.)
    SymbolDef(SymbolDef),
    /// File range (from search operations)  
    FileRange(FileRange),
}

/// Parsed XML element from walkthrough markdown
#[derive(Debug, Clone, PartialEq)]
pub enum XmlElement {
    Comment {
        location: String,
        icon: Option<String>,
        content: String,
    },
    GitDiff {
        range: String,
        exclude_unstaged: bool,
        exclude_staged: bool,
    },
    Action {
        button: String,
        message: String,
    },
    Mermaid {
        content: String,
    },
}

/// Resolved XML element with dummy data for Phase 1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedXmlElement {
    pub element_type: String,
    pub attributes: HashMap<String, String>,
    pub resolved_data: serde_json::Value,
    pub content: String,
}

/// Main walkthrough parser
pub struct WalkthroughParser<T: IpcClient + Clone + 'static> {
    interpreter: DialectInterpreter<T>,
    uuid_generator: Box<dyn Fn() -> String + Send + Sync>,
    base_uri: Option<String>,
}

impl<T: IpcClient + Clone + 'static> WalkthroughParser<T> {
    pub fn new(interpreter: DialectInterpreter<T>) -> Self {
        Self {
            interpreter,
            uuid_generator: Box::new(|| Uuid::new_v4().to_string()),
            base_uri: None,
        }
    }

    pub fn with_base_uri(mut self, base_uri: String) -> Self {
        self.base_uri = Some(base_uri);
        self
    }

    #[cfg(test)]
    pub fn with_uuid_generator<F>(interpreter: DialectInterpreter<T>, generator: F) -> Self
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        Self {
            interpreter,
            uuid_generator: Box::new(generator),
            base_uri: None,
        }
    }

    fn generate_uuid(&self) -> String {
        (self.uuid_generator)()
    }

    /// Parse markdown with embedded XML elements and return normalized output
    pub async fn parse_and_normalize(&mut self, content: &str) -> Result<String, anyhow::Error> {
        let processed_events = self.process_events_sequentially(content).await?;
        Self::render_events_to_markdown(processed_events)
    }

    /// Process pulldown-cmark event stream sequentially
    async fn process_events_sequentially<'a>(
        &mut self,
        content: &'a str,
    ) -> Result<Vec<Event<'a>>, anyhow::Error> {
        let mut input_events: VecDeque<Event<'a>> = Parser::new(content).collect();
        let mut output_events = Vec::new();

        while let Some(event) = input_events.pop_front() {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    if self.is_special_code_block(&kind) {
                        self.process_code_block(kind, &mut input_events, &mut output_events)
                            .await?;
                    } else {
                        output_events.push(Event::Start(Tag::CodeBlock(kind)));
                    }
                }
                _ => output_events.push(event),
            }
        }

        Ok(output_events)
    }

    /// Check if code block is one of our special types (mermaid, comment, etc.)
    fn is_special_code_block(&self, kind: &pulldown_cmark::CodeBlockKind) -> bool {
        match kind {
            pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                matches!(lang.trim(), "mermaid" | "comment" | "gitdiff" | "action")
            }
            _ => false,
        }
    }

    /// Parse YAML-style parameters from code block content
    /// Returns (parameters, remaining_content)
    fn parse_yaml_parameters(&self, content: &str) -> (HashMap<String, String>, String) {
        let mut params = HashMap::new();
        let mut lines: VecDeque<&str> = content.lines().collect();
        
        // Parse YAML parameters from the beginning
        while let Some(line) = lines.pop_front() {
            let trimmed = line.trim();
            
            if trimmed.is_empty() {
                // Empty line marks end of YAML section
                break;
            } else if let Some((key, value)) = trimmed.split_once(':') {
                if key.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                    // YAML parameter line looks like `foo: ...`
                    let key = key.trim().to_string();
                    let value = value.trim().to_string();
                    params.insert(key, value);
                    continue;
                }
            }

            // Line doesn't contain ':', this is content
            lines.push_front(line);
            break;
        }
        
        // Collect remaining content
        let remaining_content = lines.into_iter().collect::<Vec<_>>().join("\n");
        
        (params, remaining_content)
    }

    /// Process special code blocks (mermaid, comment, etc.)
    async fn process_code_block<'a>(
        &mut self,
        kind: pulldown_cmark::CodeBlockKind<'a>,
        input_events: &mut VecDeque<Event<'a>>,
        output_events: &mut Vec<Event<'a>>,
    ) -> Result<(), anyhow::Error> {
        // Extract the language from the code block
        let element_type = match &kind {
            pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.trim().to_string(),
            _ => return Ok(()), // Not a fenced code block
        };

        // Collect the content from the code block
        let mut content = String::new();
        while let Some(event) = input_events.pop_front() {
            match event {
                Event::Text(text) => {
                    content.push_str(&text);
                }
                Event::End(TagEnd::CodeBlock) => {
                    break; // End of code block
                }
                _ => {
                    // Unexpected event in code block, add it back and break
                    input_events.push_front(event);
                    break;
                }
            }
        }

        // Parse YAML parameters from content (except for mermaid)
        let (params, remaining_content) = if element_type == "mermaid" {
            (HashMap::new(), content)
        } else {
            self.parse_yaml_parameters(&content)
        };

        // Create the appropriate XML element
        match element_type.as_str() {
            "mermaid" => {
                let xml_element = XmlElement::Mermaid { content: remaining_content };
                let resolved = self.resolve_single_element(xml_element).await?;
                let html = self.create_mermaid_html(&resolved);
                output_events.push(Event::InlineHtml(html.into()));
            }
            "comment" => {
                let location = params.get("location").cloned().unwrap_or_default();
                let icon = params.get("icon").cloned();
                let xml_element = XmlElement::Comment { location, icon, content: remaining_content };
                let resolved = self.resolve_single_element(xml_element).await?;
                let html = self.create_comment_html(&resolved);
                output_events.push(Event::InlineHtml(html.into()));
            }
            "gitdiff" => {
                let range = params.get("range").cloned().unwrap_or_default();
                let exclude_unstaged = params.get("exclude-unstaged").is_some() || params.get("exclude_unstaged").is_some();
                let exclude_staged = params.get("exclude-staged").is_some() || params.get("exclude_staged").is_some();
                let xml_element = XmlElement::GitDiff { range, exclude_unstaged, exclude_staged };
                let resolved = self.resolve_single_element(xml_element).await?;
                let html = self.create_gitdiff_html(&resolved);
                output_events.push(Event::InlineHtml(html.into()));
            }
            "action" => {
                let button = params.get("button").cloned().unwrap_or("Action".to_string());
                let xml_element = XmlElement::Action { button, message: remaining_content };
                let resolved = self.resolve_single_element(xml_element).await?;
                let html = self.create_action_html(&resolved);
                output_events.push(Event::InlineHtml(html.into()));
            }
            _ => {
                // Unknown element type, shouldn't happen
                return Ok(());
            }
        }

        Ok(())
    }

    /// Render pulldown-cmark events back to markdown/HTML
    fn render_events_to_markdown<'a>(events: Vec<Event<'a>>) -> Result<String, anyhow::Error> {
        let mut output = String::new();
        html::push_html(&mut output, events.into_iter());
        Ok(output)
    }

    /// Resolve a single XML element with Dialect evaluation
    async fn resolve_single_element(
        &mut self,
        element: XmlElement,
    ) -> Result<ResolvedXmlElement, anyhow::Error> {
        let (element_type, attributes, resolved_data) = match &element {
            XmlElement::Comment {
                location,
                icon,
                content: _,
            } => {
                let mut attrs = HashMap::new();
                if let Some(icon) = icon {
                    attrs.insert("icon".to_string(), icon.clone());
                }

                // Resolve Dialect expression for location
                let resolved_data = if !location.is_empty() {
                    // Clone interpreter for thread safety
                    let mut interpreter = self.interpreter.clone();
                    let location_clone = location.clone();

                    let result = tokio::task::spawn_blocking(move || {
                        tokio::runtime::Handle::current()
                            .block_on(async move { interpreter.evaluate(&location_clone).await })
                    })
                    .await
                    .map_err(|e| anyhow::anyhow!("Task execution failed: {}", e))?;

                    match result {
                        Ok(result) => {
                            serde_json::json!({
                                "locations": result,
                                "dialect_expression": location
                            })
                        }
                        Err(e) => {
                            serde_json::json!({
                                "error": format!("Failed to resolve location: {}", e),
                                "dialect_expression": location
                            })
                        }
                    }
                } else {
                    serde_json::json!({
                        "locations": []
                    })
                };

                ("comment".to_string(), attrs, resolved_data)
            }
            XmlElement::GitDiff {
                range,
                exclude_unstaged,
                exclude_staged,
            } => {
                // Use GitService to generate actual file changes
                use crate::git::GitService;

                let resolved_data = match GitService::new(".") {
                    Ok(git_service) => {
                        match git_service.parse_commit_range(range).and_then(
                            |(base_oid, head_oid)| git_service.generate_diff(base_oid, head_oid),
                        ) {
                            Ok(file_changes) => {
                                serde_json::json!({
                                    "type": "gitdiff",
                                    "range": range,
                                    "files": file_changes
                                })
                            }
                            Err(e) => {
                                // Fallback for git errors (tests, non-git directories, etc.)
                                serde_json::json!({
                                    "type": "gitdiff",
                                    "range": range,
                                    "error": format!("Git error: {}", e)
                                })
                            }
                        }
                    }
                    Err(e) => {
                        // Fallback for non-git directories (like in tests)
                        serde_json::json!({
                            "type": "gitdiff",
                            "range": range,
                            "error": format!("Not a git repository: {}", e)
                        })
                    }
                };

                let mut attrs = HashMap::new();
                if *exclude_unstaged {
                    attrs.insert("exclude-unstaged".to_string(), "true".to_string());
                }
                if *exclude_staged {
                    attrs.insert("exclude-staged".to_string(), "true".to_string());
                }

                ("gitdiff".to_string(), attrs, resolved_data)
            }
            XmlElement::Action { button, message: _ } => {
                let mut attrs = HashMap::new();
                attrs.insert("button".to_string(), button.clone());

                let resolved_data = serde_json::json!({
                    "button_text": button
                });

                ("action".to_string(), attrs, resolved_data)
            }
            XmlElement::Mermaid { content: _ } => {
                let attrs = HashMap::new();
                let resolved_data = serde_json::json!({
                    "type": "mermaid",
                    "rendered": true
                });

                ("mermaid".to_string(), attrs, resolved_data)
            }
        };

        let content = match &element {
            XmlElement::Comment { content, .. } => content.clone(),
            XmlElement::Action { message, .. } => message.clone(),
            XmlElement::Mermaid { content } => content.clone(),
            XmlElement::GitDiff { .. } => String::new(),
        };

        Ok(ResolvedXmlElement {
            element_type,
            attributes,
            resolved_data,
            content,
        })
    }

    /// Generate HTML for comment elements
    
    /// Format dialect expressions in a more user-friendly way
    fn format_dialect_expression(&self, dialect_expression: &str) -> String {
        // If empty, return as-is
        if dialect_expression.is_empty() {
            return dialect_expression.to_string();
        }

        // Try to parse and format common expressions
        if dialect_expression.starts_with("search(") {
            // Parse search("path", "pattern") or search("path", "pattern", ".ext")
            if let Some(captures) = regex::Regex::new(r#"search\(\s*["`]([^"`]+)["`]\s*,\s*["`]([^"`]+)["`](?:\s*,\s*["`]([^"`]+)["`])?\s*\)"#)
                .unwrap()
                .captures(dialect_expression) 
            {
                let pattern = captures.get(2).unwrap().as_str();
                return format!("/{pattern}/");
            }
        } else if dialect_expression.starts_with("findDefinition(") {
            // Parse findDefinition("symbol") or findDefinitions("symbol")
            if let Some(captures) = regex::Regex::new(r#"findDefinitions?\(\s*["`]([^"`]+)["`]\s*\)"#)
                .unwrap()
                .captures(dialect_expression) 
            {
                let symbol = captures.get(1).unwrap().as_str();
                return format!("`{symbol}`");
            }
        } else if dialect_expression.starts_with("findReferences(") {
            // Parse findReferences("symbol")
            if let Some(captures) = regex::Regex::new(r#"findReferences\(\s*["`]([^"`]+)["`]\s*\)"#)
                .unwrap()
                .captures(dialect_expression) 
            {
                let symbol = captures.get(1).unwrap().as_str();
                return format!("References to `{symbol}`", );
            }
        } else if dialect_expression.starts_with("lines(") {
            // Parse lines("path", start, end)
            if let Some(captures) = regex::Regex::new(r#"lines\(\s*["`]([^"`]+)["`]\s*,\s*(\d+)\s*,\s*(\d+)\s*\)"#)
                .unwrap()
                .captures(dialect_expression) 
            {
                let path = captures.get(1).unwrap().as_str();
                let start = captures.get(2).unwrap().as_str();
                let end = captures.get(3).unwrap().as_str();
                return format!("`{path}:{start}-{end}`");
            }
        }

        // If we can't parse it, return the original expression
        dialect_expression.to_string()
    }

    fn create_comment_html(&self, resolved: &ResolvedXmlElement) -> String {
        // Extract and normalize locations from resolved data
        let empty_vec = vec![];
        let raw_locations = resolved
            .resolved_data
            .get("locations")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        // Normalize locations to consistent format for webview consumption
        let mut normalized_locations: Vec<FileRange> = raw_locations
            .iter()
            .filter_map(|loc| {
                // Try to deserialize as LocationData using untagged enum
                match serde_json::from_value::<LocationData>(loc.clone()) {
                    Ok(LocationData::FileRange(r)) => Some(r),
                    Ok(LocationData::SymbolDef(d)) => Some(d.defined_at),

                    // if deserialization files, ignore, but we should really do something else
                    Err(_) => None,
                }
            })
            .collect();

        // Convert paths to resolve if base-uri provided 
        if let Some(base_uri) = &self.base_uri {
            if let Ok(base_uri) = Path::new(base_uri).canonicalize() {
                for l in &mut normalized_locations {
                    if let Ok(abs_path) = std::path::Path::new(&l.path).canonicalize() {
                        if let Ok(rel_path) = abs_path.strip_prefix(&base_uri) {
                            l.path = rel_path.to_string_lossy().to_string();
                        }
                    }
                }
            }
        }

        // Generate comment data for click handler with normalized locations
        let comment_data = serde_json::json!({
            "id": format!("comment-{}", self.generate_uuid()),
            "locations": normalized_locations,
            "comment": [&resolved.content]
        });

        // Get icon from attributes
        let default_icon = "comment".to_string();
        let icon = resolved.attributes.get("icon").unwrap_or(&default_icon);
        let icon_emoji = match icon.as_str() {
            "info" => "ℹ️",
            "lightbulb" => "💡",
            "gear" => "⚙️",
            "warning" => "⚠️",
            "question" => "❓",
            _ => "💬",
        };

        // Extract and format dialect expression from resolved data
        let raw_dialect_expression = resolved
            .resolved_data
            .get("dialect_expression")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let formatted_dialect_expression = self.format_dialect_expression(raw_dialect_expression);

        // Generate location display using normalized locations
        let location_display = if normalized_locations.len() == 1 {
            // Single location - show file:line with relative path
            let loc = &normalized_locations[0];
            format!("{}:{}", loc.path, loc.start.line)
        } else if normalized_locations.len() > 1 {
            // Multiple locations - show count
            format!("({} possible locations) 🔍", normalized_locations.len())
        } else {
            "no location".to_string()
        };

        // Keep them separate for individual div rendering

        let comment_data_encoded = serde_json::to_string(&comment_data).unwrap_or_default();
        let comment_data_escaped = comment_data_encoded.replace('"', "&quot;");

        // Build the expression content for the inline div

        format!(
            r#"<div class="comment-item" data-comment="{comment_data_escaped}" style="cursor: pointer; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 8px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                <div style="display: flex; align-items: flex-start;">
                    <div class="comment-icon" style="margin-right: 8px; font-size: 16px;">{icon_emoji}</div>
                    <div class="comment-content" style="flex: 1;">
                        <div class="comment-expression" style="display: block; color: var(--vscode-textLink-foreground); font-family: var(--vscode-editor-font-family); font-size: 1.0em; font-weight: 500; margin-bottom: 6px; text-decoration: underline;">{formatted_dialect_expression}</div>
                        <div class="comment-locations" style="font-weight: 500; color: var(--vscode-textLink-foreground); margin-bottom: 4px; font-family: var(--vscode-editor-font-family); font-size: 0.9em;">{location_display}</div>
                        <div class="comment-text" style="color: var(--vscode-foreground); font-size: 0.9em;">{resolved_content}</div>
                    </div>
                </div>
            </div>"#,
            resolved_content = resolved.content
        )
    }

    /// Generate HTML for action elements
    fn create_action_html(&self, resolved: &ResolvedXmlElement) -> String {
        let default_button = "Action".to_string();
        let button_text = resolved.attributes.get("button").unwrap_or(&default_button);
        let tell_agent = resolved.content.replace('"', "&quot;");

        format!(
            r#"<button class="action-button" data-tell-agent="{}" style="background-color: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; margin: 8px 0; font-size: 0.9em;">{}</button>"#,
            tell_agent, button_text
        )
    }

    /// Generate HTML for gitdiff elements
    fn create_gitdiff_html(&self, resolved: &ResolvedXmlElement) -> String {
        // For now, return a placeholder - we'll implement this properly later
        format!(
            r#"<div class="gitdiff-container" style="border: 1px solid var(--vscode-panel-border); border-radius: 4px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                <div style="padding: 12px; color: var(--vscode-descriptionForeground);">GitDiff rendering: {}</div>
            </div>"#,
            resolved
                .resolved_data
                .get("range")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        )
    }

    /// Generate HTML for mermaid elements
    fn create_mermaid_html(&self, resolved: &ResolvedXmlElement) -> String {
        // Keep mermaid elements as-is for client-side processing
        format!("<mermaid>{}</mermaid>", resolved.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ide::test::MockIpcClient;
    use expect_test::{Expect, expect};

    fn create_test_parser() -> WalkthroughParser<MockIpcClient> {
        let mut interpreter = DialectInterpreter::new(MockIpcClient::new());
        interpreter.add_standard_ide_functions();
        WalkthroughParser::with_uuid_generator(interpreter, || "test-uuid".to_string())
    }

    fn check(input: &str, expect: Expect) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut parser = create_test_parser();
        let result = rt.block_on(parser.parse_and_normalize(input)).unwrap();
        expect.assert_eq(&result);
    }

    #[test]
    fn test_simple_comment_resolution() {
        check(
            r#"
```comment
location: findDefinitions(`User`)

User struct
```
"#,
            expect![[r#"
                <div class="comment-item" data-comment="{&quot;comment&quot;:[&quot;User struct&quot;],&quot;id&quot;:&quot;comment-test-uuid&quot;,&quot;locations&quot;:[{&quot;content&quot;:&quot;struct User {&quot;,&quot;end&quot;:{&quot;column&quot;:4,&quot;line&quot;:10},&quot;path&quot;:&quot;src/models.rs&quot;,&quot;start&quot;:{&quot;column&quot;:0,&quot;line&quot;:10}}]}" style="cursor: pointer; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 8px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                                <div style="display: flex; align-items: flex-start;">
                                    <div class="comment-icon" style="margin-right: 8px; font-size: 16px;">💬</div>
                                    <div class="comment-content" style="flex: 1;">
                                        <div class="comment-expression" style="display: block; color: var(--vscode-textLink-foreground); font-family: var(--vscode-editor-font-family); font-size: 1.0em; font-weight: 500; margin-bottom: 6px; text-decoration: underline;">findDefinitions(`User`)</div>
                                        <div class="comment-locations" style="font-weight: 500; color: var(--vscode-textLink-foreground); margin-bottom: 4px; font-family: var(--vscode-editor-font-family); font-size: 0.9em;">src/models.rs:10</div>
                                        <div class="comment-text" style="color: var(--vscode-foreground); font-size: 0.9em;">User struct</div>
                                    </div>
                                </div>
                            </div>"#]],
        );
    }

    #[test]
    fn test_self_closing_gitdiff() {
        check(
            r#"
```gitdiff
range: HEAD~1..HEAD
```
"#,
            expect![[r#"
                <div class="gitdiff-container" style="border: 1px solid var(--vscode-panel-border); border-radius: 4px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                                <div style="padding: 12px; color: var(--vscode-descriptionForeground);">GitDiff rendering: HEAD~1..HEAD</div>
                            </div>"#]],
        );
    }

    #[test]
    fn test_action_element() {
        check(
            r#"<action button="Next Step">What should we do next?</action>"#,
            expect![[r#"
                <p><action button="Next Step">What should we do next?</action></p>
            "#]],
        );
    }

    #[test]
    fn test_full_walkthrough_with_mixed_content() {
        check(
            r#"# My Walkthrough

This is some markdown content.

```comment
location: findDefinitions(`User`)
icon: lightbulb

This explains the User struct
```

More markdown here.

```gitdiff
range: HEAD~1..HEAD
```

```action
button: Next Step

What should we do next?
```
"#,
            expect![[r#"
                <h1>My Walkthrough</h1>
                <p>This is some markdown content.</p>
                <div class="comment-item" data-comment="{&quot;comment&quot;:[&quot;This explains the User struct&quot;],&quot;id&quot;:&quot;comment-test-uuid&quot;,&quot;locations&quot;:[{&quot;content&quot;:&quot;struct User {&quot;,&quot;end&quot;:{&quot;column&quot;:4,&quot;line&quot;:10},&quot;path&quot;:&quot;src/models.rs&quot;,&quot;start&quot;:{&quot;column&quot;:0,&quot;line&quot;:10}}]}" style="cursor: pointer; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 8px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                                <div style="display: flex; align-items: flex-start;">
                                    <div class="comment-icon" style="margin-right: 8px; font-size: 16px;">💡</div>
                                    <div class="comment-content" style="flex: 1;">
                                        <div class="comment-expression" style="display: block; color: var(--vscode-textLink-foreground); font-family: var(--vscode-editor-font-family); font-size: 1.0em; font-weight: 500; margin-bottom: 6px; text-decoration: underline;">findDefinitions(`User`)</div>
                                        <div class="comment-locations" style="font-weight: 500; color: var(--vscode-textLink-foreground); margin-bottom: 4px; font-family: var(--vscode-editor-font-family); font-size: 0.9em;">src/models.rs:10</div>
                                        <div class="comment-text" style="color: var(--vscode-foreground); font-size: 0.9em;">This explains the User struct</div>
                                    </div>
                                </div>
                            </div>
                <p>More markdown here.</p>
                <div class="gitdiff-container" style="border: 1px solid var(--vscode-panel-border); border-radius: 4px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                                <div style="padding: 12px; color: var(--vscode-descriptionForeground);">GitDiff rendering: HEAD~1..HEAD</div>
                            </div><button class="action-button" data-tell-agent="What should we do next?" style="background-color: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; margin: 8px 0; font-size: 0.9em;">Next Step</button>"#]],
        );
    }

    #[test]
    fn test_markdown_structure_preservation() {
        check(
            r#"# Title
Some text before
```comment
location: findDefinitions(`User`)

User comment
```
Some text after
```gitdiff
range:HEAD
```
More text"#,
            expect![[r#"
                <h1>Title</h1>
                <p>Some text before</p>
                <div class="comment-item" data-comment="{&quot;comment&quot;:[&quot;User comment&quot;],&quot;id&quot;:&quot;comment-test-uuid&quot;,&quot;locations&quot;:[{&quot;content&quot;:&quot;struct User {&quot;,&quot;end&quot;:{&quot;column&quot;:4,&quot;line&quot;:10},&quot;path&quot;:&quot;src/models.rs&quot;,&quot;start&quot;:{&quot;column&quot;:0,&quot;line&quot;:10}}]}" style="cursor: pointer; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 8px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                                <div style="display: flex; align-items: flex-start;">
                                    <div class="comment-icon" style="margin-right: 8px; font-size: 16px;">💬</div>
                                    <div class="comment-content" style="flex: 1;">
                                        <div class="comment-expression" style="display: block; color: var(--vscode-textLink-foreground); font-family: var(--vscode-editor-font-family); font-size: 1.0em; font-weight: 500; margin-bottom: 6px; text-decoration: underline;">findDefinitions(`User`)</div>
                                        <div class="comment-locations" style="font-weight: 500; color: var(--vscode-textLink-foreground); margin-bottom: 4px; font-family: var(--vscode-editor-font-family); font-size: 0.9em;">src/models.rs:10</div>
                                        <div class="comment-text" style="color: var(--vscode-foreground); font-size: 0.9em;">User comment</div>
                                    </div>
                                </div>
                            </div>
                <p>Some text after</p>
                <div class="gitdiff-container" style="border: 1px solid var(--vscode-panel-border); border-radius: 4px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                                <div style="padding: 12px; color: var(--vscode-descriptionForeground);">GitDiff rendering: HEAD</div>
                            </div>
                <p>More text</p>
            "#]],
        );
    }

    #[test]
    fn test_markdown_inside_xml_elements() {
        check(
            r#"
```comment
location:findDefinitions(`User`)

This has *emphasis* and **bold** text
```"#,
            expect![[r#"
                <div class="comment-item" data-comment="{&quot;comment&quot;:[&quot;This has *emphasis* and **bold** text&quot;],&quot;id&quot;:&quot;comment-test-uuid&quot;,&quot;locations&quot;:[{&quot;content&quot;:&quot;struct User {&quot;,&quot;end&quot;:{&quot;column&quot;:4,&quot;line&quot;:10},&quot;path&quot;:&quot;src/models.rs&quot;,&quot;start&quot;:{&quot;column&quot;:0,&quot;line&quot;:10}}]}" style="cursor: pointer; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 8px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                                <div style="display: flex; align-items: flex-start;">
                                    <div class="comment-icon" style="margin-right: 8px; font-size: 16px;">💬</div>
                                    <div class="comment-content" style="flex: 1;">
                                        <div class="comment-expression" style="display: block; color: var(--vscode-textLink-foreground); font-family: var(--vscode-editor-font-family); font-size: 1.0em; font-weight: 500; margin-bottom: 6px; text-decoration: underline;">findDefinitions(`User`)</div>
                                        <div class="comment-locations" style="font-weight: 500; color: var(--vscode-textLink-foreground); margin-bottom: 4px; font-family: var(--vscode-editor-font-family); font-size: 0.9em;">src/models.rs:10</div>
                                        <div class="comment-text" style="color: var(--vscode-foreground); font-size: 0.9em;">This has *emphasis* and **bold** text</div>
                                    </div>
                                </div>
                            </div>"#]],
        );
    }

    #[test]
    fn test_parse_yaml_parameters() {
        let parser = create_test_parser();
        
        // Test simple parameters
        let content = "location: findDefinition(`test`)\nicon: lightbulb\n\nThis is the content";
        let (params, remaining) = parser.parse_yaml_parameters(content);
        assert_eq!(params.get("location").unwrap(), "findDefinition(`test`)");
        assert_eq!(params.get("icon").unwrap(), "lightbulb");
        assert_eq!(remaining, "This is the content");
        
        // Test boolean flags
        let content = "range: HEAD~2\nexclude_unstaged: true\nexclude_staged: true\n";
        let (params, remaining) = parser.parse_yaml_parameters(content);
        assert_eq!(params.get("range").unwrap(), "HEAD~2");
        assert_eq!(params.get("exclude_unstaged").unwrap(), "true");
        assert_eq!(params.get("exclude_staged").unwrap(), "true");
        assert_eq!(remaining, "");
        
        // Test content only (no parameters)
        let content = "This is just content\nwith multiple lines";
        let (params, remaining) = parser.parse_yaml_parameters(content);
        assert!(params.is_empty());
        assert_eq!(remaining, "This is just content\nwith multiple lines");
    }

    #[tokio::test]
    async fn test_parse_mermaid_code_block() {
        let mut parser = create_test_parser();
        let markdown = r#"# Test Walkthrough

Here's a mermaid diagram:

```mermaid
flowchart TD
    A[Start] --> B[End]
```

More content here."#;

        let result = parser.parse_and_normalize(markdown).await.unwrap();
        
        // Should contain the mermaid HTML element
        assert!(result.contains("<mermaid>"));
        assert!(result.contains("flowchart TD"));
        assert!(result.contains("A[Start] --> B[End]"));
        assert!(result.contains("</mermaid>"));
    }

    #[tokio::test]
    async fn test_parse_comment_code_block_yaml() {
        let mut parser = create_test_parser();
        let markdown = r#"# Test Walkthrough

Here's a comment:

```comment
location: findDefinition(`foo`)
icon: lightbulb

This explains the foo function
```

More content here."#;

        let result = parser.parse_and_normalize(markdown).await.unwrap();
        
        // Should contain the comment HTML element
        assert!(result.contains("data-comment=\""));
        assert!(result.contains("This explains the foo function"));
        assert!(result.contains("💡")); // lightbulb icon

        expect_test::expect![[r#"
            <h1>Test Walkthrough</h1>
            <p>Here's a comment:</p>
            <div class="comment-item" data-comment="{&quot;comment&quot;:[&quot;This explains the foo function&quot;],&quot;id&quot;:&quot;comment-test-uuid&quot;,&quot;locations&quot;:[]}" style="cursor: pointer; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 8px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                            <div style="display: flex; align-items: flex-start;">
                                <div class="comment-icon" style="margin-right: 8px; font-size: 16px;">💡</div>
                                <div class="comment-content" style="flex: 1;">
                                    <div class="comment-expression" style="display: block; color: var(--vscode-textLink-foreground); font-family: var(--vscode-editor-font-family); font-size: 1.0em; font-weight: 500; margin-bottom: 6px; text-decoration: underline;">`foo`</div>
                                    <div class="comment-locations" style="font-weight: 500; color: var(--vscode-textLink-foreground); margin-bottom: 4px; font-family: var(--vscode-editor-font-family); font-size: 0.9em;">no location</div>
                                    <div class="comment-text" style="color: var(--vscode-foreground); font-size: 0.9em;">This explains the foo function</div>
                                </div>
                            </div>
                        </div>
            <p>More content here.</p>
        "#]].assert_eq(&result);
    }

    #[tokio::test]
    async fn test_parse_gitdiff_code_block_yaml() {
        let mut parser = create_test_parser();
        let markdown = r#"# Test Walkthrough

Here's a git diff:

```gitdiff
range: HEAD~2..HEAD
exclude_unstaged: true
exclude_staged: true
```

More content here."#;

        let result = parser.parse_and_normalize(markdown).await.unwrap();
        
        // Should contain the gitdiff HTML element
        assert!(result.contains("gitdiff-container"));
        assert!(result.contains("HEAD~2..HEAD"));
        assert!(result.contains("GitDiff rendering"));
    }

    #[tokio::test]
    async fn test_parse_action_code_block_yaml() {
        let mut parser = create_test_parser();
        let markdown = r#"# Test Walkthrough

Here's an action:

```action
button: Run Tests

Should we run the test suite now?
```

More content here."#;

        let result = parser.parse_and_normalize(markdown).await.unwrap();
        
        // Should contain the action HTML element
        assert!(result.contains("action-button"));
        assert!(result.contains("Run Tests"));
        assert!(result.contains("Should we run the test suite now?"));
    }

    #[tokio::test]
    async fn test_walkthrough_from_2025_09_12() {
        let mut parser = create_test_parser();
        let markdown = r#"# Testing Triple-Tickification After Restart

Let's test if the new code block syntax is working now!

## Mermaid Test
```mermaid
flowchart LR
    A[Old XML] --> B[Triple-Tickification]
    B --> C[New Code Blocks]
    C --> D[Success!]
```

## Comment Test
```comment
location: findDefinition(`WalkthroughParser`)
icon: rocket

This should now render as a proper comment box instead of raw markdown!
The parser should recognize this as a special code block and convert it to HTML.
```

## GitDiff Test
```gitdiff
range:"HEAD~3..HEAD"

```

## Action Test
```action
button: It's working!

Click this if you see a proper button instead of raw markdown text.
```

If you see rendered elements (diagram, comment box, diff container, button) instead of raw ````code blocks`, then triple-tickification is working! 🎉"#;

        let result = parser.parse_and_normalize(markdown).await.unwrap();
        
        // Should contain the comment HTML element
        expect_test::expect![[r#"
            <h1>Testing Triple-Tickification After Restart</h1>
            <p>Let's test if the new code block syntax is working now!</p>
            <h2>Mermaid Test</h2>
            <mermaid>flowchart LR
                A[Old XML] --> B[Triple-Tickification]
                B --> C[New Code Blocks]
                C --> D[Success!]
            </mermaid>
            <h2>Comment Test</h2>
            <div class="comment-item" data-comment="{&quot;comment&quot;:[&quot;This should now render as a proper comment box instead of raw markdown!\nThe parser should recognize this as a special code block and convert it to HTML.&quot;],&quot;id&quot;:&quot;comment-test-uuid&quot;,&quot;locations&quot;:[]}" style="cursor: pointer; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 8px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                            <div style="display: flex; align-items: flex-start;">
                                <div class="comment-icon" style="margin-right: 8px; font-size: 16px;">💬</div>
                                <div class="comment-content" style="flex: 1;">
                                    <div class="comment-expression" style="display: block; color: var(--vscode-textLink-foreground); font-family: var(--vscode-editor-font-family); font-size: 1.0em; font-weight: 500; margin-bottom: 6px; text-decoration: underline;">`WalkthroughParser`</div>
                                    <div class="comment-locations" style="font-weight: 500; color: var(--vscode-textLink-foreground); margin-bottom: 4px; font-family: var(--vscode-editor-font-family); font-size: 0.9em;">no location</div>
                                    <div class="comment-text" style="color: var(--vscode-foreground); font-size: 0.9em;">This should now render as a proper comment box instead of raw markdown!
            The parser should recognize this as a special code block and convert it to HTML.</div>
                                </div>
                            </div>
                        </div>
            <h2>GitDiff Test</h2>
            <div class="gitdiff-container" style="border: 1px solid var(--vscode-panel-border); border-radius: 4px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                            <div style="padding: 12px; color: var(--vscode-descriptionForeground);">GitDiff rendering: "HEAD~3..HEAD"</div>
                        </div>
            <h2>Action Test</h2>
            <button class="action-button" data-tell-agent="Click this if you see a proper button instead of raw markdown text." style="background-color: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; margin: 8px 0; font-size: 0.9em;">It's working!</button>
            <p>If you see rendered elements (diagram, comment box, diff container, button) instead of raw ````code blocks`, then triple-tickification is working! 🎉</p>
        "#]].assert_eq(&result);
    }

    #[tokio::test]
    async fn test_walkthrough_from_2025_09_12_2() {
        let mut parser = create_test_parser();

        let markdown = r#"# Triple-Tickification: Complete Implementation Walkthrough

We've successfully implemented the complete transition from XML syntax to markdown code blocks! Let's walk through all the key components.

## Architecture Overview

The new YAML-style parsing architecture handles all four element types:

```mermaid
flowchart TD
    A[Markdown Input] --> B{Code Block?}
    B -->|Regular| C[Standard Markdown]
    B -->|Special| D[Parse Language ID]
    D --> E{Known Element?}
    E -->|mermaid| F[Direct Content]
    E -->|comment/gitdiff/action| G[Parse YAML Parameters]
    F --> H[Create XML Element]
    G --> H
    H --> I[Resolve & Generate HTML]
    C --> J[Final HTML Output]
    I --> J
```

## Core Implementation: YAML Parameter Parser

The heart of the new system parses YAML-style parameters cleanly:

```comment
location: findDefinition(`parse_yaml_parameters`)
icon: gear

This function separates YAML parameters from content by processing lines sequentially.
It stops at the first empty line or non-YAML line, ensuring clean parameter extraction.
The key fix was replacing the flawed logic that mixed parameters with content.
```

## Element Processing Pipeline

Each code block type gets processed through a unified pipeline:

```comment
location: findDefinition(`process_code_block`)
icon: arrow-right

The processing pipeline handles all four element types (mermaid, comment, gitdiff, action)
with a unified approach. YAML parameters are extracted first, then the appropriate
XML element is created and resolved through the existing HTML generation system.
```

## New Syntax Examples

Here are examples of all four element types in the new YAML-style format:

```comment
location: search(`guidance.md`, `comment`)
icon: lightbulb
```

Comments now use clean YAML parameters:

```comment
location: findDefinition(`User`)
icon: rocket

This explains the User struct
```

GitDiff elements support boolean flags:
```gitdiff
range: HEAD~3..HEAD
exclude_unstaged: true
exclude_staged: true
```

Actions have simple button parameters:
```action
button: Run Tests

Should we execute the test suite now?
```

## What We Accomplished

Here's the complete diff of our changes:

```gitdiff
range: HEAD~15..HEAD
```

## Key Benefits Achieved

```action
button: Better Markdown Compatibility

The simple language identifiers (comment, gitdiff, action, mermaid) work perfectly 
with standard markdown parsers, fixing the compatibility issues we had with 
complex function-call syntax.
```

```action
button: Cleaner Syntax

YAML-style parameters are much more readable and maintainable than the old 
function-call syntax with complex escaping.
```

```action
button: Unified Implementation

All elements now use the same YAML parameter parsing approach, making the 
codebase more consistent and easier to extend.
```

## Testing the Implementation

The new system passes all core functionality tests and works seamlessly with the VSCode extension. The HTML output remains identical, so no changes were needed to the frontend!

🎉 **Triple-tickification is complete and working!**"#;

 let result = parser.parse_and_normalize(markdown).await.unwrap();
        
        // Should contain the comment HTML element
        expect_test::expect![[r#"
            <h1>Triple-Tickification: Complete Implementation Walkthrough</h1>
            <p>We've successfully implemented the complete transition from XML syntax to markdown code blocks! Let's walk through all the key components.</p>
            <h2>Architecture Overview</h2>
            <p>The new YAML-style parsing architecture handles all four element types:</p>
            <mermaid>flowchart TD
                A[Markdown Input] --> B{Code Block?}
                B -->|Regular| C[Standard Markdown]
                B -->|Special| D[Parse Language ID]
                D --> E{Known Element?}
                E -->|mermaid| F[Direct Content]
                E -->|comment/gitdiff/action| G[Parse YAML Parameters]
                F --> H[Create XML Element]
                G --> H
                H --> I[Resolve & Generate HTML]
                C --> J[Final HTML Output]
                I --> J
            </mermaid>
            <h2>Core Implementation: YAML Parameter Parser</h2>
            <p>The heart of the new system parses YAML-style parameters cleanly:</p>
            <div class="comment-item" data-comment="{&quot;comment&quot;:[&quot;This function separates YAML parameters from content by processing lines sequentially.\nIt stops at the first empty line or non-YAML line, ensuring clean parameter extraction.\nThe key fix was replacing the flawed logic that mixed parameters with content.&quot;],&quot;id&quot;:&quot;comment-test-uuid&quot;,&quot;locations&quot;:[]}" style="cursor: pointer; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 8px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                            <div style="display: flex; align-items: flex-start;">
                                <div class="comment-icon" style="margin-right: 8px; font-size: 16px;">⚙️</div>
                                <div class="comment-content" style="flex: 1;">
                                    <div class="comment-expression" style="display: block; color: var(--vscode-textLink-foreground); font-family: var(--vscode-editor-font-family); font-size: 1.0em; font-weight: 500; margin-bottom: 6px; text-decoration: underline;">`parse_yaml_parameters`</div>
                                    <div class="comment-locations" style="font-weight: 500; color: var(--vscode-textLink-foreground); margin-bottom: 4px; font-family: var(--vscode-editor-font-family); font-size: 0.9em;">no location</div>
                                    <div class="comment-text" style="color: var(--vscode-foreground); font-size: 0.9em;">This function separates YAML parameters from content by processing lines sequentially.
            It stops at the first empty line or non-YAML line, ensuring clean parameter extraction.
            The key fix was replacing the flawed logic that mixed parameters with content.</div>
                                </div>
                            </div>
                        </div>
            <h2>Element Processing Pipeline</h2>
            <p>Each code block type gets processed through a unified pipeline:</p>
            <div class="comment-item" data-comment="{&quot;comment&quot;:[&quot;The processing pipeline handles all four element types (mermaid, comment, gitdiff, action)\nwith a unified approach. YAML parameters are extracted first, then the appropriate\nXML element is created and resolved through the existing HTML generation system.&quot;],&quot;id&quot;:&quot;comment-test-uuid&quot;,&quot;locations&quot;:[]}" style="cursor: pointer; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 8px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                            <div style="display: flex; align-items: flex-start;">
                                <div class="comment-icon" style="margin-right: 8px; font-size: 16px;">💬</div>
                                <div class="comment-content" style="flex: 1;">
                                    <div class="comment-expression" style="display: block; color: var(--vscode-textLink-foreground); font-family: var(--vscode-editor-font-family); font-size: 1.0em; font-weight: 500; margin-bottom: 6px; text-decoration: underline;">`process_code_block`</div>
                                    <div class="comment-locations" style="font-weight: 500; color: var(--vscode-textLink-foreground); margin-bottom: 4px; font-family: var(--vscode-editor-font-family); font-size: 0.9em;">no location</div>
                                    <div class="comment-text" style="color: var(--vscode-foreground); font-size: 0.9em;">The processing pipeline handles all four element types (mermaid, comment, gitdiff, action)
            with a unified approach. YAML parameters are extracted first, then the appropriate
            XML element is created and resolved through the existing HTML generation system.</div>
                                </div>
                            </div>
                        </div>
            <h2>New Syntax Examples</h2>
            <p>Here are examples of all four element types in the new YAML-style format:</p>
            <div class="comment-item" data-comment="{&quot;comment&quot;:[&quot;&quot;],&quot;id&quot;:&quot;comment-test-uuid&quot;,&quot;locations&quot;:[]}" style="cursor: pointer; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 8px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                            <div style="display: flex; align-items: flex-start;">
                                <div class="comment-icon" style="margin-right: 8px; font-size: 16px;">💡</div>
                                <div class="comment-content" style="flex: 1;">
                                    <div class="comment-expression" style="display: block; color: var(--vscode-textLink-foreground); font-family: var(--vscode-editor-font-family); font-size: 1.0em; font-weight: 500; margin-bottom: 6px; text-decoration: underline;">/comment/</div>
                                    <div class="comment-locations" style="font-weight: 500; color: var(--vscode-textLink-foreground); margin-bottom: 4px; font-family: var(--vscode-editor-font-family); font-size: 0.9em;">no location</div>
                                    <div class="comment-text" style="color: var(--vscode-foreground); font-size: 0.9em;"></div>
                                </div>
                            </div>
                        </div>
            <p>Comments now use clean YAML parameters:</p>
            <div class="comment-item" data-comment="{&quot;comment&quot;:[&quot;This explains the User struct&quot;],&quot;id&quot;:&quot;comment-test-uuid&quot;,&quot;locations&quot;:[{&quot;content&quot;:&quot;struct User {&quot;,&quot;end&quot;:{&quot;column&quot;:4,&quot;line&quot;:10},&quot;path&quot;:&quot;src/models.rs&quot;,&quot;start&quot;:{&quot;column&quot;:0,&quot;line&quot;:10}}]}" style="cursor: pointer; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 8px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                            <div style="display: flex; align-items: flex-start;">
                                <div class="comment-icon" style="margin-right: 8px; font-size: 16px;">💬</div>
                                <div class="comment-content" style="flex: 1;">
                                    <div class="comment-expression" style="display: block; color: var(--vscode-textLink-foreground); font-family: var(--vscode-editor-font-family); font-size: 1.0em; font-weight: 500; margin-bottom: 6px; text-decoration: underline;">`User`</div>
                                    <div class="comment-locations" style="font-weight: 500; color: var(--vscode-textLink-foreground); margin-bottom: 4px; font-family: var(--vscode-editor-font-family); font-size: 0.9em;">src/models.rs:10</div>
                                    <div class="comment-text" style="color: var(--vscode-foreground); font-size: 0.9em;">This explains the User struct</div>
                                </div>
                            </div>
                        </div>
            <p>GitDiff elements support boolean flags:</p>
            <div class="gitdiff-container" style="border: 1px solid var(--vscode-panel-border); border-radius: 4px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                            <div style="padding: 12px; color: var(--vscode-descriptionForeground);">GitDiff rendering: HEAD~3..HEAD</div>
                        </div>
            <p>Actions have simple button parameters:</p>
            <button class="action-button" data-tell-agent="Should we execute the test suite now?" style="background-color: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; margin: 8px 0; font-size: 0.9em;">Run Tests</button>
            <h2>What We Accomplished</h2>
            <p>Here's the complete diff of our changes:</p>
            <div class="gitdiff-container" style="border: 1px solid var(--vscode-panel-border); border-radius: 4px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                            <div style="padding: 12px; color: var(--vscode-descriptionForeground);">GitDiff rendering: HEAD~15..HEAD</div>
                        </div>
            <h2>Key Benefits Achieved</h2>
            <button class="action-button" data-tell-agent="The simple language identifiers (comment, gitdiff, action, mermaid) work perfectly 
            with standard markdown parsers, fixing the compatibility issues we had with 
            complex function-call syntax." style="background-color: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; margin: 8px 0; font-size: 0.9em;">Better Markdown Compatibility</button><button class="action-button" data-tell-agent="YAML-style parameters are much more readable and maintainable than the old 
            function-call syntax with complex escaping." style="background-color: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; margin: 8px 0; font-size: 0.9em;">Cleaner Syntax</button><button class="action-button" data-tell-agent="All elements now use the same YAML parameter parsing approach, making the 
            codebase more consistent and easier to extend." style="background-color: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; margin: 8px 0; font-size: 0.9em;">Unified Implementation</button>
            <h2>Testing the Implementation</h2>
            <p>The new system passes all core functionality tests and works seamlessly with the VSCode extension. The HTML output remains identical, so no changes were needed to the frontend!</p>
            <p>🎉 <strong>Triple-tickification is complete and working!</strong></p>
        "#]].assert_eq(&result);
    }
}
