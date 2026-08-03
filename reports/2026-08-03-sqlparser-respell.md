# 2026-08-03 — respell for the stock sqlparser tokenizer; REFRESH dropped

Decision record for building the server parser **within** the DataFusion
parser machinery instead of beside it. Supersedes the "statement spelling vs
sqlparser" section of `2026-08-03-poc-substrate.md` where the two disagree.
All claims verified against the crates the server pins: datafusion 54.1.0 →
sqlparser 0.62.0 (probe project in session scratch; registry sources, paths
below relative to the crate roots).

## The constraint

`DFParser` tokenizes the **entire input up front** with sqlparser's tokenizer
and parses over tokens (`datafusion-sql/src/parser.rs:424-427`; dispatch
`:547`; the inner `Parser` is public, `:326`). Anything the tokenizer mangles
is unrecoverable — "parse within DataFusion" therefore means: every byte of
glossql must lex with stock sqlparser tokens. Measured against that bar
(tokenizer probe, generic × postgres × duckdb dialects):

| construct | verdict |
|---|---|
| bare-brace JSON bodies | **destroyed** in all dialects — `\"` inside JSON strings ends the double-quoted region (`tokenizer.rs:2064` knows only quote-doubling); even escape-free bodies lose their quoting token-by-token |
| `ACCEPTS producer#/ptr` | dialect-unstable — generic folds `#` into the word, postgres lexes `CustomBinaryOperator("#/")`, duckdb `Sharp Div` |
| recipe tails (`AS <sql>`) | foreign-dialect SQL; one alien lexeme fails the whole script's upfront tokenization |
| `DECLARE`/`GLOSS`/`USE` heads | fine — hand-parse on the shared token stream; unknown words are `Keyword::NoKeyword` (`tokenizer.rs:437`) |
| `->`, `=>`, `<->` | `Token::Arrow` / `Token::RArrow` everywhere; `<->` is one `TwoWayArrow` token on the **postgres dialect only** |
| reads: `USE`, both `GLOSSARY()` forms (incl. `all => true`), `ATTEST` sweeps, `->`/`<->` pair paths | parse stock (parser-level probe); `<->` requires the postgres dialect |

## The respell (approved by the project lead, 2026-08-03)

1. **JSON bodies** (`WITH` / `AS` / `ACCEPTS` / `RETURNS`) are dollar-quoted:
   `AS $${"value": "EUR"}$$`. One verbatim `DollarQuotedString` token in
   every relevant dialect (`tokenizer.rs:1862,1922`); the JSON document rides
   byte-exact — no escaping, ever; `$tag$ … $tag$` covers a body containing
   `$$`. Weighed against single-quoted string bodies (portable, but `''`
   doubling hits the *common* path — grounding bodies embed SQL with single
   quotes — and a missed doubling can misparse quietly, while a `$$`
   collision fails loudly). The 2026-08-03 dissolution rejection was about
   escaping; dollar quotes have none. One normative spelling: the parser
   admits only dollar bodies.
2. **Schema pointer** is a string: `ACCEPTS 'period_grain#/properties/days'`
   (`producer#/json/pointer` inside one token).
3. **Recipe tails** are dollar-quoted: `AS $$SELECT …$$`. One rule across the
   language: foreign text rides in dollar quotes; substrate SQL (DataFusion's
   own) stays bare.
4. **`REFRESH` is dropped** — same silent-misparse class that killed
   `SEQUENTIAL | PARALLEL` (a SELECT falling through to the substrate parser
   would swallow trailing `REFRESH` as a table alias). Re-running is
   removal: the cache is an ordinary relation like `glossary`; DELETE the
   cached rows and select again. No modifier, no ordering surface.

Corpus + SPEC re-spelled the same day (30 blocks, mechanical transform);
fixture semantics untouched.

## Architecture consequence

One parser, one token stream — the in-tree `custom_sql_parser.rs` shape:
`GlossqlParser` wraps `DFParser` (postgres dialect), peeks the head word,
hand-parses the glossql forms with `Parser` primitives (`maybe_parse`,
`parser/mod.rs:5057`, gives clean extract-probe rollback), and delegates
everything else to `DFParser::parse_statement` → `DFStatement`. Execution
never re-parses: substrate goes through
`SessionState::statement_to_plan` (`datafusion/src/execution/session_state.rs:526`);
`GLOSSARY()`/`ATTEST()` are UDTFs via `register_udtf`
(`context/mod.rs:1603`, trait `datafusion-catalog/src/table.rs:551`); pair
paths inside read arguments resolve through `ExprPlanner::plan_binary_op`
(`datafusion-expr/src/planner.rs:159`); the session dialect is a config
option (`datafusion-common/src/config.rs:283`), set to postgres.
`datafusion-functions-json 0.54.2` resolves against datafusion 54.1 (closes
the stack-eval w4 gap).

Layout: workspace at the repo root (`crates/…`, directories unprefixed,
package names `glossql-*`), crates per concern: parser · glossary · catalog ·
scripts · import · session · serverd. Crates are created when they have real
content; the corpus suite lives in `crates/parser` and is the standing
invariant.
