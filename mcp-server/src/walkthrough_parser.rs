use anyhow::Result;
use pulldown_cmark::{CowStr, Event, Parser, Tag, TagEnd, html};
use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;
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
                Event::InlineHtml(html) => {
                    if self.is_xml_element(&html) {
                        self.process_inline_xml(html, &mut input_events, &mut output_events)
                            .await?;
                    } else {
                        output_events.push(Event::InlineHtml(html));
                    }
                }
                Event::Start(Tag::HtmlBlock) => {
                    if self.is_xml_block(&input_events) {
                        self.process_xml_block(&mut input_events, &mut output_events)
                            .await?;
                    } else {
                        output_events.push(Event::Start(Tag::HtmlBlock));
                    }
                }
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

    /// Check if HTML content is one of our XML elements
    fn is_xml_element(&self, html: &str) -> bool {
        html.trim_start().starts_with("<comment")
            || html.trim_start().starts_with("<gitdiff")
            || html.trim_start().starts_with("<action")
            || html.trim_start().starts_with("</comment")
            || html.trim_start().starts_with("</gitdiff")
            || html.trim_start().starts_with("</action")
    }

    /// Check if upcoming events contain XML block content
    fn is_xml_block(&self, upcoming_events: &VecDeque<Event>) -> bool {
        if let Some(Event::Html(html)) = upcoming_events.front() {
            self.is_xml_element(html)
        } else {
            false
        }
    }

    /// Check if code block is one of our special types (mermaid, comment, etc.)
    fn is_special_code_block(&self, kind: &pulldown_cmark::CodeBlockKind) -> bool {
        match kind {
            pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                self.parse_code_block_language(lang).is_some()
            }
            _ => false,
        }
    }

    /// Parse code block language into identifier and parameters
    /// Supports: "identifier" or "identifier(param1="value1", param2="value2")"
    fn parse_code_block_language(&self, lang: &str) -> Option<(String, HashMap<String, String>)> {
        let lang = lang.trim();
        
        // Check if it has parameters (contains parentheses)
        if let Some(paren_pos) = lang.find('(') {
            let identifier = lang[..paren_pos].trim().to_string();
            
            // Only process known identifiers
            if !matches!(identifier.as_str(), "mermaid" | "comment" | "gitdiff" | "action") {
                return None;
            }
            
            // Extract parameter string (everything between parentheses)
            if let Some(close_paren) = lang.rfind(')') {
                let params_str = &lang[paren_pos + 1..close_paren];
                let params = self.parse_parameters(params_str);
                Some((identifier, params))
            } else {
                // Missing closing parenthesis
                None
            }
        } else {
            // No parameters, just identifier
            if matches!(lang, "mermaid" | "comment" | "gitdiff" | "action") {
                Some((lang.to_string(), HashMap::new()))
            } else {
                None
            }
        }
    }

    /// Parse parameter string like 'param1="value1", param2="value2", flag1, flag2'
    fn parse_parameters(&self, params_str: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        // Simple parser for comma-separated key="value" pairs and boolean flags
        let mut current_param = String::new();
        let mut in_quotes = false;
        let mut escape_next = false;
        
        for ch in params_str.chars() {
            if escape_next {
                current_param.push(ch);
                escape_next = false;
            } else if ch == '\\' {
                escape_next = true;
                current_param.push(ch);
            } else if ch == '"' {
                in_quotes = !in_quotes;
                current_param.push(ch);
            } else if ch == ',' && !in_quotes {
                // End of parameter
                self.parse_and_add_parameter(&current_param, &mut params);
                current_param.clear();
            } else {
                current_param.push(ch);
            }
        }
        
        // Handle last parameter
        if !current_param.trim().is_empty() {
            self.parse_and_add_parameter(&current_param, &mut params);
        }
        
        params
    }

    /// Parse and add a single parameter (either key="value" or boolean flag)
    fn parse_and_add_parameter(&self, param: &str, params: &mut HashMap<String, String>) {
        let param = param.trim();
        if param.is_empty() {
            return;
        }
        
        if let Some(eq_pos) = param.find('=') {
            // Key-value parameter
            let key = param[..eq_pos].trim().to_string();
            let value_part = param[eq_pos + 1..].trim();
            
            // Remove quotes if present
            let value = if value_part.starts_with('"') && value_part.ends_with('"') && value_part.len() >= 2 {
                value_part[1..value_part.len() - 1].to_string()
            } else {
                value_part.to_string()
            };
            
            params.insert(key, value);
        } else {
            // Boolean flag (no value)
            params.insert(param.to_string(), String::new());
        }
    }

    /// Process special code blocks (mermaid, comment, etc.)
    async fn process_code_block<'a>(
        &mut self,
        kind: pulldown_cmark::CodeBlockKind<'a>,
        input_events: &mut VecDeque<Event<'a>>,
        output_events: &mut Vec<Event<'a>>,
    ) -> Result<(), anyhow::Error> {
        // Extract the language/parameters from the code block
        let (element_type, params) = match &kind {
            pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                if let Some((id, params)) = self.parse_code_block_language(lang) {
                    (id, params)
                } else {
                    return Ok(()); // Not a special block, shouldn't happen
                }
            }
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

        // Create the appropriate XML element
        match element_type.as_str() {
            "mermaid" => {
                let xml_element = XmlElement::Mermaid { content };
                let resolved = self.resolve_single_element(xml_element).await?;
                let html = self.create_mermaid_html(&resolved);
                output_events.push(Event::InlineHtml(html.into()));
            }
            "comment" => {
                let location = params.get("location").cloned().unwrap_or_default();
                let icon = params.get("icon").cloned();
                let xml_element = XmlElement::Comment { location, icon, content };
                let resolved = self.resolve_single_element(xml_element).await?;
                let html = self.create_comment_html(&resolved);
                output_events.push(Event::InlineHtml(html.into()));
            }
            "gitdiff" => {
                let range = params.get("range").cloned().unwrap_or_default();
                let exclude_unstaged = params.contains_key("exclude-unstaged");
                let exclude_staged = params.contains_key("exclude-staged");
                let xml_element = XmlElement::GitDiff { range, exclude_unstaged, exclude_staged };
                let resolved = self.resolve_single_element(xml_element).await?;
                let html = self.create_gitdiff_html(&resolved);
                output_events.push(Event::InlineHtml(html.into()));
            }
            "action" => {
                let button = params.get("button").cloned().unwrap_or("Action".to_string());
                let xml_element = XmlElement::Action { button, message: content };
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

    /// Process inline XML elements (opening tag, content, closing tag)
    async fn process_inline_xml<'a>(
        &mut self,
        html: CowStr<'a>,
        input_events: &mut VecDeque<Event<'a>>,
        output_events: &mut Vec<Event<'a>>,
    ) -> Result<(), anyhow::Error> {
        // If this is a self-closing tag, handle it directly
        if html.contains("/>") {
            let xml_content = html.to_string();
            if let Ok(xml_element) = self.parse_xml_element(&xml_content) {
                let resolved = self.resolve_single_element(xml_element).await?;
                let normalized_xml = self.create_normalized_xml(&resolved);
                output_events.push(Event::InlineHtml(normalized_xml.into()));
            } else {
                output_events.push(Event::InlineHtml(html));
            }
            return Ok(());
        }

        // If this is a closing tag, pass it through (shouldn't happen in our flow)
        if html.starts_with("</") {
            output_events.push(Event::InlineHtml(html));
            return Ok(());
        }

        // This is an opening tag - collect all events until closing tag
        let mut content_events = Vec::new();

        while let Some(event) = input_events.pop_front() {
            match event {
                Event::InlineHtml(closing_html) if closing_html.starts_with("</") => {
                    // Found closing tag - render collected content and create complete XML
                    let mut content_html = String::new();
                    html::push_html(&mut content_html, content_events.iter().cloned());

                    // Try to parse just the opening tag to get attributes
                    if let Ok(xml_element) =
                        self.parse_xml_element(&format!("{}</{}>", html, &closing_html[2..]))
                    {
                        let resolved = self.resolve_single_element(xml_element).await?;

                        // Create the resolved XML with the rendered content
                        let mut attrs = String::new();
                        let resolved_json =
                            serde_json::to_string(&resolved.resolved_data).unwrap_or_default();
                        attrs.push_str(&format!(" data-resolved='{}'", resolved_json));

                        for (key, value) in &resolved.attributes {
                            attrs.push_str(&format!(" {}=\"{}\"", key, value));
                        }

                        let tag_name = resolved.element_type;
                        let normalized_xml =
                            format!("<{}{}>{}{}", tag_name, attrs, content_html, closing_html);
                        output_events.push(Event::InlineHtml(normalized_xml.into()));
                    } else {
                        // If parsing fails, pass through original
                        output_events.push(Event::InlineHtml(html));
                        output_events.extend(content_events);
                        output_events.push(Event::InlineHtml(closing_html));
                    }
                    return Ok(());
                }
                _ => {
                    // Collect all events between opening and closing tags
                    content_events.push(event);
                }
            }
        }

        // If we get here, no closing tag was found - pass through original
        output_events.push(Event::InlineHtml(html));
        output_events.extend(content_events);
        Ok(())
    }

    /// Process block-level XML elements
    async fn process_xml_block<'a>(
        &mut self,
        input_events: &mut VecDeque<Event<'a>>,
        output_events: &mut Vec<Event<'a>>,
    ) -> Result<(), anyhow::Error> {
        let mut xml_content = String::new();

        // Collect all HTML events until End(HtmlBlock)
        while let Some(event) = input_events.pop_front() {
            match event {
                Event::Html(html) => xml_content.push_str(&html),
                Event::End(TagEnd::HtmlBlock) => break,
                _ => {
                    // Put back unexpected event and break
                    input_events.push_front(event);
                    break;
                }
            }
        }

        // Parse and resolve the complete XML block
        if let Ok(xml_element) = self.parse_xml_element(&xml_content) {
            let resolved = self.resolve_single_element(xml_element).await?;
            let normalized_xml = self.create_normalized_xml(&resolved);

            // Emit as HTML block
            output_events.push(Event::Start(Tag::HtmlBlock));
            output_events.push(Event::Html(normalized_xml.into()));
            output_events.push(Event::End(TagEnd::HtmlBlock));
        } else {
            // If parsing fails, pass through original
            output_events.push(Event::Start(Tag::HtmlBlock));
            output_events.push(Event::Html(xml_content.into()));
            output_events.push(Event::End(TagEnd::HtmlBlock));
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
                use crate::synthetic_pr::git_service::GitService;

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

    fn parse_xml_element(&self, xml_text: &str) -> Result<XmlElement, anyhow::Error> {
        let mut reader = Reader::from_str(xml_text);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut element_name = String::new();
        let mut attributes = HashMap::new();
        let mut content = String::new();

        loop {
            match reader.read_event_into(&mut buf)? {
                XmlEvent::Start(e) => {
                    element_name = String::from_utf8(e.name().as_ref().to_vec())?;

                    // Parse attributes
                    for attr in e.attributes() {
                        let attr = attr?;
                        let key = String::from_utf8(attr.key.as_ref().to_vec())?;
                        let value = String::from_utf8(attr.value.to_vec())?;
                        attributes.insert(key, value);
                    }
                }
                XmlEvent::Text(e) => {
                    content = std::str::from_utf8(&e)?.to_string();
                }
                XmlEvent::End(_) => break,
                XmlEvent::Empty(e) => {
                    element_name = String::from_utf8(e.name().as_ref().to_vec())?;

                    // Parse attributes for self-closing tags
                    for attr in e.attributes() {
                        let attr = attr?;
                        let key = String::from_utf8(attr.key.as_ref().to_vec())?;
                        let value = String::from_utf8(attr.value.to_vec())?;
                        attributes.insert(key, value);
                    }
                    break;
                }
                XmlEvent::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        // Convert to appropriate XmlElement variant
        match element_name.as_str() {
            "comment" => Ok(XmlElement::Comment {
                location: attributes.get("location").unwrap_or(&String::new()).clone(),
                icon: attributes.get("icon").cloned(),
                content,
            }),
            "gitdiff" => Ok(XmlElement::GitDiff {
                range: attributes.get("range").unwrap_or(&String::new()).clone(),
                exclude_unstaged: attributes.contains_key("exclude-unstaged"),
                exclude_staged: attributes.contains_key("exclude-staged"),
            }),
            "action" => Ok(XmlElement::Action {
                button: attributes.get("button").unwrap_or(&String::new()).clone(),
                message: content,
            }),
            _ => Err(anyhow::anyhow!("Unknown XML element: {}", element_name)),
        }
    }

    /// Create final HTML element with resolved data
    fn create_normalized_xml(&self, resolved: &ResolvedXmlElement) -> String {
        match resolved.element_type.as_str() {
            "comment" => self.create_comment_html(resolved),
            "action" => self.create_action_html(resolved),
            "gitdiff" => self.create_gitdiff_html(resolved),
            _ => {
                // Fallback to original XML format for unknown types
                let mut attrs = String::new();
                let resolved_json =
                    serde_json::to_string(&resolved.resolved_data).unwrap_or_default();
                attrs.push_str(&format!(" data-resolved='{}'", resolved_json));

                for (key, value) in &resolved.attributes {
                    attrs.push_str(&format!(" {}=\"{}\"", key, value));
                }

                if resolved.content.is_empty() {
                    format!("<{}{} />", resolved.element_type, attrs)
                } else {
                    format!(
                        "<{}{}>{}</{}>",
                        resolved.element_type, attrs, resolved.content, resolved.element_type
                    )
                }
            }
        }
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
            r#"<comment location="findDefinitions(`User`)">User struct</comment>"#,
            expect![[r#"
                <p><comment data-resolved='{"dialect_expression":"findDefinitions(`User`)","locations":[{"definedAt":{"content":"struct User {","end":{"column":4,"line":10},"path":"src/models.rs","start":{"column":0,"line":10}},"kind":"struct","name":"User"}]}'>User struct</comment></p>
            "#]],
        );
    }

    #[test]
    fn test_self_closing_gitdiff() {
        check(
            r#"<gitdiff range="HEAD~1..HEAD" />"#,
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
                <p><action data-resolved='{"button_text":"Next Step"}' button="Next Step">What should we do next?</action></p>
            "#]],
        );
    }

    #[test]
    fn test_full_walkthrough_with_mixed_content() {
        check(
            r#"# My Walkthrough

This is some markdown content.

<comment location="findDefinitions(`User`)" icon="lightbulb">
This explains the User struct
</comment>

More markdown here.

<gitdiff range="HEAD~1..HEAD" />

<action button="Next Step">What should we do next?</action>"#,
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
                            </div>
                <p><action data-resolved='{"button_text":"Next Step"}' button="Next Step">What should we do next?</action></p>
            "#]],
        );
    }

    #[test]
    fn test_markdown_structure_preservation() {
        check(
            r#"# Title
Some text before
<comment location="findDefinitions(`User`)">User comment</comment>
Some text after
<gitdiff range="HEAD" />
More text"#,
            expect![[r#"
                <h1>Title</h1>
                <p>Some text before
                <comment data-resolved='{"dialect_expression":"findDefinitions(`User`)","locations":[{"definedAt":{"content":"struct User {","end":{"column":4,"line":10},"path":"src/models.rs","start":{"column":0,"line":10}},"kind":"struct","name":"User"}]}'>User comment</comment>
                Some text after
                <div class="gitdiff-container" style="border: 1px solid var(--vscode-panel-border); border-radius: 4px; margin: 8px 0; background-color: var(--vscode-editor-background);">
                                <div style="padding: 12px; color: var(--vscode-descriptionForeground);">GitDiff rendering: HEAD</div>
                            </div>
                More text</p>
            "#]],
        );
    }

    #[test]
    fn test_markdown_inside_xml_elements() {
        check(
            r#"<comment location="findDefinitions(`User`)">This has *emphasis* and **bold** text</comment>"#,
            expect![[r#"
                <p><comment data-resolved='{"dialect_expression":"findDefinitions(`User`)","locations":[{"definedAt":{"content":"struct User {","end":{"column":4,"line":10},"path":"src/models.rs","start":{"column":0,"line":10}},"kind":"struct","name":"User"}]}'>This has <em>emphasis</em> and <strong>bold</strong> text</comment></p>
            "#]],
        );
    }
    #[test]
    fn test_parse_comment_element() {
        let parser = create_test_parser();
        let xml = r#"<comment location="findDefinitions(`User`)" icon="lightbulb">This is a comment</comment>"#;

        let element = parser.parse_xml_element(xml).unwrap();

        match element {
            XmlElement::Comment {
                location,
                icon,
                content,
            } => {
                assert_eq!(location, "findDefinitions(`User`)");
                assert_eq!(icon, Some("lightbulb".to_string()));
                assert_eq!(content, "This is a comment");
            }
            _ => panic!("Expected Comment element"),
        }
    }

    #[test]
    fn test_parse_self_closing_gitdiff() {
        let parser = create_test_parser();
        let xml = r#"<gitdiff range="HEAD~2..HEAD" exclude-unstaged="true" />"#;

        let element = parser.parse_xml_element(xml).unwrap();

        match element {
            XmlElement::GitDiff {
                range,
                exclude_unstaged,
                exclude_staged,
            } => {
                assert_eq!(range, "HEAD~2..HEAD");
                assert!(exclude_unstaged);
                assert!(!exclude_staged);
            }
            _ => panic!("Expected GitDiff element"),
        }
    }

    #[test]
    fn test_parse_action_element() {
        let parser = create_test_parser();
        let xml = r#"<action button="Test the changes">Run the test suite</action>"#;

        let element = parser.parse_xml_element(xml).unwrap();

        match element {
            XmlElement::Action { button, message } => {
                assert_eq!(button, "Test the changes");
                assert_eq!(message, "Run the test suite");
            }
            _ => panic!("Expected Action element"),
        }
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

    #[test]
    fn test_parse_code_block_language() {
        let parser = create_test_parser();
        
        // Test simple identifier
        let (id, params) = parser.parse_code_block_language("mermaid").unwrap();
        assert_eq!(id, "mermaid");
        assert!(params.is_empty());
        
        // Test identifier with parameters
        let (id, params) = parser.parse_code_block_language(r#"comment(location="findDefinition('foo')", icon="lightbulb")"#).unwrap();
        assert_eq!(id, "comment");
        assert_eq!(params.get("location").unwrap(), "findDefinition('foo')");
        assert_eq!(params.get("icon").unwrap(), "lightbulb");
        
        // Test unknown identifier
        assert!(parser.parse_code_block_language("unknown").is_none());
        
        // Test malformed parameters
        assert!(parser.parse_code_block_language("comment(missing_close").is_none());
    }

    #[test]
    fn test_parse_parameters() {
        let parser = create_test_parser();
        
        // Test simple parameters
        let params = parser.parse_parameters(r#"location="test", icon="info""#);
        assert_eq!(params.get("location").unwrap(), "test");
        assert_eq!(params.get("icon").unwrap(), "info");
        
        // Test parameters with complex values
        let params = parser.parse_parameters(r#"location="findDefinition('MyClass')", icon="lightbulb""#);
        assert_eq!(params.get("location").unwrap(), "findDefinition('MyClass')");
        assert_eq!(params.get("icon").unwrap(), "lightbulb");
        
        // Test single parameter
        let params = parser.parse_parameters(r#"button="Click me""#);
        assert_eq!(params.get("button").unwrap(), "Click me");
        
        // Test boolean flags
        let params = parser.parse_parameters(r#"range="HEAD~2", exclude-unstaged, exclude-staged"#);
        assert_eq!(params.get("range").unwrap(), "HEAD~2");
        assert!(params.contains_key("exclude-unstaged"));
        assert!(params.contains_key("exclude-staged"));
        assert_eq!(params.get("exclude-unstaged").unwrap(), ""); // Boolean flags have empty values
        
        // Test mixed parameters and flags
        let params = parser.parse_parameters(r#"location="test", readonly, icon="info""#);
        assert_eq!(params.get("location").unwrap(), "test");
        assert_eq!(params.get("icon").unwrap(), "info");
        assert!(params.contains_key("readonly"));
        
        // Test empty parameters
        let params = parser.parse_parameters("");
        assert!(params.is_empty());
    }

    #[tokio::test]
    async fn test_parse_gitdiff_code_block() {
        let mut parser = create_test_parser();
        let markdown = r#"# Test Walkthrough

Here's a git diff:

```gitdiff(range="HEAD~2..HEAD", exclude-unstaged, exclude-staged)
```

More content here."#;

        let result = parser.parse_and_normalize(markdown).await.unwrap();
        
        // Should contain the gitdiff HTML element
        assert!(result.contains("gitdiff-container"));
        assert!(result.contains("HEAD~2..HEAD"));
        assert!(result.contains("GitDiff rendering"));
    }

    #[tokio::test]
    async fn test_parse_action_code_block() {
        let mut parser = create_test_parser();
        let markdown = r#"# Test Walkthrough

Here's an action:

```action(button="Run Tests")
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
    async fn test_parse_comment_code_block() {
        let mut parser = create_test_parser();
        let markdown = r#"# Test Walkthrough

Here's a comment:

```comment(location="findDefinition('foo')", icon="lightbulb")
This explains the foo function
```

More content here."#;

        let result = parser.parse_and_normalize(markdown).await.unwrap();
        
        // Should contain the comment HTML element
        assert!(result.contains("data-comment"));
        assert!(result.contains("This explains the foo function"));
        assert!(result.contains("findDefinition('foo')"));
        assert!(result.contains("💡")); // lightbulb icon
    }
}
