use mdbook::book::{Book, BookItem, Chapter};
use mdbook::errors::Error;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use regex::Regex;
use serde_json;
use std::collections::HashMap;
use std::io::{self, Read};
use std::process;

pub struct RfdPreprocessor;

impl Preprocessor for RfdPreprocessor {
    fn name(&self) -> &str {
        "rfd-preprocessor"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        let rfd_sections = parse_rfd_sections(&book)?;

        book.for_each_mut(|item| {
            if let BookItem::Chapter(chapter) = item {
                process_chapter(chapter, &rfd_sections);
            }
        });

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}

fn parse_rfd_sections(book: &Book) -> Result<HashMap<String, String>, Error> {
    let mut sections = HashMap::new();

    // Find SUMMARY.md content
    if let Some(summary) = book.iter().find_map(|item| {
        if let BookItem::Chapter(ch) = item {
            if ch.name == "Summary"
                || ch
                    .source_path
                    .as_ref()
                    .map(|p| p.ends_with("SUMMARY.md"))
                    .unwrap_or(false)
            {
                Some(&ch.content)
            } else {
                None
            }
        } else {
            None
        }
    }) {
        let mut current_section = String::new();

        for line in summary.lines() {
            if line.contains("Draft") {
                current_section = "draft".to_string();
            } else if line.contains("Preview") {
                current_section = "preview".to_string();
            } else if line.contains("Completed") {
                current_section = "completed".to_string();
            } else if line.contains("To be removed") {
                current_section = "rejected".to_string();
            } else if line.contains("](./rfds/") {
                // Extract RFD name from markdown link
                if let Some(captures) = Regex::new(r"\]\(\./rfds/([^.]+)\.md\)")
                    .unwrap()
                    .captures(line)
                {
                    if let Some(rfd_name) = captures.get(1) {
                        sections.insert(rfd_name.as_str().to_string(), current_section.clone());
                    }
                }
            }
        }
    }

    Ok(sections)
}

fn process_chapter(chapter: &mut Chapter, rfd_sections: &HashMap<String, String>) {
    let re = Regex::new(r"\{RFD:([^}]+)\}").unwrap();

    chapter.content = re
        .replace_all(&chapter.content, |caps: &regex::Captures| {
            let rfd_name = &caps[1];
            let (color, label) = match rfd_sections.get(rfd_name).map(|s| s.as_str()) {
                Some("draft") => ("blue", "Draft"),
                Some("preview") => ("orange", "Preview"),
                Some("completed") => ("green", "Completed"),
                Some("rejected") => ("red", "To be removed"),
                _ => ("gray", "Unknown"),
            };

            format!(
                "[![{}: {}](https://img.shields.io/badge/{}-{}-{})](../rfds/{}.md)",
                label,
                rfd_name,
                label.replace(" ", "%20"),
                rfd_name.replace("-", "--"),
                color,
                rfd_name
            )
        })
        .to_string();
}

fn main() {
    let mut stdin = io::stdin();
    let mut buffer = String::new();

    if let Err(e) = stdin.read_to_string(&mut buffer) {
        eprintln!("Error reading from stdin: {}", e);
        process::exit(1);
    }

    let (ctx, book): (PreprocessorContext, Book) = match serde_json::from_str(&buffer) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Error parsing input: {}", e);
            process::exit(1);
        }
    };

    let preprocessor = RfdPreprocessor;
    let processed_book = match preprocessor.run(&ctx, book) {
        Ok(book) => book,
        Err(e) => {
            eprintln!("Error running preprocessor: {}", e);
            process::exit(1);
        }
    };

    match serde_json::to_string(&processed_book) {
        Ok(output) => println!("{}", output),
        Err(e) => {
            eprintln!("Error serializing output: {}", e);
            process::exit(1);
        }
    }
}
