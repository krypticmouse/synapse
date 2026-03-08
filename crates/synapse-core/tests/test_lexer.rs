use logos::Logos;
use synapse_core::lexer::*;

#[test]
fn keywords() {
    let mut lex = Token::lexer("config memory namespace query update on fn for in policy none");
    assert_eq!(lex.next(), Some(Ok(Token::Config)));
    assert_eq!(lex.next(), Some(Ok(Token::Memory)));
    assert_eq!(lex.next(), Some(Ok(Token::Namespace)));
    assert_eq!(lex.next(), Some(Ok(Token::Query)));
    assert_eq!(lex.next(), Some(Ok(Token::Update)));
    assert_eq!(lex.next(), Some(Ok(Token::On)));
    assert_eq!(lex.next(), Some(Ok(Token::Fn)));
    assert_eq!(lex.next(), Some(Ok(Token::For)));
    assert_eq!(lex.next(), Some(Ok(Token::In)));
    assert_eq!(lex.next(), Some(Ok(Token::Policy)));
    assert_eq!(lex.next(), Some(Ok(Token::None)));
    assert_eq!(lex.next(), std::option::Option::None);
}

#[test]
fn decorators() {
    let mut lex = Token::lexer("@extern @index @invariant");
    assert_eq!(lex.next(), Some(Ok(Token::Extern)));
    assert_eq!(lex.next(), Some(Ok(Token::Index)));
    assert_eq!(lex.next(), Some(Ok(Token::Invariant)));
    assert_eq!(lex.next(), std::option::Option::None);
}

#[test]
fn types() {
    let mut lex = Token::lexer("string int float bool timestamp");
    assert_eq!(lex.next(), Some(Ok(Token::TyString)));
    assert_eq!(lex.next(), Some(Ok(Token::TyInt)));
    assert_eq!(lex.next(), Some(Ok(Token::TyFloat)));
    assert_eq!(lex.next(), Some(Ok(Token::TyBool)));
    assert_eq!(lex.next(), Some(Ok(Token::TyTimestamp)));
    assert_eq!(lex.next(), std::option::Option::None);
}

#[test]
fn float_literal() {
    let mut lex = Token::lexer("3.14 0.95 1.0");
    assert_eq!(lex.next(), Some(Ok(Token::FloatLiteral(3.14))));
    assert_eq!(lex.next(), Some(Ok(Token::FloatLiteral(0.95))));
    assert_eq!(lex.next(), Some(Ok(Token::FloatLiteral(1.0))));
    assert_eq!(lex.next(), std::option::Option::None);
}

#[test]
fn durations() {
    let mut lex = Token::lexer("5s 10m 2h 7d 3w");
    assert_eq!(lex.next(), Some(Ok(Token::DurationSec(5))));
    assert_eq!(lex.next(), Some(Ok(Token::DurationMin(10))));
    assert_eq!(lex.next(), Some(Ok(Token::DurationHour(2))));
    assert_eq!(lex.next(), Some(Ok(Token::DurationDay(7))));
    assert_eq!(lex.next(), Some(Ok(Token::DurationWeek(3))));
    assert_eq!(lex.next(), std::option::Option::None);
}

#[test]
fn int_literal() {
    let mut lex = Token::lexer("42 0 999");
    assert_eq!(lex.next(), Some(Ok(Token::IntLiteral(42))));
    assert_eq!(lex.next(), Some(Ok(Token::IntLiteral(0))));
    assert_eq!(lex.next(), Some(Ok(Token::IntLiteral(999))));
    assert_eq!(lex.next(), std::option::Option::None);
}

#[test]
fn double_quoted_strings() {
    let mut lex = Token::lexer(r#""hello world" "test""#);
    assert_eq!(
        lex.next(),
        Some(Ok(Token::StringLiteral("hello world".into())))
    );
    assert_eq!(lex.next(), Some(Ok(Token::StringLiteral("test".into()))));
    assert_eq!(lex.next(), std::option::Option::None);
}

#[test]
fn single_quoted_strings() {
    let mut lex = Token::lexer("'hello' 'world'");
    assert_eq!(lex.next(), Some(Ok(Token::StringLiteral("hello".into()))));
    assert_eq!(lex.next(), Some(Ok(Token::StringLiteral("world".into()))));
    assert_eq!(lex.next(), std::option::Option::None);
}

#[test]
fn comments_skipped() {
    let mut lex = Token::lexer("config # this is a comment\nmemory");
    assert_eq!(lex.next(), Some(Ok(Token::Config)));
    assert_eq!(lex.next(), Some(Ok(Token::Memory)));
    assert_eq!(lex.next(), std::option::Option::None);
}

#[test]
fn operators() {
    let mut lex = Token::lexer("+ - * / % = == != < <= > >= |>");
    assert_eq!(lex.next(), Some(Ok(Token::Plus)));
    assert_eq!(lex.next(), Some(Ok(Token::Minus)));
    assert_eq!(lex.next(), Some(Ok(Token::Star)));
    assert_eq!(lex.next(), Some(Ok(Token::Slash)));
    assert_eq!(lex.next(), Some(Ok(Token::Percent)));
    assert_eq!(lex.next(), Some(Ok(Token::Eq)));
    assert_eq!(lex.next(), Some(Ok(Token::EqEq)));
    assert_eq!(lex.next(), Some(Ok(Token::BangEq)));
    assert_eq!(lex.next(), Some(Ok(Token::Lt)));
    assert_eq!(lex.next(), Some(Ok(Token::LtEq)));
    assert_eq!(lex.next(), Some(Ok(Token::Gt)));
    assert_eq!(lex.next(), Some(Ok(Token::GtEq)));
    assert_eq!(lex.next(), Some(Ok(Token::PipeArrow)));
    assert_eq!(lex.next(), std::option::Option::None);
}

#[test]
fn delimiters() {
    let mut lex = Token::lexer(": ; , ( ) [ ] { } . .. -> => | ?");
    assert_eq!(lex.next(), Some(Ok(Token::Colon)));
    assert_eq!(lex.next(), Some(Ok(Token::Semi)));
    assert_eq!(lex.next(), Some(Ok(Token::Comma)));
    assert_eq!(lex.next(), Some(Ok(Token::LParen)));
    assert_eq!(lex.next(), Some(Ok(Token::RParen)));
    assert_eq!(lex.next(), Some(Ok(Token::LBracket)));
    assert_eq!(lex.next(), Some(Ok(Token::RBracket)));
    assert_eq!(lex.next(), Some(Ok(Token::LBrace)));
    assert_eq!(lex.next(), Some(Ok(Token::RBrace)));
    assert_eq!(lex.next(), Some(Ok(Token::Dot)));
    assert_eq!(lex.next(), Some(Ok(Token::DotDot)));
    assert_eq!(lex.next(), Some(Ok(Token::Arrow)));
    assert_eq!(lex.next(), Some(Ok(Token::FatArrow)));
    assert_eq!(lex.next(), Some(Ok(Token::Pipe)));
    assert_eq!(lex.next(), Some(Ok(Token::Question)));
    assert_eq!(lex.next(), std::option::Option::None);
}

#[test]
fn pipe_arrow_not_pipe_then_gt() {
    let tokens = tokenize("|>");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, Token::PipeArrow);
}

#[test]
fn identifiers() {
    let mut lex = Token::lexer("foo bar_baz _priv x123");
    assert_eq!(lex.next(), Some(Ok(Token::Ident("foo".into()))));
    assert_eq!(lex.next(), Some(Ok(Token::Ident("bar_baz".into()))));
    assert_eq!(lex.next(), Some(Ok(Token::Ident("_priv".into()))));
    assert_eq!(lex.next(), Some(Ok(Token::Ident("x123".into()))));
    assert_eq!(lex.next(), std::option::Option::None);
}
