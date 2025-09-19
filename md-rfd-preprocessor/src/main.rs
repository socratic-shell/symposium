use anyhow::Context;
use clap::Parser;
use mdbook::book::{Book, BookItem, Chapter};
use mdbook::errors::Error;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use regex::Regex;
use semver::{Version, VersionReq};
use serde_json;
use std::collections::HashMap;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(clap::Parser, Debug)]
#[structopt(about = "Project goal preprocessor")]
struct Opt {
    #[command(subcommand)]
    cmd: Option<Command>,
}

#[derive(clap::Subcommand, Debug)]
#[allow(dead_code)]
enum Command {
    /// Command used by mdbook to check if the preprocessor supports a renderer
    Supports { renderer: String },
}

pub struct RfdPreprocessor;

impl RfdPreprocessor {
    /// A convenience function custom preprocessors can use to parse the input
    /// written to `stdin` by a `CmdRenderer`.
    pub fn parse_input<R: Read>(reader: R) -> anyhow::Result<(PreprocessorContext, Book)> {
        serde_json::from_reader(reader).with_context(|| "Unable to parse the input")
    }
}

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

struct RfdChapterInfo {
    label: String,
    path: PathBuf,
}

impl RfdChapterInfo {
    fn color(&self) -> String {
        match &self.label[..] {
            "Draft" => "blue",
            "Preview" => "orange",
            "Completed" => "green",
            "To be removed" => "red",
            _ => "gray",
        }
        .to_string()
    }
}

fn parse_rfd_sections(book: &Book) -> Result<HashMap<String, RfdChapterInfo>, Error> {
    let mut sections = HashMap::new();

    // Iterate over each chapter...
    for chapter in book.iter() {
        match chapter {
            BookItem::Separator => (),
            BookItem::PartTitle(_) => (),

            // ...looking for chapters with a path like `.../rfds/rfd-name.md`...
            BookItem::Chapter(chapter) => {
                let Some(path) = &chapter.path else {
                    continue;
                };

                if !path.components().any(|c| c.as_os_str() == "rfds") {
                    continue;
                }

                let Some(rfd_name) = path.file_stem() else {
                    continue;
                };

                let Some(rfd_name) = rfd_name.to_str() else {
                    continue;
                };

                match chapter.parent_names.last() {
                    None => {
                        // The chapter is not part of any sub-chapter.
                        //
                        // This takes place in the "index" or "TEMPLATE", for example
                    }
                    Some(parent_section) => {
                        sections.insert(
                            rfd_name.to_string(),
                            RfdChapterInfo {
                                label: parent_section.to_string(),
                                path: path.to_owned(),
                            },
                        );
                    }
                }
            }
        }
    }

    Ok(sections)
}

fn process_chapter(chapter: &mut Chapter, rfd_sections: &HashMap<String, RfdChapterInfo>) {
    let re = Regex::new(r"\{RFD:([^}]+)\}").unwrap();

    chapter.content = re
        .replace_all(&chapter.content, |caps: &regex::Captures| {
            let rfd_name = &caps[1];

            if rfd_name == "rfd-name" {
                // We use this for demonstration purposes. Just print it as is.
                return format!("{{RFD:{rfd_name}}}");
            }

            // Otherwise, look up the RFC to see what section it belongs in.
            let Some(rfd_chapter) = rfd_sections.get(rfd_name) else {
                // This RFD doesn't seem to be in a section!
                eprintln!("WARNING: RFD `{rfd_name}` not found.");
                return format!("{{RFD:{rfd_name}}}");
            };

            // Format as a link to the md chapter
            format!(
                "[![{label}: {name}](https://img.shields.io/badge/{elabel}-{ename}-{color})](/{path})",
                label = rfd_chapter.label,
                name = rfd_name,
                elabel = rfd_chapter.label.replace(" ", "%20"),
                ename = rfd_name.replace("-", "--"),
                color = rfd_chapter.color(),
                path = rfd_chapter.path.display(),
            )
        })
        .to_string();
}

// from https://github.com/rust-lang/mdBook/blob/master/examples/nop-preprocessor.rs
fn handle_supports(pre: &dyn Preprocessor, renderer: &str) -> anyhow::Result<()> {
    let supported = pre.supports_renderer(renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        Ok(())
    } else {
        anyhow::bail!("renderer `{}` unsupported", renderer)
    }
}

fn main() -> anyhow::Result<()> {
    match Opt::parse().cmd {
        Some(Command::Supports { renderer }) => {
            handle_supports(&RfdPreprocessor, &renderer)?;
        }
        None => {
            let (ctx, book) = RfdPreprocessor::parse_input(io::stdin())?;

            let book_version = Version::parse(&ctx.mdbook_version)?;
            let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;

            if !version_req.matches(&book_version) {
                eprintln!(
                    "Warning: The {} plugin was built against version {} of mdbook, \
             but we're being called from version {}",
                    RfdPreprocessor.name(),
                    mdbook::MDBOOK_VERSION,
                    ctx.mdbook_version
                );
            }

            let processed_book = RfdPreprocessor.run(&ctx, book)?;

            let output = serde_json::to_string(&processed_book)?;
            println!("{output}");
        }
    }
    Ok(())
}
