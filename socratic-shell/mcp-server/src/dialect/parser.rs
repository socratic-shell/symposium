use std::{collections::BTreeMap, iter::Peekable};

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum ParseError {
    #[error("Unexpected end of input")]
    UnexpectedEof { position: usize },
    #[error("Unexpected token: {token:?}")]
    UnexpectedToken { token: String, position: usize },
    #[error("Unexpected identifier without function call")]
    UnexpectedIdent { position: usize },
    #[error("Expected ')'")]
    ExpectedCloseParen { position: usize },
    #[error("Expected ']'")]
    ExpectedCloseBracket { position: usize },
    #[error("Expected '}}'")]
    ExpectedCloseBrace { position: usize },
    #[error("Expected key")]
    ExpectedKey { position: usize },
    #[error("Expected ':' after key")]
    ExpectedColon { position: usize },
    #[error("Expected string or identifier as key")]
    ExpectedStringOrIdent { position: usize },
    #[error("Unterminated string literal")]
    UnterminatedString { position: usize },
    #[error("Invalid escape sequence: \\{char}")]
    InvalidEscape { char: char, position: usize },
    #[error("Unterminated escape sequence")]
    UnterminatedEscape { position: usize },
    #[error("Unexpected character '{char}' following \"{preceding}\"")]
    UnexpectedChar {
        char: char,
        preceding: String,
        position: usize,
    },
}

#[derive(Debug)]
pub enum Ast {
    Call(String, Vec<Ast>),
    Int(u64),
    String(String),
    Boolean(bool),
    Array(Vec<Ast>),
    Object(BTreeMap<String, Ast>),
}

pub fn parse<'a>(input: &'a str) -> Result<Ast, ParseError> {
    let tokens = tokenize(input)?;
    let mut tokens = tokens.into_iter().peekable();
    let ast = parse_ast(&mut tokens, input)?;
    if let Some(token) = tokens.next() {
        return Err(ParseError::UnexpectedToken {
            token: format!("{:?}", token.kind),
            position: token.start,
        });
    }
    Ok(ast)
}

fn parse_ast(
    tokens: &mut Peekable<std::vec::IntoIter<Token<'_>>>,
    input: &str,
) -> Result<Ast, ParseError> {
    let token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEof { position: input.len() })?;

    match token.kind {
        TokenKind::Integer(n) => Ok(Ast::Int(n)),
        TokenKind::Boolean(b) => Ok(Ast::Boolean(b)),
        TokenKind::String(s) => Ok(Ast::String(s)),

        TokenKind::Ident(name) => {
            if tokens.peek().map(|t| &t.kind) == Some(&TokenKind::Sym('(')) {
                tokens.next(); // consume '('
                let mut args = Vec::new();

                while tokens.peek().map(|t| &t.kind) != Some(&TokenKind::Sym(')')) {
                    args.push(parse_ast(tokens, input)?);
                    if tokens.peek().map(|t| &t.kind) == Some(&TokenKind::Sym(',')) {
                        tokens.next(); // consume ','
                        // Allow trailing comma - if next token is ')', we're done
                        if tokens.peek().map(|t| &t.kind) == Some(&TokenKind::Sym(')')) {
                            break;
                        }
                    }
                }

                tokens.next().ok_or(ParseError::ExpectedCloseParen {
                    position: input.len(),
                })?;
                Ok(Ast::Call(name.to_string(), args))
            } else {
                Err(ParseError::UnexpectedIdent {
                    position: token.start,
                })
            }
        }

        TokenKind::Sym('[') => {
            let mut elements = Vec::new();

            while tokens.peek().map(|t| &t.kind) != Some(&TokenKind::Sym(']')) {
                elements.push(parse_ast(tokens, input)?);
                if tokens.peek().map(|t| &t.kind) == Some(&TokenKind::Sym(',')) {
                    tokens.next(); // consume ','
                    // Allow trailing comma - if next token is ']', we're done
                    if tokens.peek().map(|t| &t.kind) == Some(&TokenKind::Sym(']')) {
                        break;
                    }
                }
            }

            tokens.next().ok_or(ParseError::ExpectedCloseBracket {
                position: input.len(),
            })?;
            Ok(Ast::Array(elements))
        }

        TokenKind::Sym('{') => {
            let mut map = BTreeMap::new();

            while tokens.peek().map(|t| &t.kind) != Some(&TokenKind::Sym('}')) {
                let key_token = tokens.next().ok_or(ParseError::ExpectedCloseBrace {
                    position: if map.is_empty() {
                        token.end
                    } else {
                        input.len()
                    },
                })?;
                let key = match key_token.kind {
                    TokenKind::String(s) => s,
                    TokenKind::Ident(s) => s.to_string(),
                    _ => {
                        return Err(ParseError::ExpectedStringOrIdent {
                            position: key_token.start,
                        });
                    }
                };

                let colon_token = tokens.next().ok_or(ParseError::ExpectedColon {
                    position: input.len(),
                })?;
                if colon_token.kind != TokenKind::Sym(':') {
                    return Err(ParseError::ExpectedColon {
                        position: colon_token.start,
                    });
                }

                let value = parse_ast(tokens, input)?;
                map.insert(key, value);

                if tokens.peek().map(|t| &t.kind) == Some(&TokenKind::Sym(',')) {
                    tokens.next(); // consume ','
                    // Allow trailing comma - if next token is '}', we're done
                    if tokens.peek().map(|t| &t.kind) == Some(&TokenKind::Sym('}')) {
                        break;
                    }
                }
            }

            tokens.next().ok_or(ParseError::ExpectedCloseBrace {
                position: input.len(),
            })?;
            Ok(Ast::Object(map))
        }

        _ => Err(ParseError::UnexpectedToken {
            token: format!("{:?}", token.kind),
            position: token.start,
        }),
    }
}

#[derive(Debug)]
struct Token<'a> {
    kind: TokenKind<'a>,
    start: usize,
    end: usize,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum TokenKind<'a> {
    Ident(&'a str),
    Integer(u64),
    Boolean(bool),
    String(String),
    Sym(char),
    EOF,
}

fn tokenize<'a>(input: &'a str) -> Result<Vec<Token<'a>>, ParseError> {
    let mut tokens = Vec::new();
    let chars = &mut input.char_indices().peekable();

    while let Some((start_index, start_ch)) = chars.next() {
        if start_ch.is_digit(10) {
            let (end_index, num) = take_chars(input, start_index, chars, |c| c.is_digit(10));
            tokens.push(Token {
                kind: TokenKind::Integer(num.parse().unwrap()),
                start: start_index,
                end: end_index,
            });
            continue;
        }

        // Dear claude: fix the code below to create tokens

        if start_ch.is_alphabetic() {
            let (end_index, text) = take_chars(input, start_index, chars, |c| c.is_alphabetic());
            let kind = match text {
                "true" => TokenKind::Boolean(true),
                "false" => TokenKind::Boolean(false),
                _ => TokenKind::Ident(text),
            };
            tokens.push(Token {
                kind,
                start: start_index,
                end: end_index,
            });
            continue;
        }

        if start_ch.is_whitespace() {
            continue;
        }

        if start_ch == '"' || start_ch == '\'' || start_ch == '`' {
            let mut s = String::new();
            let mut end_index = start_index;
            while let Some((next_index, next_ch)) = chars.next() {
                if next_ch == start_ch {
                    end_index = next_index;
                    break;
                }

                // Escape:
                if next_ch == '\\' {
                    match chars.next() {
                        Some((_, 'n')) => s.push('\n'),
                        Some((_, 't')) => s.push('\t'),
                        Some((_, 'r')) => s.push('\r'),
                        Some((_, '"')) => s.push('"'),
                        Some((_, '\'')) => s.push('\''),
                        Some((_, '`')) => s.push('`'),
                        Some((_, '\\')) => s.push('\\'),
                        Some((_, c)) => {
                            return Err(ParseError::InvalidEscape {
                                char: c,
                                position: next_index,
                            });
                        }
                        None => {
                            return Err(ParseError::UnterminatedEscape {
                                position: next_index,
                            });
                        }
                    }
                } else {
                    s.push(next_ch);
                }
            }

            if end_index == start_index {
                return Err(ParseError::UnterminatedString {
                    position: start_index,
                });
            }

            tokens.push(Token {
                kind: TokenKind::String(s),
                start: start_index,
                end: end_index + 1,
            });
            continue;
        }

        if let '[' | ']' | '{' | '}' | '(' | ')' | ',' | ':' = start_ch {
            tokens.push(Token {
                kind: TokenKind::Sym(start_ch),
                start: start_index,
                end: start_index + 1,
            });
            continue;
        }

        return Err(ParseError::UnexpectedChar {
            char: start_ch,
            preceding: input[..start_index].to_string(),
            position: start_index,
        });
    }

    Ok(tokens)
}

/// Given an iterator `chars` over the the input `input`,
/// keep taking chars so long as `op(ch)` is true,
/// then return `&input[c_index..X]` where `X` is the index
/// of the next character.
fn take_chars<'i>(
    input: &'i str,
    c_index: usize,
    chars: &mut Peekable<impl Iterator<Item = (usize, char)>>,
    op: impl Fn(char) -> bool,
) -> (usize, &'i str) {
    let mut end_index = input.len();
    while let Some((next_index, next_ch)) = chars.peek() {
        if op(*next_ch) {
            chars.next();
            continue;
        }

        end_index = *next_index;
        break;
    }

    (end_index, &input[c_index..end_index])
}

#[cfg(test)]
mod tests {
    use super::*;
    use annotate_snippets::{Level, Renderer, Snippet};
    use expect_test::{Expect, expect};

    fn check_parse(input: &str, expected: Expect) {
        let result = parse(input).unwrap();
        expected.assert_debug_eq(&result);
    }

    fn check_parse_error(input: &str, expected: Expect) {
        let result = parse(input);
        match result {
            Err(error) => {
                let position = match &error {
                    ParseError::UnexpectedEof { position } => *position,
                    ParseError::UnexpectedToken { position, .. } => *position,
                    ParseError::UnexpectedIdent { position } => *position,
                    ParseError::ExpectedCloseParen { position } => *position,
                    ParseError::ExpectedCloseBracket { position } => *position,
                    ParseError::ExpectedCloseBrace { position } => *position,
                    ParseError::ExpectedKey { position } => *position,
                    ParseError::ExpectedColon { position } => *position,
                    ParseError::ExpectedStringOrIdent { position } => *position,
                    ParseError::UnterminatedString { position } => *position,
                    ParseError::InvalidEscape { position, .. } => *position,
                    ParseError::UnterminatedEscape { position } => *position,
                    ParseError::UnexpectedChar { position, .. } => *position,
                };

                let error_message = error.to_string();
                let message = Level::Error.title(&error_message).snippet(
                    Snippet::source(input)
                        .annotation(Level::Error.span(position..position.saturating_add(1))),
                );

                let renderer = Renderer::plain();
                let output = renderer.render(message).to_string();
                expected.assert_eq(&output);
            }
            Ok(_) => panic!("Expected parse error, but parsing succeeded"),
        }
    }

    #[test]
    fn test_parse_function_call() {
        check_parse(
            "foo(42, \"hello\")",
            expect![[r#"
                Call(
                    "foo",
                    [
                        Int(
                            42,
                        ),
                        String(
                            "hello",
                        ),
                    ],
                )
            "#]],
        );
    }

    #[test]
    fn test_backtick_strings() {
        check_parse(
            "findDefinition(`validateToken`)",
            expect![[r#"
                Call(
                    "findDefinition",
                    [
                        String(
                            "validateToken",
                        ),
                    ],
                )
            "#]],
        );
    }

    #[test]
    fn test_parse_array() {
        check_parse(
            "[1, 2, 3]",
            expect![[r#"
                Array(
                    [
                        Int(
                            1,
                        ),
                        Int(
                            2,
                        ),
                        Int(
                            3,
                        ),
                    ],
                )
            "#]],
        );
    }

    #[test]
    fn test_parse_object() {
        check_parse(
            "{\"key\": 42}",
            expect![[r#"
                Object(
                    {
                        "key": Int(
                            42,
                        ),
                    },
                )
            "#]],
        );
    }

    #[test]
    fn test_parse_nested_structure() {
        check_parse(
            "process([{\"name\": \"test\", \"value\": 123}, true])",
            expect![[r#"
                Call(
                    "process",
                    [
                        Array(
                            [
                                Object(
                                    {
                                        "name": String(
                                            "test",
                                        ),
                                        "value": Int(
                                            123,
                                        ),
                                    },
                                ),
                                Boolean(
                                    true,
                                ),
                            ],
                        ),
                    ],
                )
            "#]],
        );
    }

    #[test]
    fn test_unexpected_token() {
        check_parse_error(
            "foo(42 extra)",
            expect![[r#"
                error: Unexpected identifier without function call
                  |
                1 | foo(42 extra)
                  |        ^
                  |"#]],
        );
    }

    #[test]
    fn test_unterminated_string() {
        check_parse_error(
            "\"unterminated",
            expect![[r#"
                error: Unterminated string literal
                  |
                1 | "unterminated
                  | ^
                  |"#]],
        );
    }

    #[test]
    fn test_missing_closing_paren() {
        check_parse_error(
            "foo(42",
            expect![[r#"
                error: Unexpected end of input
                  |
                1 | foo(42
                  |       ^
                  |"#]],
        );
    }

    #[test]
    fn test_missing_closing_bracket() {
        check_parse_error(
            "[1, 2",
            expect![[r#"
                error: Unexpected end of input
                  |
                1 | [1, 2
                  |      ^
                  |"#]],
        );
    }

    #[test]
    fn test_missing_closing_brace() {
        check_parse_error(
            "{\"key\": 42",
            expect![[r#"
                error: Expected '}'
                  |
                1 | {"key": 42
                  |           ^
                  |"#]],
        );
    }

    #[test]
    fn test_missing_colon_in_object() {
        check_parse_error(
            "{\"key\" 42}",
            expect![[r#"
                error: Expected ':' after key
                  |
                1 | {"key" 42}
                  |        ^
                  |"#]],
        );
    }

    #[test]
    fn test_invalid_escape_sequence() {
        check_parse_error(
            "\"invalid\\x\"",
            expect![[r#"
                error: Invalid escape sequence: \x
                  |
                1 | "invalid\x"
                  |         ^
                  |"#]],
        );
    }

    #[test]
    fn test_unexpected_character() {
        check_parse_error(
            "@invalid",
            expect![[r#"
                error: Unexpected character '@' following ""
                  |
                1 | @invalid
                  | ^
                  |"#]],
        );
    }

    #[test]
    fn test_identifier_without_call() {
        check_parse_error(
            "standalone",
            expect![[r#"
                error: Unexpected identifier without function call
                  |
                1 | standalone
                  | ^
                  |"#]],
        );
    }

    #[test]
    fn test_trailing_commas() {
        // Array with trailing comma
        check_parse(
            "[1, 2, 3,]",
            expect![[r#"
                Array(
                    [
                        Int(
                            1,
                        ),
                        Int(
                            2,
                        ),
                        Int(
                            3,
                        ),
                    ],
                )
            "#]],
        );

        // Object with trailing comma
        check_parse(
            "{\"key\": 42,}",
            expect![[r#"
                Object(
                    {
                        "key": Int(
                            42,
                        ),
                    },
                )
            "#]],
        );

        // Function call with trailing comma
        check_parse(
            "foo(42, \"hello\",)",
            expect![[r#"
                Call(
                    "foo",
                    [
                        Int(
                            42,
                        ),
                        String(
                            "hello",
                        ),
                    ],
                )
            "#]],
        );
    }

    #[test]
    fn test_mismatched_delimiters() {
        check_parse_error(
            "[1}",
            expect![[r#"
                error: Unexpected token: "Sym('}')"
                  |
                1 | [1}
                  |   ^
                  |"#]],
        );
    }

    #[test]
    fn test_extra_tokens() {
        check_parse_error(
            "42 extra",
            expect![[r#"
                error: Unexpected token: "Ident(\"extra\")"
                  |
                1 | 42 extra
                  |    ^
                  |"#]],
        );
    }
}
