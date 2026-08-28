//! Lossless Polyglot Tokenizer & Lexer.
//!
//! Converts source code into a linear sequence of `(SyntaxKind, &str)` tokens
//! preserving 100% of trivia (whitespace, newlines, comments) without losing a single byte.

#![forbid(unsafe_code)]

use crate::cst::green::SyntaxKind;

/// Tokenizes a source string into a 100% lossless stream of `(SyntaxKind, &str)`.
pub fn tokenize(source: &str) -> Vec<(SyntaxKind, &str)> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let start = i;
        let b = bytes[i];

        // 1. Whitespace (spaces, tabs)
        if b == b' ' || b == b'\t' {
            while i < len && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            tokens.push((SyntaxKind::Whitespace, &source[start..i]));
            continue;
        }

        // 2. Newlines
        if b == b'\n' {
            i += 1;
            tokens.push((SyntaxKind::Newline, &source[start..i]));
            continue;
        }
        if b == b'\r' {
            i += 1;
            if i < len && bytes[i] == b'\n' {
                i += 1;
            }
            tokens.push((SyntaxKind::Newline, &source[start..i]));
            continue;
        }

        // 3. Comments (Line //, Doc /// or //!, Block /* */, Python #)
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            i += 2;
            let is_doc = i < len && (bytes[i] == b'/' || bytes[i] == b'!');
            while i < len && bytes[i] != b'\n' && bytes[i] != b'\r' {
                i += 1;
            }
            let kind = if is_doc {
                SyntaxKind::DocComment
            } else {
                SyntaxKind::LineComment
            };
            tokens.push((kind, &source[start..i]));
            continue;
        }
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            let is_doc = i < len && bytes[i] == b'*';
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            } else {
                i = len;
            }
            let kind = if is_doc {
                SyntaxKind::DocComment
            } else {
                SyntaxKind::BlockComment
            };
            tokens.push((kind, &source[start..i]));
            continue;
        }
        if b == b'#' {
            i += 1;
            while i < len && bytes[i] != b'\n' && bytes[i] != b'\r' {
                i += 1;
            }
            tokens.push((SyntaxKind::LineComment, &source[start..i]));
            continue;
        }

        // 4. Raw Strings r#"..."# or r"..."
        if b == b'r' && i + 1 < len && (bytes[i + 1] == b'"' || bytes[i + 1] == b'#') {
            let mut hashes = 0;
            let mut j = i + 1;
            while j < len && bytes[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < len && bytes[j] == b'"' {
                i = j + 1;
                'raw_scan: while i < len {
                    if bytes[i] == b'"' {
                        let mut match_hashes = true;
                        for h in 0..hashes {
                            if i + 1 + h >= len || bytes[i + 1 + h] != b'#' {
                                match_hashes = false;
                                break;
                            }
                        }
                        if match_hashes {
                            i += 1 + hashes;
                            break 'raw_scan;
                        }
                    }
                    i += 1;
                }
                tokens.push((SyntaxKind::StringLiteral, &source[start..i]));
                continue;
            }
        }

        // 5. Standard Strings "..." and Template Literals `...`
        if b == b'"' || b == b'`' {
            let quote = b;
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            tokens.push((SyntaxKind::StringLiteral, &source[start..i]));
            continue;
        }

        // 6. Char Literal '...' or Lifetime 'a
        if b == b'\'' {
            if i + 2 < len && bytes[i + 2] == b'\'' && bytes[i + 1] != b'\\' {
                i += 3;
                tokens.push((SyntaxKind::CharLiteral, &source[start..i]));
                continue;
            } else if i + 3 < len && bytes[i + 1] == b'\\' && bytes[i + 3] == b'\'' {
                i += 4;
                tokens.push((SyntaxKind::CharLiteral, &source[start..i]));
                continue;
            } else if i + 1 < len && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_') {
                i += 1;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                tokens.push((SyntaxKind::Lifetime, &source[start..i]));
                continue;
            } else {
                // JS single quoted string
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'\'' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                tokens.push((SyntaxKind::StringLiteral, &source[start..i]));
                continue;
            }
        }

        // 7. Numbers (Hex, Binary, Octal, Decimal, Float)
        if b.is_ascii_digit() {
            if b == b'0'
                && i + 1 < len
                && (bytes[i + 1] == b'x' || bytes[i + 1] == b'b' || bytes[i + 1] == b'o')
            {
                i += 2;
                while i < len && (bytes[i].is_ascii_hexdigit() || bytes[i] == b'_') {
                    i += 1;
                }
            } else {
                while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                    i += 1;
                }
                if i < len
                    && bytes[i] == b'.'
                    && i + 1 < len
                    && bytes[i + 1].is_ascii_digit()
                    && (i + 2 >= len || bytes[i + 1] != b'.')
                {
                    i += 1;
                    while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                        i += 1;
                    }
                    tokens.push((SyntaxKind::FloatLiteral, &source[start..i]));
                    continue;
                }
            }
            tokens.push((SyntaxKind::IntLiteral, &source[start..i]));
            continue;
        }

        // 8. Multi-character Operators & Punctuation
        if b == b':' && i + 1 < len && bytes[i + 1] == b':' {
            i += 2;
            tokens.push((SyntaxKind::ColonColon, &source[start..i]));
            continue;
        }
        if b == b'?' && i + 1 < len && bytes[i + 1] == b'.' {
            i += 2;
            tokens.push((SyntaxKind::QuestionDot, &source[start..i]));
            continue;
        }
        if b == b'-' && i + 1 < len && bytes[i + 1] == b'>' {
            i += 2;
            tokens.push((SyntaxKind::Arrow, &source[start..i]));
            continue;
        }
        if b == b'=' && i + 1 < len && bytes[i + 1] == b'>' {
            i += 2;
            tokens.push((SyntaxKind::FatArrow, &source[start..i]));
            continue;
        }
        if b == b'=' && i + 1 < len && bytes[i + 1] == b'=' {
            i += 2;
            tokens.push((SyntaxKind::EqEq, &source[start..i]));
            continue;
        }
        if b == b'!' && i + 1 < len && bytes[i + 1] == b'=' {
            i += 2;
            tokens.push((SyntaxKind::BangEq, &source[start..i]));
            continue;
        }
        if b == b'<' && i + 1 < len && bytes[i + 1] == b'=' {
            i += 2;
            tokens.push((SyntaxKind::LtEq, &source[start..i]));
            continue;
        }
        if b == b'>' && i + 1 < len && bytes[i + 1] == b'=' {
            i += 2;
            tokens.push((SyntaxKind::GtEq, &source[start..i]));
            continue;
        }

        // 9. Single-character Punctuation
        let punct_kind = match b {
            b'(' => Some(SyntaxKind::OpenParen),
            b')' => Some(SyntaxKind::CloseParen),
            b'{' => Some(SyntaxKind::OpenBrace),
            b'}' => Some(SyntaxKind::CloseBrace),
            b'[' => Some(SyntaxKind::OpenBracket),
            b']' => Some(SyntaxKind::CloseBracket),
            b';' => Some(SyntaxKind::Semicolon),
            b':' => Some(SyntaxKind::Colon),
            b',' => Some(SyntaxKind::Comma),
            b'.' => Some(SyntaxKind::Dot),
            b'=' => Some(SyntaxKind::Eq),
            b'!' => Some(SyntaxKind::Excl),
            b'+' => Some(SyntaxKind::Plus),
            b'-' => Some(SyntaxKind::Minus),
            b'*' => Some(SyntaxKind::Star),
            b'/' => Some(SyntaxKind::Slash),
            b'%' => Some(SyntaxKind::Percent),
            b'&' => Some(SyntaxKind::Amp),
            b'|' => Some(SyntaxKind::Pipe),
            b'<' => Some(SyntaxKind::Lt),
            b'>' => Some(SyntaxKind::Gt),
            _ => None,
        };

        if let Some(kind) = punct_kind {
            i += 1;
            tokens.push((kind, &source[start..i]));
            continue;
        }

        // 10. Identifiers, Keywords, and Booleans
        if b.is_ascii_alphabetic() || b == b'_' || b == b'$' {
            i += 1;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$') {
                i += 1;
            }
            let word = &source[start..i];
            let kind = match word {
                "fn" => SyntaxKind::FnKw,
                "struct" => SyntaxKind::StructKw,
                "enum" => SyntaxKind::EnumKw,
                "trait" => SyntaxKind::TraitKw,
                "impl" => SyntaxKind::ImplKw,
                "type" => SyntaxKind::TypeKw,
                "pub" => SyntaxKind::PubKw,
                "async" => SyntaxKind::AsyncKw,
                "let" => SyntaxKind::LetKw,
                "const" => SyntaxKind::ConstKw,
                "mut" => SyntaxKind::MutKw,
                "if" => SyntaxKind::IfKw,
                "else" => SyntaxKind::ElseKw,
                "match" => SyntaxKind::MatchKw,
                "return" => SyntaxKind::ReturnKw,
                "use" => SyntaxKind::UseKw,
                "mod" => SyntaxKind::ModKw,
                "import" => SyntaxKind::ImportKw,
                "export" => SyntaxKind::ExportKw,
                "from" => SyntaxKind::FromKw,
                "default" => SyntaxKind::DefaultKw,
                "class" => SyntaxKind::ClassKw,
                "interface" => SyntaxKind::InterfaceKw,
                "extends" => SyntaxKind::ExtendsKw,
                "def" => SyntaxKind::DefKw,
                "true" | "false" => SyntaxKind::BoolLiteral,
                _ => SyntaxKind::Ident,
            };
            tokens.push((kind, word));
            continue;
        }

        // 11. Fallback single byte
        i += 1;
        tokens.push((SyntaxKind::Ident, &source[start..i]));
    }

    tokens
}
