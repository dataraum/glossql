use datafusion_common::Result;
use datafusion_sql::parser::{DFParser, DFParserBuilder};
use datafusion_sql::sqlparser::ast::Ident;
use datafusion_sql::sqlparser::dialect::PostgreSqlDialect;
use datafusion_sql::sqlparser::keywords::Keyword;
use datafusion_sql::sqlparser::parser::{Parser, ParserError};
use datafusion_sql::sqlparser::tokenizer::{Token, TokenWithSpan};

use crate::ast::*;

// The postgres dialect is load-bearing: it lexes `<->` as one TwoWayArrow
// token and supports dollar-quoted bodies; the session uses the same
// dialect for planning (reports/2026-08-03-sqlparser-respell.md).
static DIALECT: PostgreSqlDialect = PostgreSqlDialect {};

/// The glossql front parser: DataFusion's `DFParser` custom-statement
/// pattern with glossql heads.
///
/// One token stream. The head word routes: `DECLARE`, `GLOSS`, and `USE`
/// are parsed here with `Parser` primitives; a `SELECT` is probed as an
/// extraction (`maybe_parse` restores the position on mismatch); everything
/// else delegates to `DFParser::parse_statement` and stays substrate.
pub struct GlossqlParser<'a> {
    df: DFParser<'a>,
}

impl<'a> GlossqlParser<'a> {
    pub fn new(sql: &'a str) -> Result<Self> {
        let df = DFParserBuilder::new(sql).with_dialect(&DIALECT).build()?;
        Ok(GlossqlParser { df })
    }

    /// Parse a script of `;`-terminated statements.
    pub fn parse_sql(sql: &str) -> Result<Vec<Statement>> {
        GlossqlParser::new(sql)?.parse_statements()
    }

    /// Mirrors `DFParser::parse_statements`: empty statements between
    /// delimiters are dropped, a missing delimiter between statements is an
    /// error.
    pub fn parse_statements(&mut self) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();
        let mut expecting_delimiter = false;
        loop {
            while self.df.parser.consume_token(&Token::SemiColon) {
                expecting_delimiter = false;
            }
            if self.df.parser.peek_token_ref().token == Token::EOF {
                break;
            }
            if expecting_delimiter {
                let found = self.df.parser.peek_token();
                return Ok(expected("end of statement (`;`)", &found)?);
            }
            statements.push(self.parse_statement()?);
            expecting_delimiter = true;
        }
        Ok(statements)
    }

    pub fn parse_statement(&mut self) -> Result<Statement> {
        let token = self.df.parser.peek_token();
        if let Token::Word(w) = &token.token
            && w.quote_style.is_none()
        {
            match w.keyword {
                Keyword::DECLARE => {
                    return Ok(Statement::Declare(Box::new(parse_declaration(
                        &mut self.df.parser,
                    )?)));
                }
                Keyword::USE => {
                    return Ok(Statement::Use(parse_use(&mut self.df.parser)?));
                }
                Keyword::SELECT => {
                    if let Some(extract) = self.df.parser.maybe_parse(parse_extract)? {
                        return Ok(Statement::Extract(extract));
                    }
                }
                _ if w.value.eq_ignore_ascii_case("GLOSS") => {
                    return Ok(Statement::Gloss(parse_gloss(&mut self.df.parser)?));
                }
                _ if w.value.eq_ignore_ascii_case("PROBE") => {
                    return Ok(Statement::Probe(parse_probe(&mut self.df.parser)?));
                }
                _ => {}
            }
        }
        Ok(Statement::Substrate(Box::new(self.df.parse_statement()?)))
    }
}

fn expected<T>(what: &str, found: &TokenWithSpan) -> Result<T, ParserError> {
    Err(ParserError::ParserError(format!(
        "Expected: {what}, found: {found}{}",
        found.span.start
    )))
}

fn peek_word_is(p: &Parser, word: &str) -> bool {
    matches!(&p.peek_token_ref().token,
        Token::Word(w) if w.quote_style.is_none() && w.value.eq_ignore_ascii_case(word))
}

fn consume_word(p: &mut Parser, word: &str) -> bool {
    if peek_word_is(p, word) {
        p.next_token();
        true
    } else {
        false
    }
}

fn expect_word(p: &mut Parser, word: &str) -> Result<(), ParserError> {
    if consume_word(p, word) {
        Ok(())
    } else {
        let found = p.peek_token();
        expected(&format!("`{word}`"), &found)
    }
}

fn parse_probe(p: &mut Parser) -> Result<Probe, ParserError> {
    expect_word(p, "PROBE")?;
    let source = p.parse_identifier()?;
    expect_word(p, "AS")?;
    let sql = parse_dollar(p, "dollar-quoted probe SQL — AS $$ SELECT … $$")?;
    Ok(Probe { source, sql })
}

fn parse_declaration(p: &mut Parser) -> Result<Declaration, ParserError> {
    p.expect_keyword(Keyword::DECLARE)?;
    let head = p.peek_token();
    let word = match &head.token {
        Token::Word(w) if w.quote_style.is_none() => w.value.to_ascii_uppercase(),
        _ => String::new(),
    };
    match word.as_str() {
        "SOURCE" => {
            p.next_token();
            let name = p.parse_identifier()?;
            expect_word(p, "SET")?;
            let settings = parse_settings(p)?;
            Ok(Declaration::Source(SourceDecl { name, settings }))
        }
        "RECIPE" => {
            p.next_token();
            let table = p.parse_identifier()?;
            expect_word(p, "ON")?;
            let dataset = p.parse_identifier()?;
            expect_word(p, "FROM")?;
            let source = p.parse_identifier()?;
            expect_word(p, "AS")?;
            let sql = parse_dollar(p, "dollar-quoted recipe SQL — AS $$ SELECT … $$")?;
            Ok(Declaration::Recipe(RecipeDecl {
                table,
                dataset,
                source,
                sql,
            }))
        }
        "DATASET" => {
            p.next_token();
            let name = p.parse_identifier()?;
            expect_word(p, "SET")?;
            let settings = parse_settings(p)?;
            Ok(Declaration::Dataset(DatasetDecl { name, settings }))
        }
        "RELATIONSHIP" => {
            p.next_token();
            let left = parse_column_path(p)?;
            let op = parse_rel_op(p)?;
            let right = parse_column_path(p)?;
            Ok(Declaration::Relationship(RelationshipDecl {
                left,
                op,
                right,
            }))
        }
        "ASPECT" => {
            p.next_token();
            let name = p.parse_identifier()?;
            expect_word(p, "WITH")?;
            let schema = parse_json_body(p)?;
            expect_word(p, "AS")?;
            let kind_token = p.peek_token();
            let kind = if consume_word(p, "MEASUREMENT") {
                AspectKind::Measurement
            } else if consume_word(p, "FACT") {
                AspectKind::Fact
            } else if consume_word(p, "QUERY") {
                AspectKind::Query
            } else {
                return expected("MEASUREMENT, FACT, or QUERY", &kind_token);
            };
            Ok(Declaration::Aspect(AspectDecl { name, schema, kind }))
        }
        "FUNCTION" => {
            p.next_token();
            let name = p.parse_identifier()?;
            expect_word(p, "FOR")?;
            let scope = if consume_word(p, "GLOBAL") {
                FunctionScope::Global
            } else {
                FunctionScope::Dataset(p.parse_identifier()?)
            };
            expect_word(p, "FROM")?;
            let script = parse_string(p, "the script reference as a string")?;
            let accepts = if consume_word(p, "ACCEPTS") {
                parse_accepts(p)?
            } else {
                Vec::new()
            };
            expect_word(p, "RETURNS")?;
            let returns = parse_json_body(p)?;
            Ok(Declaration::Function(FunctionDecl {
                name,
                scope,
                script,
                accepts,
                returns,
            }))
        }
        "WITNESS" => {
            p.next_token();
            let name = p.parse_identifier()?;
            expect_word(p, "ON")?;
            let aspect = p.parse_identifier()?;
            expect_word(p, "BY")?;
            p.expect_token(&Token::LParen)?;
            let mut speakers = Vec::new();
            loop {
                speakers.push(parse_speaker(p)?);
                if p.consume_token(&Token::RParen) {
                    break;
                }
                p.expect_token(&Token::Comma)?;
            }
            let detector = if consume_word(p, "DETECTOR") {
                Some(p.parse_identifier()?)
            } else {
                None
            };
            let threshold = if consume_word(p, "THRESHOLD") {
                Some(parse_number(p)?)
            } else {
                None
            };
            Ok(Declaration::Witness(WitnessDecl {
                name,
                aspect,
                speakers,
                detector,
                threshold,
            }))
        }
        _ => expected(
            "SOURCE, RECIPE, DATASET, RELATIONSHIP, ASPECT, FUNCTION, or WITNESS after DECLARE",
            &head,
        ),
    }
}

fn parse_settings(p: &mut Parser) -> Result<Vec<Setting>, ParserError> {
    p.expect_token(&Token::LParen)?;
    let mut settings = Vec::new();
    loop {
        let key = p.parse_identifier()?;
        p.expect_token(&Token::Colon)?;
        let value = match p.peek_token_ref().token {
            Token::SingleQuotedString(_) => match p.next_token().token {
                Token::SingleQuotedString(s) => SettingValue::String(s),
                _ => unreachable!("peeked"),
            },
            Token::Number(..) => match p.next_token().token {
                Token::Number(n, _) => SettingValue::Number(n),
                _ => unreachable!("peeked"),
            },
            _ => SettingValue::Name(p.parse_identifier()?),
        };
        settings.push(Setting { key, value });
        if p.consume_token(&Token::RParen) {
            break;
        }
        p.expect_token(&Token::Comma)?;
    }
    Ok(settings)
}

fn parse_speaker(p: &mut Parser) -> Result<Speaker, ParserError> {
    if consume_word(p, "FUNCTION") {
        return Ok(Speaker::Function(p.parse_identifier()?));
    }
    if consume_word(p, "AGENT") {
        return Ok(Speaker::Agent);
    }
    if consume_word(p, "HUMAN") {
        return Ok(Speaker::Human);
    }
    let found = p.peek_token();
    expected("FUNCTION <name>, AGENT, or HUMAN", &found)
}

/// `ACCEPTS (aspect, …)` — the aspects whose current values the server
/// hands the script as its context document. Settings are context, never
/// call arguments.
fn parse_accepts(p: &mut Parser) -> Result<Vec<Ident>, ParserError> {
    p.expect_token(&Token::LParen)?;
    let mut aspects = vec![p.parse_identifier()?];
    while !p.consume_token(&Token::RParen) {
        p.expect_token(&Token::Comma)?;
        aspects.push(p.parse_identifier()?);
    }
    Ok(aspects)
}

/// A dollar-quoted region holding a JSON object, validated per RFC 8259.
fn parse_json_body(p: &mut Parser) -> Result<JsonBody, ParserError> {
    let token = p.next_token();
    let at = token.span.start;
    match token.token {
        Token::DollarQuotedString(s) => {
            let value: serde_json::Value = serde_json::from_str(&s.value)
                .map_err(|e| ParserError::ParserError(format!("invalid JSON body{at}: {e}")))?;
            if !value.is_object() {
                return Err(ParserError::ParserError(format!(
                    "JSON body must be an object{at}"
                )));
            }
            Ok(JsonBody {
                value,
                raw: s.value,
            })
        }
        Token::LBrace => Err(ParserError::ParserError(format!(
            "JSON bodies are dollar-quoted — write $${{…}}$$; bare braces do not survive the SQL tokenizer{at}"
        ))),
        _ => expected("a dollar-quoted JSON body $${…}$$", &token),
    }
}

fn parse_dollar(p: &mut Parser, what: &str) -> Result<String, ParserError> {
    let token = p.next_token();
    match token.token {
        Token::DollarQuotedString(s) => Ok(s.value),
        _ => expected(what, &token),
    }
}

fn parse_string(p: &mut Parser, what: &str) -> Result<String, ParserError> {
    let token = p.next_token();
    match token.token {
        Token::SingleQuotedString(s) => Ok(s),
        _ => expected(what, &token),
    }
}

fn parse_number(p: &mut Parser) -> Result<String, ParserError> {
    let token = p.next_token();
    match token.token {
        Token::Number(n, _) => Ok(n),
        _ => expected("a number", &token),
    }
}

fn parse_use(p: &mut Parser) -> Result<Use, ParserError> {
    p.expect_keyword(Keyword::USE)?;
    let dataset = p.parse_identifier()?;
    Ok(Use { dataset })
}

fn parse_gloss(p: &mut Parser) -> Result<Gloss, ParserError> {
    expect_word(p, "GLOSS")?;
    let aspect = p.parse_identifier()?;
    expect_word(p, "ON")?;
    let subject = parse_subject(p)?;
    expect_word(p, "AS")?;
    let body = parse_json_body(p)?;
    Ok(Gloss {
        aspect,
        subject,
        body,
    })
}

/// The `extract` production, complete: `SELECT call, … FROM subject` ending
/// at the statement boundary. Any deviation is a `ParserError`, which the
/// caller's `maybe_parse` turns into fall-through to substrate.
fn parse_extract(p: &mut Parser) -> Result<Extract, ParserError> {
    p.expect_keyword(Keyword::SELECT)?;
    let mut calls = vec![parse_call(p)?];
    while p.consume_token(&Token::Comma) {
        calls.push(parse_call(p)?);
    }
    p.expect_keyword(Keyword::FROM)?;
    let subject = parse_subject(p)?;
    match p.peek_token_ref().token {
        Token::SemiColon | Token::EOF => Ok(Extract { calls, subject }),
        _ => {
            let found = p.peek_token();
            expected(
                "end of statement — an extraction is SELECT calls FROM subject",
                &found,
            )
        }
    }
}

/// A call is bare — `f()`. Settings reach scripts as context (`ACCEPTS`),
/// never as arguments; a call with arguments fails here and falls through
/// to substrate SQL, where planning rejects it loudly.
fn parse_call(p: &mut Parser) -> Result<Ident, ParserError> {
    let function = p.parse_identifier()?;
    p.expect_token(&Token::LParen)?;
    p.expect_token(&Token::RParen)?;
    Ok(function)
}

fn parse_rel_op(p: &mut Parser) -> Result<RelOp, ParserError> {
    if p.consume_token(&Token::TwoWayArrow) {
        return Ok(RelOp::OneToOne);
    }
    if p.consume_token(&Token::Arrow) {
        return Ok(RelOp::ManyToOne);
    }
    let found = p.peek_token();
    expected("`->` or `<->`", &found)
}

fn parse_path_segments(p: &mut Parser) -> Result<Vec<Ident>, ParserError> {
    let mut segments = vec![p.parse_identifier()?];
    while p.consume_token(&Token::Period) {
        segments.push(p.parse_identifier()?);
        if segments.len() > 3 {
            let found = p.peek_token();
            return expected(
                "a path of at most three segments (dataset.table.column)",
                &found,
            );
        }
    }
    Ok(segments)
}

fn into_column_path(mut segments: Vec<Ident>, p: &Parser) -> Result<ColumnPath, ParserError> {
    match segments.len() {
        2 | 3 => {
            let column = segments.pop().expect("length checked");
            let table = segments.pop().expect("length checked");
            Ok(ColumnPath {
                dataset: segments.pop(),
                table,
                column,
            })
        }
        _ => {
            let found = p.peek_token();
            expected(
                "a relationship endpoint of table.column or dataset.table.column",
                &found,
            )
        }
    }
}

fn parse_column_path(p: &mut Parser) -> Result<ColumnPath, ParserError> {
    let segments = parse_path_segments(p)?;
    into_column_path(segments, p)
}

fn parse_subject(p: &mut Parser) -> Result<Subject, ParserError> {
    let segments = parse_path_segments(p)?;
    let op = if p.consume_token(&Token::TwoWayArrow) {
        Some(RelOp::OneToOne)
    } else if p.consume_token(&Token::Arrow) {
        Some(RelOp::ManyToOne)
    } else {
        None
    };
    match op {
        None => Ok(Subject::Path(Path { segments })),
        Some(op) => {
            let left = into_column_path(segments, p)?;
            let right = parse_column_path(p)?;
            Ok(Subject::Pair(Box::new(PairPath { left, op, right })))
        }
    }
}
