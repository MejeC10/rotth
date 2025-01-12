use std::{io::Read, path::PathBuf};

use crate::{span::Span, Error, Result};
use chumsky::{prelude::*, text::Character, Error as CError, Stream};
use somok::Somok;

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Token {
    Bool(bool),
    Word(String),
    Str(String),
    Char(char),
    KeyWord(KeyWord),
    Num(String),
    Ignore,
    SigSep,
    Ptr,
    FieldAccess,
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(b) => write!(f, "{}", b),
            Self::Word(word) => write!(f, "{}", word),
            Self::Str(str) => write!(f, "{:?}", str),
            Self::Char(c) => write!(f, "{:?}", c),
            Self::KeyWord(keyword) => keyword.fmt(f),
            Self::Num(num) => write!(f, "{}", num),
            Self::Ignore => write!(f, "_"),
            Self::SigSep => write!(f, ":"),
            Self::Ptr => write!(f, "&>"),
            Self::FieldAccess => write!(f, "->"),
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum KeyWord {
    Include,
    Return,
    Cond,
    If,
    Else,
    Proc,
    While,
    Do,
    Bind,
    Const,
    Mem,
    Var,
    Struct,
    Cast,
    End,
}

impl std::fmt::Display for KeyWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

pub fn word_parser<C: Character, E: CError<C>>(
) -> impl Parser<C, C::Collection, Error = E> + Copy + Clone {
    const ALLOWED_NON_ALPHA: &[u8; 26] = b"(){}[]<>|\\/!@#$%^&*-=+_?.,";
    filter(|c: &C| {
        c.to_char().is_ascii_alphabetic() || ALLOWED_NON_ALPHA.contains(&(c.to_char() as u8))
    })
    .map(Some)
    .chain::<C, Vec<_>, _>(
        filter(|c: &C| {
            c.to_char().is_ascii_alphanumeric() || ALLOWED_NON_ALPHA.contains(&(c.to_char() as u8))
        })
        .repeated(),
    )
    .collect()
}

fn lexer() -> impl Parser<char, Vec<(Token, Span)>, Error = Simple<char, Span>>
where
{
    let escaped = just('\\').ignore_then(any()).map(|c| match c {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\\' => '\\',
        _ => panic!("Invalid escape sequence"),
    });

    let char = just('\'')
        .ignore_then(choice((escaped, any())))
        .then_ignore(just('\''))
        .map(Token::Char);

    let string = just('"')
        .ignore_then(none_of(['"']).repeated().collect())
        .then_ignore(just('"'))
        .map(|s: String| {
            let mut res = Vec::new();
            let mut escape = false;
            for b in s.into_bytes() {
                if escape {
                    match b {
                        b'n' => res.push(b'\n'),
                        b'r' => res.push(b'\r'),
                        b't' => res.push(b'\t'),
                        b'\\' => res.push(b'\\'),
                        _ => panic!("Invalid escape sequence \\{}!", b as char),
                    }
                    escape = false;
                } else if b == b'\\' {
                    escape = true;
                    continue;
                } else {
                    res.push(b)
                }
            }
            String::from_utf8(res).unwrap()
        })
        .map(Token::Str);

    let num = text::int(10).map(Token::Num);

    let word = word_parser().map(Token::Word);

    let bool = word_parser().try_map(|i: String, s| {
        Token::Bool(match i.as_str() {
            "true" => true,
            "false" => false,
            _ => return Simple::custom(s, "Invalid keyword".to_string()).error(),
        })
        .okay()
    });

    let keyword = word_parser().try_map(|i: String, s| {
        Token::KeyWord(match i.as_str() {
            "include" => KeyWord::Include,
            "return" => KeyWord::Return,
            "cond" => KeyWord::Cond,
            "if" => KeyWord::If,
            "else" => KeyWord::Else,
            "proc" => KeyWord::Proc,
            "while" => KeyWord::While,
            "do" => KeyWord::Do,
            "bind" => KeyWord::Bind,
            "const" => KeyWord::Const,
            "mem" => KeyWord::Mem,
            "var" => KeyWord::Var,
            "struct" => KeyWord::Struct,
            "cast" => KeyWord::Cast,
            "end" => KeyWord::End,
            _ => return Simple::custom(s, "Invalid keyword").error(),
        })
        .okay()
    });

    let ignore = word_parser().try_map(|i: String, s| match i.as_str() {
        "_" => Token::Ignore.okay(),
        _ => Simple::custom(s, "Invalid keyword").error(),
    });

    let ptr = just('&').ignore_then(just('>').ignored()).to(Token::Ptr);

    let sig_sep = just(':').to(Token::SigSep);
    let field_access = just('-').then(just('>')).to(Token::FieldAccess);

    let token = choice((
        num,
        char,
        string,
        field_access,
        ptr,
        sig_sep,
        ignore,
        bool,
        keyword,
        word,
    ))
    .recover_with(skip_then_retry_until([]));

    let comment = just(";").then(take_until(just('\n'))).padded();

    token
        .map_with_span(|a, b| (a, b))
        .padded_by(comment.repeated())
        .padded()
        .repeated()
}

pub fn lex(source: PathBuf) -> Result<Vec<(Token, Span)>> {
    let mut src = String::new();
    std::fs::File::open(&source)?.read_to_string(&mut src)?;

    match lexer().parse(Stream::from_iter(
        Span::new(source.to_string_lossy().into_owned(), src.len(), src.len()),
        src.chars()
            .enumerate()
            .map(|(i, c)| (c, Span::point(source.to_string_lossy().into_owned(), i))),
    )) {
        Ok(tokens) => tokens.okay(),
        Err(es) => Error::Lexer(es).error(),
    }
}

pub fn lex_string(source: String, file: PathBuf) -> Result<Vec<(Token, Span)>> {
    match lexer().parse(Stream::from_iter(
        Span::new(file.clone(), source.len(), source.len()),
        source
            .chars()
            .enumerate()
            .map(|(i, c)| (c, Span::point(file.clone(), i))),
    )) {
        Ok(tokens) => tokens.okay(),
        Err(es) => Error::Lexer(es).error(),
    }
}
