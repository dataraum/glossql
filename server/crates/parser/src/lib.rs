//! Statement-level parser for glossql.
//!
//! Rust port of the §9.1 harness parser (`harness/glossql_parser.py`), which
//! follows `grammar.ebnf`. Statement forms are enforced strictly; JSON payloads
//! (aspect schemas, gloss bodies, ACCEPTS/RETURNS) are captured as single
//! tokens and validated with serde_json. Substrate SQL (recipes, views, SELECT
//! bodies, DELETE) is consumed opaquely — except `GLOSSARY(...)` and
//! `ATTEST(...)` calls, whose argument shapes are validated wherever they
//! appear. The corpus is the acceptance suite (`tests/corpus.rs` mirrors
//! `harness/check.py`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Str,
    Num,
    Ident,
    Relop,
    Fatarrow,
    Json,
    Punct,
}

#[derive(Debug, Clone)]
pub struct Tok {
    pub kind: Kind,
    pub text: String,
}

impl Tok {
    fn is_kw(&self, kw: &str) -> bool {
        self.kind == Kind::Ident && self.text.eq_ignore_ascii_case(kw)
    }

    fn is_punct(&self, ch: char) -> bool {
        self.kind == Kind::Punct && self.text.chars().next() == Some(ch) && self.text.chars().count() == 1
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ParseError(pub String);

fn err(msg: impl Into<String>) -> ParseError {
    ParseError(msg.into())
}

const ASPECT_KINDS: [&str; 3] = ["MEASUREMENT", "FACT", "QUERY"];

/// `src[start] == '{'`. Returns the end byte index of the balanced JSON
/// object, respecting double-quoted strings with backslash escapes.
fn capture_json(src: &str, start: usize) -> Result<usize, ParseError> {
    let b = src.as_bytes();
    let (mut depth, mut i, mut in_str) = (0i32, start, false);
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'\\' {
                i += 1;
            } else if c == b'"' {
                in_str = false;
            }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                return Ok(i + 1);
            }
        }
        i += 1;
    }
    Err(err("unbalanced { in JSON payload"))
}

pub fn tokenize(src: &str) -> Result<Vec<Tok>, ParseError> {
    let b = src.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'-' && b.get(i + 1) == Some(&b'-') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if c == b'\'' {
            let start = i;
            i += 1;
            loop {
                match b.get(i) {
                    None => return Err(err("unterminated string literal")),
                    Some(b'\'') if b.get(i + 1) == Some(&b'\'') => i += 2,
                    Some(b'\'') => {
                        i += 1;
                        break;
                    }
                    Some(_) => i += 1,
                }
            }
            toks.push(Tok { kind: Kind::Str, text: src[start..i].to_string() });
        } else if c == b'"' {
            // double-quoted identifier — never a keyword (quotes stay in the text)
            let start = i;
            i += 1;
            loop {
                match b.get(i) {
                    None => return Err(err("unterminated quoted identifier")),
                    Some(b'"') if b.get(i + 1) == Some(&b'"') => i += 2,
                    Some(b'"') => {
                        i += 1;
                        break;
                    }
                    Some(_) => i += 1,
                }
            }
            toks.push(Tok { kind: Kind::Ident, text: src[start..i].to_string() });
        } else if c.is_ascii_digit() {
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if b.get(i) == Some(&b'.') && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            toks.push(Tok { kind: Kind::Num, text: src[start..i].to_string() });
        } else if c == b'_' || c.is_ascii_alphabetic() {
            let start = i;
            while i < b.len() && (b[i] == b'_' || b[i] == b'$' || b[i].is_ascii_alphanumeric()) {
                i += 1;
            }
            toks.push(Tok { kind: Kind::Ident, text: src[start..i].to_string() });
        } else if src[i..].starts_with("<->") {
            toks.push(Tok { kind: Kind::Relop, text: "<->".to_string() });
            i += 3;
        } else if src[i..].starts_with("->") {
            toks.push(Tok { kind: Kind::Relop, text: "->".to_string() });
            i += 2;
        } else if src[i..].starts_with("=>") {
            toks.push(Tok { kind: Kind::Fatarrow, text: "=>".to_string() });
            i += 2;
        } else if c == b'{' {
            let end = capture_json(src, i)?;
            toks.push(Tok { kind: Kind::Json, text: src[i..end].to_string() });
            i = end;
        } else {
            let ch = src[i..].chars().next().expect("in-bounds char");
            toks.push(Tok { kind: Kind::Punct, text: ch.to_string() });
            i += ch.len_utf8();
        }
    }
    Ok(toks)
}

/// Split on top-level `;` (parenthesis depth 0). JSON payloads and string
/// literals are single tokens, so their semicolons never split.
pub fn split_statements(toks: Vec<Tok>) -> Vec<Vec<Tok>> {
    let mut stmts = Vec::new();
    let mut cur = Vec::new();
    let mut depth = 0i32;
    for t in toks {
        if t.is_punct('(') {
            depth += 1;
        } else if t.is_punct(')') {
            depth -= 1;
        }
        if t.is_punct(';') && depth == 0 {
            if !cur.is_empty() {
                stmts.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(t);
        }
    }
    if !cur.is_empty() {
        stmts.push(cur);
    }
    stmts
}

struct P<'a> {
    toks: &'a [Tok],
    i: usize,
}

impl<'a> P<'a> {
    fn new(toks: &'a [Tok]) -> Self {
        P { toks, i: 0 }
    }

    // -- primitives -------------------------------------------------------

    fn peek(&self, ahead: usize) -> Option<&Tok> {
        self.toks.get(self.i + ahead)
    }

    fn at_kw(&self, kw: &str, ahead: usize) -> bool {
        self.peek(ahead).is_some_and(|t| t.is_kw(kw))
    }

    fn at_punct(&self, ch: char, ahead: usize) -> bool {
        self.peek(ahead).is_some_and(|t| t.is_punct(ch))
    }

    fn take(&mut self) -> Result<Tok, ParseError> {
        let t = self.peek(0).cloned().ok_or_else(|| err("unexpected end of statement"))?;
        self.i += 1;
        Ok(t)
    }

    fn expect_kw(&mut self, kw: &str) -> Result<(), ParseError> {
        let t = self.take()?;
        if t.is_kw(kw) { Ok(()) } else { Err(err(format!("expected {kw}, got {:?}", t.text))) }
    }

    fn expect_punct(&mut self, ch: char) -> Result<(), ParseError> {
        let t = self.take()?;
        if t.is_punct(ch) { Ok(()) } else { Err(err(format!("expected {ch:?}, got {:?}", t.text))) }
    }

    fn ident(&mut self, what: &str) -> Result<String, ParseError> {
        let t = self.take()?;
        if t.kind == Kind::Ident { Ok(t.text) } else { Err(err(format!("expected {what}, got {:?}", t.text))) }
    }

    fn string(&mut self, what: &str) -> Result<(), ParseError> {
        let t = self.take()?;
        if t.kind == Kind::Str { Ok(()) } else { Err(err(format!("expected {what}, got {:?}", t.text))) }
    }

    fn number(&mut self) -> Result<(), ParseError> {
        let t = self.take()?;
        if t.kind == Kind::Num { Ok(()) } else { Err(err(format!("expected number, got {:?}", t.text))) }
    }

    fn json_payload(&mut self, what: &str) -> Result<(), ParseError> {
        let t = self.take()?;
        if t.kind != Kind::Json {
            return Err(err(format!("expected {what} {{...}}, got {:?}", t.text)));
        }
        serde_json::from_str::<serde_json::Value>(&t.text)
            .map(|_| ())
            .map_err(|e| err(format!("invalid JSON in {what}: {e}")))
    }

    fn done(&self) -> bool {
        self.i >= self.toks.len()
    }

    fn end(&self) -> Result<(), ParseError> {
        match self.peek(0) {
            None => Ok(()),
            Some(t) => Err(err(format!("trailing tokens: {:?}", t.text))),
        }
    }

    // -- shared shapes ----------------------------------------------------

    /// `name{.name}` up to `most` segments; returns the segment count.
    fn dotted(&mut self, what: &str, most: usize) -> Result<usize, ParseError> {
        self.ident(what)?;
        let mut n = 1;
        while n < most && self.at_punct('.', 0) {
            self.take()?;
            self.ident(&format!("{what} segment"))?;
            n += 1;
        }
        Ok(n)
    }

    fn column_path(&mut self) -> Result<(), ParseError> {
        if self.dotted("column path", 3)? < 2 {
            return Err(err("column path needs at least table.column"));
        }
        Ok(())
    }

    fn subject(&mut self) -> Result<(), ParseError> {
        let n = self.dotted("subject", 3)?;
        if self.peek(0).is_some_and(|t| t.kind == Kind::Relop) {
            if n < 2 {
                return Err(err("pair path needs table.column on the left"));
            }
            self.take()?;
            self.column_path()?;
        }
        Ok(())
    }

    fn pairs(&mut self) -> Result<(), ParseError> {
        self.expect_punct('(')?;
        loop {
            self.ident("key")?;
            self.expect_punct(':')?;
            let t = self.take()?;
            if !matches!(t.kind, Kind::Ident | Kind::Str | Kind::Num) {
                return Err(err(format!("expected value after ':', got {:?}", t.text)));
            }
            if self.at_punct(')', 0) {
                self.take()?;
                return Ok(());
            }
            self.expect_punct(',')?;
        }
    }

    fn named_arg(&mut self) -> Result<(), ParseError> {
        self.ident("argument name")?;
        let t = self.take()?;
        if t.kind != Kind::Fatarrow {
            return Err(err(format!("expected =>, got {:?}", t.text)));
        }
        let t = self.take()?;
        if !matches!(t.kind, Kind::Ident | Kind::Str | Kind::Num) {
            return Err(err(format!("expected argument value, got {:?}", t.text)));
        }
        Ok(())
    }

    fn opaque_to_end(&mut self, what: &str) -> Result<(), ParseError> {
        if self.done() {
            return Err(err(format!("expected {what}")));
        }
        self.i = self.toks.len();
        Ok(())
    }

    // -- declarations -----------------------------------------------------

    fn declaration(&mut self) -> Result<(), ParseError> {
        let t = self.take()?;
        if t.is_kw("SOURCE") {
            self.decl_source()
        } else if t.is_kw("RECIPE") {
            self.decl_recipe()
        } else if t.is_kw("DATASET") {
            self.decl_dataset()
        } else if t.is_kw("RELATIONSHIP") {
            self.decl_relationship()
        } else if t.is_kw("ASPECT") {
            self.decl_aspect()
        } else if t.is_kw("FUNCTION") {
            self.decl_function()
        } else if t.is_kw("WITNESS") {
            self.decl_witness()
        } else {
            Err(err(format!("unknown declaration class {:?}", t.text)))
        }
    }

    fn decl_source(&mut self) -> Result<(), ParseError> {
        self.ident("source name")?;
        self.expect_kw("SET")?;
        self.pairs()?;
        self.end()
    }

    fn decl_recipe(&mut self) -> Result<(), ParseError> {
        self.ident("recipe name")?;
        self.expect_kw("ON")?;
        self.ident("dataset name")?;
        self.expect_kw("FROM")?;
        self.ident("source name")?;
        self.expect_kw("AS")?;
        self.opaque_to_end("recipe SQL body")
    }

    fn decl_dataset(&mut self) -> Result<(), ParseError> {
        self.ident("dataset name")?;
        self.expect_kw("SET")?;
        self.pairs()?;
        self.end()
    }

    fn decl_relationship(&mut self) -> Result<(), ParseError> {
        self.column_path()?;
        let t = self.take()?;
        if t.kind != Kind::Relop {
            return Err(err(format!("expected -> or <->, got {:?}", t.text)));
        }
        self.column_path()?;
        self.end()
    }

    fn decl_aspect(&mut self) -> Result<(), ParseError> {
        self.ident("aspect name")?;
        self.expect_kw("WITH")?;
        self.json_payload("aspect schema")?;
        self.expect_kw("AS")?;
        let t = self.take()?;
        if !ASPECT_KINDS.iter().any(|k| t.is_kw(k)) {
            return Err(err(format!("expected MEASUREMENT | FACT | QUERY, got {:?}", t.text)));
        }
        self.end()
    }

    fn decl_function(&mut self) -> Result<(), ParseError> {
        self.ident("function name")?;
        self.expect_kw("FOR")?;
        self.ident("dataset name or GLOBAL")?;
        self.expect_kw("FROM")?;
        self.string("script path")?;
        if self.at_kw("ACCEPTS", 0) {
            self.take()?;
            if self.peek(0).is_some_and(|t| t.kind == Kind::Json) {
                self.json_payload("ACCEPTS schema")?;
            } else {
                // schema pointer: name#/segment/segment
                self.ident("schema name")?;
                self.expect_punct('#')?;
                if !self.at_punct('/', 0) {
                    return Err(err("expected JSON pointer after '#'"));
                }
                while self.at_punct('/', 0) {
                    self.take()?;
                    let t = self.take()?;
                    if !matches!(t.kind, Kind::Ident | Kind::Num) {
                        return Err(err(format!("expected pointer segment, got {:?}", t.text)));
                    }
                }
            }
        }
        self.expect_kw("RETURNS")?;
        self.json_payload("RETURNS schema")?;
        self.end()
    }

    fn decl_witness(&mut self) -> Result<(), ParseError> {
        self.ident("witness name")?;
        self.expect_kw("ON")?;
        self.ident("aspect name")?;
        self.expect_kw("BY")?;
        self.expect_punct('(')?;
        loop {
            let t = self.take()?;
            if t.is_kw("FUNCTION") {
                self.ident("function name")?;
            } else if !t.is_kw("AGENT") && !t.is_kw("HUMAN") {
                return Err(err(format!("expected FUNCTION <fn> | AGENT | HUMAN, got {:?}", t.text)));
            }
            if self.at_punct(')', 0) {
                self.take()?;
                break;
            }
            self.expect_punct(',')?;
        }
        if self.at_kw("DETECTOR", 0) {
            self.take()?;
            self.ident("detector function name")?;
        }
        if self.at_kw("THRESHOLD", 0) {
            self.take()?;
            self.number()?;
        }
        self.end()
    }

    // -- statements -------------------------------------------------------

    fn statement(&mut self) -> Result<(), ParseError> {
        let Some(t) = self.peek(0) else { return Ok(()) };
        if t.is_kw("DECLARE") {
            self.take()?;
            self.declaration()
        } else if t.is_kw("USE") {
            self.take()?;
            self.ident("dataset name")?;
            self.end()
        } else if t.is_kw("GLOSS") {
            self.take()?;
            self.gloss()
        } else {
            self.substrate() // SELECT, CREATE VIEW, DELETE, ... (host SQL)
        }
    }

    fn gloss(&mut self) -> Result<(), ParseError> {
        self.ident("aspect name")?;
        self.expect_kw("ON")?;
        self.subject()?;
        self.expect_kw("AS")?;
        self.json_payload("gloss body")?;
        self.end()
    }

    /// Opaque host SQL, but `GLOSSARY(...)` / `ATTEST(...)` calls are strict.
    fn substrate(&mut self) -> Result<(), ParseError> {
        if self.done() {
            return Err(err("empty statement"));
        }
        while !self.done() {
            if self.at_kw("GLOSSARY", 0) && self.at_punct('(', 1) {
                self.take()?;
                self.take()?;
                self.subject()?;
                while self.at_punct(',', 0) {
                    self.take()?;
                    self.named_arg()?;
                }
                self.expect_punct(')')?;
            } else if self.at_kw("ATTEST", 0) && self.at_punct('(', 1) {
                self.take()?;
                self.take()?;
                self.attest_arg()?;
                self.expect_punct(')')?;
            } else {
                self.take()?;
            }
        }
        Ok(())
    }

    /// `subject.aspect` — the aspect is the final dotted segment.
    fn attest_arg(&mut self) -> Result<(), ParseError> {
        let n = self.dotted("attest subject", 4)?;
        if self.peek(0).is_some_and(|t| t.kind == Kind::Relop) {
            self.take()?;
            // right side of the pair path plus the trailing aspect segment
            if self.dotted("attest pair right", 4)? < 3 {
                return Err(err("ATTEST on a pair path needs table.column.aspect on the right"));
            }
        } else if n < 2 {
            return Err(err("ATTEST argument needs subject.aspect"));
        }
        Ok(())
    }
}

pub fn parse_statement(toks: &[Tok]) -> Result<(), ParseError> {
    let mut p = P::new(toks);
    p.statement()?;
    p.end()
}

/// Parse every statement in `src`. Returns `(statement_preview, error)` pairs.
pub fn check_source(src: &str) -> Vec<(String, Option<String>)> {
    let stmts = match tokenize(src) {
        Ok(toks) => split_statements(toks),
        Err(e) => {
            let preview: String = src.trim().chars().take(60).collect();
            return vec![(preview, Some(e.to_string()))];
        }
    };
    stmts
        .iter()
        .map(|stmt| {
            let preview = stmt.iter().take(8).map(|t| t.text.as_str()).collect::<Vec<_>>().join(" ");
            (preview, parse_statement(stmt).err().map(|e| e.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(src: &str) {
        for (preview, e) in check_source(src) {
            assert!(e.is_none(), "{preview}: {e:?}");
        }
    }

    fn fails(src: &str) {
        assert!(check_source(src).iter().any(|(_, e)| e.is_some()), "expected a failure in {src:?}");
    }

    #[test]
    fn heads_parse() {
        ok("USE fin;");
        ok("DECLARE SOURCE s SET (type: parquet, location: 'lake/*.parquet');");
        ok("DECLARE RELATIONSHIP orders.customer_id -> customers.id;");
        ok("DECLARE ASPECT meaning WITH {\"type\": \"object\"} AS FACT;");
        ok("GLOSS meaning ON orders.amount AS {\"value\": \"gross amount\"};");
        ok("DECLARE WITNESS w ON validity BY (FUNCTION check, AGENT, HUMAN) DETECTOR d THRESHOLD 0.5;");
        ok("SELECT * FROM GLOSSARY(fin.orders.amount);");
        ok("SELECT band FROM ATTEST(orders.customer_id -> customers.id.verified);");
    }

    #[test]
    fn strictness_holds() {
        fails("DECLARE PATTERN 'x' FOR TYPE;"); // no such head — fixture 13's rejected fork
        fails("DECLARE ASPECT a WITH {\"broken\": } AS FACT;"); // invalid JSON
        fails("GLOSS meaning ON a.b.c.d AS {\"v\": 1};"); // subject too deep
        fails("SELECT * FROM GLOSSARY(fin,);"); // malformed read args
    }

    #[test]
    fn lexer_edges() {
        // comment wins over relop; json semicolons don't split; '' escapes hold
        ok("SELECT 1 -- a -> comment\n;");
        assert_eq!(split_statements(tokenize("GLOSS a ON b AS {\"x\": \";\"};").unwrap()).len(), 1);
        ok("DECLARE SOURCE s SET (path: 'it''s');");
    }
}
