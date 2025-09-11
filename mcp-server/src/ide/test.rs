#![cfg(test)]
use std::collections::BTreeMap;

use crate::{
    dialect::{DialectFunction, DialectInterpreter},
    ide::{FileLocation, FileRange, FindDefinitions, FindReferences, IpcClient, SymbolDef},
};
use serde::Deserialize;

// Mock IPC client for testing
#[derive(Clone)]
pub struct MockIpcClient {
    symbols: BTreeMap<String, Vec<SymbolDef>>,
    references: BTreeMap<String, Vec<FileRange>>,
}

impl MockIpcClient {
    pub fn new() -> Self {
        let mut symbols = BTreeMap::new();
        let mut references = BTreeMap::new();

        // Add some test data
        symbols.insert(
            "User".to_string(),
            vec![SymbolDef {
                name: "User".to_string(),
                kind: Some("struct".to_string()),
                defined_at: FileRange {
                    path: "src/models.rs".to_string(),
                    start: FileLocation { line: 10, column: 0 },
                    end: FileLocation { line: 10, column: 4 },
                    content: Some("struct User {".to_string()),
                },
            }],
        );

        symbols.insert(
            "validateToken".to_string(),
            vec![
                SymbolDef {
                    name: "validateToken".to_string(),
                    kind: Some("function".to_string()),
                    defined_at: FileRange {
                        path: "src/auth.rs".to_string(),
                        start: FileLocation { line: 42, column: 0 },
                        end: FileLocation { line: 42, column: 13 },
                        content: Some("fn validateToken(token: &str) -> bool {".to_string()),
                    },
                },
                SymbolDef {
                    name: "validateToken".to_string(),
                    kind: Some("function".to_string()),
                    defined_at: FileRange {
                        path: "src/utils.rs".to_string(),
                        start: FileLocation { line: 15, column: 0 },
                        end: FileLocation { line: 15, column: 13 },
                        content: Some("pub fn validateToken(token: String) -> Result<(), Error> {"
                            .to_string()),
                    },
                },
            ],
        );

        references.insert(
            "User".to_string(),
            vec![
                FileRange {
                    path: "src/auth.rs".to_string(),
                    start: FileLocation { line: 5, column: 12 },
                    end: FileLocation { line: 5, column: 16 },
                    content: Some("use models::User;".to_string()),
                },
                FileRange {
                    path: "src/handlers.rs".to_string(),
                    start: FileLocation { line: 23, column: 8 },
                    end: FileLocation { line: 23, column: 12 },
                    content: Some("fn create_user() -> User {".to_string()),
                },
            ],
        );

        Self {
            symbols,
            references,
        }
    }
}

impl IpcClient for MockIpcClient {
    async fn resolve_symbol_by_name(&mut self, name: &str) -> anyhow::Result<Vec<SymbolDef>> {
        Ok(self.symbols.get(name).cloned().unwrap_or_default())
    }

    async fn find_all_references(
        &mut self,
        symbol: &SymbolDef,
    ) -> anyhow::Result<Vec<FileRange>> {
        Ok(self
            .references
            .get(&symbol.name)
            .cloned()
            .unwrap_or_default())
    }

    fn generate_uuid(&self) -> String {
        "DUMMY_UUID".to_string()
    }
}

// IDE Function Tests
#[tokio::test]
async fn test_find_definition_with_string_symbol() {
    let mut interpreter = DialectInterpreter::new(MockIpcClient::new());
    interpreter.add_function::<FindDefinitions>();

    let result = interpreter.evaluate("findDefinitions(\"User\")").await.unwrap();
    let definitions: Vec<SymbolDef> = serde_json::from_value(result).unwrap();

    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].name, "User");
    assert_eq!(definitions[0].defined_at.path, "src/models.rs");
    assert_eq!(definitions[0].defined_at.start.line, 10);
}

#[tokio::test]
async fn test_find_definition_alias_singular() {
    let mut interpreter = DialectInterpreter::new(MockIpcClient::new());
    interpreter.add_function::<FindDefinitions>();
    interpreter.add_function_with_name::<FindDefinitions>("finddefinition");

    let result = interpreter.evaluate("findDefinition(\"User\")").await.unwrap();
    let definitions: Vec<SymbolDef> = serde_json::from_value(result).unwrap();

    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].name, "User");
    assert_eq!(definitions[0].defined_at.path, "src/models.rs");
    assert_eq!(definitions[0].defined_at.start.line, 10);
}

#[tokio::test]
async fn test_find_definition_with_to_string_symbol() {
    let mut interpreter = DialectInterpreter::new(MockIpcClient::new());
    interpreter.add_function::<FindDefinitions>();

    expect_test::expect![[r#"
        Ok(
            Array [
                Object {
                    "definedAt": Object {
                        "content": String("struct User {"),
                        "end": Object {
                            "column": Number(4),
                            "line": Number(10),
                        },
                        "path": String("src/models.rs"),
                        "start": Object {
                            "column": Number(0),
                            "line": Number(10),
                        },
                    },
                    "kind": String("struct"),
                    "name": String("User"),
                },
            ],
        )
    "#]]
    .assert_debug_eq(&interpreter.evaluate("findDefinitions(\"User\")").await);
}

#[tokio::test]
async fn test_find_definition_ambiguous_symbol() {
    let mut interpreter = DialectInterpreter::new(MockIpcClient::new());
    interpreter.add_function::<FindDefinitions>();

    expect_test::expect![[r#"
        Ok(
            Array [
                Object {
                    "definedAt": Object {
                        "content": String("fn validateToken(token: &str) -> bool {"),
                        "end": Object {
                            "column": Number(13),
                            "line": Number(42),
                        },
                        "path": String("src/auth.rs"),
                        "start": Object {
                            "column": Number(0),
                            "line": Number(42),
                        },
                    },
                    "kind": String("function"),
                    "name": String("validateToken"),
                },
                Object {
                    "definedAt": Object {
                        "content": String("pub fn validateToken(token: String) -> Result<(), Error> {"),
                        "end": Object {
                            "column": Number(13),
                            "line": Number(15),
                        },
                        "path": String("src/utils.rs"),
                        "start": Object {
                            "column": Number(0),
                            "line": Number(15),
                        },
                    },
                    "kind": String("function"),
                    "name": String("validateToken"),
                },
            ],
        )
    "#]].assert_debug_eq(&interpreter.evaluate("findDefinitions(\"validateToken\")").await);
}

#[tokio::test]
async fn test_find_references() {
    let mut interpreter = DialectInterpreter::new(MockIpcClient::new());
    interpreter.add_function::<FindReferences>();

    expect_test::expect![[r#"
        Ok(
            Array [
                Object {
                    "definedAt": Object {
                        "content": String("struct User {"),
                        "end": Object {
                            "column": Number(4),
                            "line": Number(10),
                        },
                        "path": String("src/models.rs"),
                        "start": Object {
                            "column": Number(0),
                            "line": Number(10),
                        },
                    },
                    "kind": String("struct"),
                    "name": String("User"),
                    "referencedAt": Object {
                        "content": String("use models::User;"),
                        "end": Object {
                            "column": Number(16),
                            "line": Number(5),
                        },
                        "path": String("src/auth.rs"),
                        "start": Object {
                            "column": Number(12),
                            "line": Number(5),
                        },
                    },
                },
                Object {
                    "definedAt": Object {
                        "content": String("struct User {"),
                        "end": Object {
                            "column": Number(4),
                            "line": Number(10),
                        },
                        "path": String("src/models.rs"),
                        "start": Object {
                            "column": Number(0),
                            "line": Number(10),
                        },
                    },
                    "kind": String("struct"),
                    "name": String("User"),
                    "referencedAt": Object {
                        "content": String("fn create_user() -> User {"),
                        "end": Object {
                            "column": Number(12),
                            "line": Number(23),
                        },
                        "path": String("src/handlers.rs"),
                        "start": Object {
                            "column": Number(8),
                            "line": Number(23),
                        },
                    },
                },
            ],
        )
    "#]]
    .assert_debug_eq(&interpreter.evaluate("findReferences(\"User\")").await);
}

#[tokio::test]
async fn test_symbol_not_found() {
    let mut interpreter = DialectInterpreter::new(MockIpcClient::new());
    interpreter.add_function::<FindDefinitions>();

    expect_test::expect![[r#"
        Ok(
            Array [],
        )
    "#]]
    .assert_debug_eq(&interpreter.evaluate("findDefinitions(\"NonExistentSymbol\")").await);
}

#[tokio::test]
async fn test_resolve_symbol_by_name_ipc() {
    let mut interpreter = DialectInterpreter::new(MockIpcClient::new());

    // Test that the IPC call is made correctly (MockIpcClient returns empty results)
    let result = interpreter.resolve_symbol_by_name("TestSymbol").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0); // MockIpcClient returns empty vec
}

#[tokio::test]
async fn test_find_all_references_ipc() {
    let mut interpreter = DialectInterpreter::new(MockIpcClient::new());

    let test_symbol = crate::ide::SymbolDef {
        name: "TestSymbol".to_string(),
        kind: Some("function".to_string()),
        defined_at: crate::ide::FileRange {
            path: "test.rs".to_string(),
            start: crate::ide::FileLocation { line: 10, column: 5 },
            end: crate::ide::FileLocation { line: 10, column: 18 },
            content: Some("fn test_function() {".to_string()),
        },
    };

    // Test that the IPC call is made correctly (MockIpcClient returns empty results)
    let result = interpreter.find_all_references(&test_symbol).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0); // MockIpcClient returns empty vec
}

// Simple test function - string manipulation
#[derive(Deserialize)]
struct Uppercase {
    text: String,
}

impl DialectFunction<()> for Uppercase {
    type Output = String;

    const PARAMETER_ORDER: &'static [&'static str] = &["text"];

    async fn execute(
        self,
        _interpreter: &mut DialectInterpreter<()>,
    ) -> anyhow::Result<Self::Output> {
        Ok(self.text.to_uppercase())
    }
}

// Test function with composition
#[derive(Deserialize)]
struct Concat {
    left: String,
    right: String,
}

impl DialectFunction<()> for Concat {
    type Output = String;

    const PARAMETER_ORDER: &'static [&'static str] = &["left", "right"];

    async fn execute(
        self,
        _interpreter: &mut DialectInterpreter<()>,
    ) -> anyhow::Result<Self::Output> {
        Ok(format!("{}{}", self.left, self.right))
    }
}

// Test function that returns a number
#[derive(Deserialize)]
struct Add {
    a: i32,
    b: i32,
}

impl DialectFunction<()> for Add {
    type Output = i32;

    const PARAMETER_ORDER: &'static [&'static str] = &["a", "b"];

    async fn execute(
        self,
        _interpreter: &mut DialectInterpreter<()>,
    ) -> anyhow::Result<Self::Output> {
        Ok(self.a + self.b)
    }
}

#[tokio::test]
async fn test_simple_function() {
    let mut interpreter = DialectInterpreter::new(());
    interpreter.add_function::<Uppercase>();

    let result = interpreter.evaluate("uppercase(\"hello\")").await.unwrap();

    assert_eq!(result, serde_json::json!("HELLO"));
}

#[tokio::test]
async fn test_function_composition() {
    let mut interpreter = DialectInterpreter::new(());
    interpreter.add_function::<Uppercase>();
    interpreter.add_function::<Concat>();

    let result = interpreter.evaluate("concat(uppercase(\"hello\"), \" world\")").await.unwrap();
    assert_eq!(result, serde_json::json!("HELLO world"));
}

#[tokio::test]
async fn test_nested_composition() {
    let mut interpreter = DialectInterpreter::new(());
    interpreter.add_function::<Add>();
    interpreter.add_function::<Uppercase>();

    let result = interpreter.evaluate("uppercase(\"hello world\")").await.unwrap();
    assert_eq!(result, serde_json::json!("HELLO WORLD"));
}

#[tokio::test]
async fn test_literal_values() {
    let mut interpreter = DialectInterpreter::new(());

    // Test that literal values pass through unchanged
    assert_eq!(
        interpreter
            .evaluate("\"hello\"")
            .await
            .unwrap(),
        serde_json::json!("hello")
    );
    assert_eq!(
        interpreter.evaluate("42").await.unwrap(),
        serde_json::json!(42)
    );
    assert_eq!(
        interpreter.evaluate("true").await.unwrap(),
        serde_json::json!(true)
    );
    assert_eq!(
        interpreter.evaluate("\"null\"").await.unwrap(),
        serde_json::json!("null")
    );
}

#[tokio::test]
async fn test_array_evaluation() {
    let mut interpreter = DialectInterpreter::new(());
    interpreter.add_function::<Add>();

    let result = interpreter.evaluate("[add(1, 2), add(3, 4), \"literal\"]").await.unwrap();
    assert_eq!(result, serde_json::json!([3, 7, "literal"]));
}

#[tokio::test]
async fn test_unknown_function_error() {
    let mut interpreter = DialectInterpreter::new(());

    let result = interpreter.evaluate("unknown(\"value\")").await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("unknown function: unknown")
    );
}

#[tokio::test]
async fn test_invalid_function_format() {
    let mut interpreter = DialectInterpreter::new(());

    // Invalid syntax should cause parse errors
    let result = interpreter.evaluate("func1() func2()").await;  // Invalid: two function calls without array
    assert!(result.is_err());

    // Function with invalid syntax
    let result = interpreter.evaluate("func(").await;  // Unclosed parenthesis
    assert!(result.is_err());
}

#[tokio::test]
async fn test_search_function() {
    use expect_test::expect;
    
    let mock_client = MockIpcClient::new();
    let mut interpreter = DialectInterpreter::new(mock_client);
    interpreter.add_function::<FindDefinitions>();
    interpreter.add_function::<FindReferences>();
    interpreter.add_function::<crate::ide::Search>();
    
    let result = interpreter.evaluate("search(\"nonexistent_file.rs\", \"fn\\\\s+\\\\w+\")").await;
    
    // Should return empty results since file doesn't exist
    expect![[r#"
        Ok(
            Array [],
        )
    "#]]
    .assert_debug_eq(&result);
}

#[tokio::test]
async fn test_gitdiff_function() {
    use test_utils::TestRepo;
    
    // Create a temporary git repo with some changes
    let temp_repo = TestRepo::new()
        .overwrite_and_add("src/main.rs", "fn main() {\n    println!(\"Hello\");\n}\n")
        .commit("Initial commit")
        .overwrite("src/main.rs", "fn main() {\n    println!(\"Hello, World!\");\n}\n")
        .add("src/main.rs")
        .commit("Update greeting")
        .create();
    
    let mock_client = MockIpcClient::new();
    let mut interpreter = DialectInterpreter::new(mock_client);
    interpreter.add_function::<FindDefinitions>();
    interpreter.add_function::<FindReferences>();
    interpreter.add_function::<crate::ide::Search>();
    interpreter.add_function::<crate::ide::GitDiff>();
    
    // Change to the temp repo directory
    let original_dir = crate::workspace_dir::current_dir().unwrap();
    std::env::set_current_dir(temp_repo.path()).unwrap();
    
    let result = interpreter.evaluate("gitDiff(\"HEAD~1..HEAD\")").await;
    
    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
    
    // Should succeed and return file changes
    assert!(result.is_ok());
    let changes = result.unwrap();
    
    // Verify the structure using expect-test
    use expect_test::expect;
    expect![[r#"
        Object {
            "files": Array [
                Object {
                    "additions": Number(1),
                    "deletions": Number(1),
                    "hunks": Array [
                        Object {
                            "header": String("@@ -1,3 +1,3 @@"),
                            "lines": Array [
                                Object {
                                    "content": String("fn main() {"),
                                    "line_type": String("Context"),
                                    "new_line_number": Number(1),
                                    "old_line_number": Number(1),
                                },
                                Object {
                                    "content": String("    println!(\"Hello\");"),
                                    "line_type": String("Removed"),
                                    "new_line_number": Null,
                                    "old_line_number": Number(2),
                                },
                                Object {
                                    "content": String("    println!(\"Hello, World!\");"),
                                    "line_type": String("Added"),
                                    "new_line_number": Number(2),
                                    "old_line_number": Null,
                                },
                                Object {
                                    "content": String("}"),
                                    "line_type": String("Context"),
                                    "new_line_number": Number(3),
                                    "old_line_number": Number(3),
                                },
                            ],
                            "new_lines": Number(3),
                            "new_start": Number(1),
                            "old_lines": Number(3),
                            "old_start": Number(1),
                        },
                    ],
                    "path": String("src/main.rs"),
                    "status": String("Modified"),
                },
            ],
        }
    "#]]
    .assert_debug_eq(&changes);
}

#[tokio::test]
async fn test_comment_function() {
    use expect_test::expect;
    
    let mock_client = MockIpcClient::new();
    let mut interpreter = DialectInterpreter::new(mock_client);
    interpreter.add_function::<FindDefinitions>();
    interpreter.add_function::<FindReferences>();
    interpreter.add_function::<crate::ide::Search>();
    interpreter.add_function::<crate::ide::GitDiff>();
    interpreter.add_function::<crate::ide::Comment>();
    
    // Test comment with direct FileRange location (wrapped as Dialect value)
    let result = interpreter.evaluate(r#"comment({
        path: "src/main.rs",
        start: {line: 10, column: 1},
        end: {line: 10, column: 20},
        content: "fn main() {"
    }, "info", ["This is the main function", "Entry point of the program"])"#).await;
    
    expect![[r#"
        Ok(
            Object {
                "comment": Array [
                    String("This is the main function"),
                    String("Entry point of the program"),
                ],
                "icon": String("info"),
                "id": String("DUMMY_UUID"),
                "locations": Array [
                    Object {
                        "content": String("fn main() {"),
                        "end": Object {
                            "column": Number(20),
                            "line": Number(10),
                        },
                        "path": String("src/main.rs"),
                        "start": Object {
                            "column": Number(1),
                            "line": Number(10),
                        },
                    },
                ],
            },
        )
    "#]]
    .assert_debug_eq(&result);
}

#[tokio::test]
async fn test_comment_function_with_symbol_def() {
    use expect_test::expect;
    
    let mock_client = MockIpcClient::new();
    let mut interpreter = DialectInterpreter::new(mock_client);
    interpreter.add_function::<FindDefinitions>();
    interpreter.add_function::<FindReferences>();
    interpreter.add_function::<crate::ide::Search>();
    interpreter.add_function::<crate::ide::GitDiff>();
    interpreter.add_function::<crate::ide::Comment>();
    
    // Test comment with SymbolDef location (should extract definedAt field)
    let result = interpreter.evaluate(r#"comment(findDefinitions("validateToken"), "warning", ["This function needs better error handling"])"#).await;
    
    // Should normalize SymbolDef to its definedAt FileRange
    expect![[r#"
        Ok(
            Object {
                "comment": Array [
                    String("This function needs better error handling"),
                ],
                "icon": String("warning"),
                "id": String("DUMMY_UUID"),
                "locations": Array [
                    Object {
                        "content": String("fn validateToken(token: &str) -> bool {"),
                        "end": Object {
                            "column": Number(13),
                            "line": Number(42),
                        },
                        "path": String("src/auth.rs"),
                        "start": Object {
                            "column": Number(0),
                            "line": Number(42),
                        },
                    },
                    Object {
                        "content": String("pub fn validateToken(token: String) -> Result<(), Error> {"),
                        "end": Object {
                            "column": Number(13),
                            "line": Number(15),
                        },
                        "path": String("src/utils.rs"),
                        "start": Object {
                            "column": Number(0),
                            "line": Number(15),
                        },
                    },
                ],
            },
        )
    "#]]
    .assert_debug_eq(&result);
}

#[tokio::test]
async fn test_action_function() {
    use expect_test::expect;
    
    let mock_client = MockIpcClient::new();
    let mut interpreter = DialectInterpreter::new(mock_client);
    interpreter.add_function::<FindDefinitions>();
    interpreter.add_function::<FindReferences>();
    interpreter.add_function::<crate::ide::Search>();
    interpreter.add_function::<crate::ide::GitDiff>();
    interpreter.add_function::<crate::ide::Comment>();
    interpreter.add_function::<crate::ide::Action>();
    
    // Test action with tell_agent
    let result = interpreter.evaluate(r#"action("Generate Auth", "Create a complete authentication system with login, logout, and middleware")"#).await;
    
    expect![[r#"
        Ok(
            Object {
                "button": String("Generate Auth"),
                "tell_agent": String("Create a complete authentication system with login, logout, and middleware"),
            },
        )
    "#]]
    .assert_debug_eq(&result);
}

#[tokio::test]
async fn test_lines_function() {
    use expect_test::expect;
    use std::fs;
    use tempfile::NamedTempFile;
    
    // Create a temporary file with known content
    let temp_file = NamedTempFile::new().unwrap();
    let content = "line 1\nline 2\nline 3\nline 4\nline 5\n";
    fs::write(&temp_file, content).unwrap();
    let file_path = temp_file.path().to_str().unwrap();
    
    let mock_client = MockIpcClient::new();
    let mut interpreter = DialectInterpreter::new(mock_client);
    interpreter.add_function::<crate::ide::Lines>();
    
    // Test selecting lines 2-4
    let query = format!(r#"lines("{}", 2, 4)"#, file_path);
    let result = interpreter.evaluate(&query).await;
    
    expect![[r#"
        Ok(
            Object {
                "content": String("line 2\nline 3\nline 4"),
                "end": Object {
                    "column": Number(6),
                    "line": Number(4),
                },
                "path": String("[TEMP_FILE_PATH]"),
                "start": Object {
                    "column": Number(1),
                    "line": Number(2),
                },
            },
        )
    "#]]
    .assert_debug_eq(&result.map(|mut v| {
        // Replace the actual temp file path with a placeholder for consistent testing
        if let Some(obj) = v.as_object_mut() {
            if let Some(path) = obj.get_mut("path") {
                *path = serde_json::json!("[TEMP_FILE_PATH]");
            }
        }
        v
    }));
}
