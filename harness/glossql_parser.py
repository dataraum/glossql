"""Disposable §9.1 harness: statement-level parser for the simplified glossql.

Follows grammar.ebnf (2026-08-03). Statement forms are enforced strictly; JSON
payloads (aspect schemas, gloss bodies, ACCEPTS/RETURNS) are captured as single
tokens and validated with json.loads. Substrate SQL (recipes, views, SELECT
bodies, DELETE) is consumed opaquely — except GLOSSARY(...) and ATTEST(...)
calls, whose argument shapes are validated wherever they appear.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass

TOKEN_RE = re.compile(
    r"""(?P<ws>\s+)
      | (?P<comment>--[^\n]*)
      | (?P<str>'(?:[^']|'')*')
      | (?P<dqident>"(?:[^"]|"")*")
      | (?P<num>\d+(?:\.\d+)?)
      | (?P<ident>[A-Za-z_][A-Za-z0-9_$]*)
      | (?P<relop><->|->)
      | (?P<fatarrow>=>)
      | (?P<punct>.)
    """,
    re.VERBOSE | re.DOTALL,
)

ASPECT_KINDS = {"MEASUREMENT", "FACT", "QUERY"}
DECL_CLASSES = {"SOURCE", "RECIPE", "DATASET", "RELATIONSHIP", "ASPECT",
                "FUNCTION", "WITNESS"}


@dataclass
class Tok:
    kind: str  # str | num | ident | relop | fatarrow | json | punct
    text: str

    def up(self) -> str:
        return self.text.upper() if self.kind == "ident" else self.text


class ParseError(Exception):
    pass


def _capture_json(src: str, start: int) -> int:
    """src[start] == '{'. Return end index of the balanced JSON object,
    respecting double-quoted strings with backslash escapes."""
    depth, i, in_str = 0, start, False
    while i < len(src):
        c = src[i]
        if in_str:
            if c == "\\":
                i += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    raise ParseError("unbalanced { in JSON payload")


def tokenize(src: str) -> list[Tok]:
    toks: list[Tok] = []
    pos = 0
    while pos < len(src):
        m = TOKEN_RE.match(src, pos)
        kind = m.lastgroup
        if kind == "punct" and m.group() == "{":
            end = _capture_json(src, pos)
            toks.append(Tok("json", src[pos:end]))
            pos = end
            continue
        pos = m.end()
        if kind in ("ws", "comment"):
            continue
        if kind == "dqident":  # quoted identifier — never a keyword
            kind = "ident"
        toks.append(Tok(kind, m.group()))
    return toks


def split_statements(toks: list[Tok]) -> list[list[Tok]]:
    stmts, cur, depth = [], [], 0
    for t in toks:
        if t.kind == "punct" and t.text == "(":
            depth += 1
        elif t.kind == "punct" and t.text == ")":
            depth -= 1
        if t.kind == "punct" and t.text == ";" and depth == 0:
            if cur:
                stmts.append(cur)
            cur = []
        else:
            cur.append(t)
    if cur:
        stmts.append(cur)
    return stmts


class P:
    def __init__(self, toks: list[Tok]):
        self.toks = toks
        self.i = 0

    # -- primitives -------------------------------------------------------
    def peek(self, ahead: int = 0) -> Tok | None:
        j = self.i + ahead
        return self.toks[j] if j < len(self.toks) else None

    def at_kw(self, kw: str, ahead: int = 0) -> bool:
        t = self.peek(ahead)
        return t is not None and t.kind == "ident" and t.up() == kw

    def at_punct(self, ch: str, ahead: int = 0) -> bool:
        t = self.peek(ahead)
        return t is not None and t.kind == "punct" and t.text == ch

    def take(self) -> Tok:
        t = self.peek()
        if t is None:
            raise ParseError("unexpected end of statement")
        self.i += 1
        return t

    def expect_kw(self, kw: str) -> None:
        t = self.take()
        if not (t.kind == "ident" and t.up() == kw):
            raise ParseError(f"expected {kw}, got {t.text!r}")

    def expect_punct(self, ch: str) -> None:
        t = self.take()
        if not (t.kind == "punct" and t.text == ch):
            raise ParseError(f"expected {ch!r}, got {t.text!r}")

    def ident(self, what: str = "identifier") -> str:
        t = self.take()
        if t.kind != "ident":
            raise ParseError(f"expected {what}, got {t.text!r}")
        return t.text

    def string(self, what: str = "string literal") -> str:
        t = self.take()
        if t.kind != "str":
            raise ParseError(f"expected {what}, got {t.text!r}")
        return t.text

    def number(self) -> str:
        t = self.take()
        if t.kind != "num":
            raise ParseError(f"expected number, got {t.text!r}")
        return t.text

    def json_payload(self, what: str = "JSON payload") -> None:
        t = self.take()
        if t.kind != "json":
            raise ParseError(f"expected {what} {{...}}, got {t.text!r}")
        try:
            json.loads(t.text)
        except json.JSONDecodeError as e:
            raise ParseError(f"invalid JSON in {what}: {e}") from None

    def done(self) -> bool:
        return self.i >= len(self.toks)

    def end(self) -> None:
        if not self.done():
            raise ParseError(f"trailing tokens: {self.peek().text!r}")

    # -- shared shapes ----------------------------------------------------
    def dotted(self, what: str, most: int = 3) -> int:
        """name{.name} up to `most` segments; returns segment count."""
        self.ident(what)
        n = 1
        while n < most and self.at_punct("."):
            self.take()
            self.ident(f"{what} segment")
            n += 1
        return n

    def column_path(self) -> None:
        n = self.dotted("column path", most=3)
        if n < 2:
            raise ParseError("column path needs at least table.column")

    def subject(self) -> None:
        n = self.dotted("subject", most=3)
        if self.peek() and self.peek().kind == "relop":
            if n < 2:
                raise ParseError("pair path needs table.column on the left")
            self.take()
            self.column_path()

    def pairs(self) -> None:
        self.expect_punct("(")
        while True:
            self.ident("key")
            self.expect_punct(":")
            t = self.take()
            if t.kind not in ("ident", "str", "num"):
                raise ParseError(f"expected value after ':', got {t.text!r}")
            if self.at_punct(")"):
                self.take()
                return
            self.expect_punct(",")

    def named_arg(self) -> None:
        self.ident("argument name")
        t = self.take()
        if t.kind != "fatarrow":
            raise ParseError(f"expected =>, got {t.text!r}")
        t = self.take()
        if t.kind not in ("ident", "str", "num"):
            raise ParseError(f"expected argument value, got {t.text!r}")

    def opaque_to_end(self, what: str) -> None:
        if self.done():
            raise ParseError(f"expected {what}")
        while not self.done():
            self.take()

    # -- declarations -----------------------------------------------------
    def declaration(self) -> None:
        t = self.take()
        cls = t.up() if t.kind == "ident" else None
        if cls not in DECL_CLASSES:
            raise ParseError(f"unknown declaration class {t.text!r}")
        getattr(self, f"decl_{cls.lower()}")()

    def decl_source(self) -> None:
        self.ident("source name")
        self.expect_kw("SET")
        self.pairs()
        self.end()

    def decl_recipe(self) -> None:
        self.ident("recipe name")
        self.expect_kw("ON")
        self.ident("dataset name")
        self.expect_kw("FROM")
        self.ident("source name")
        self.expect_kw("AS")
        self.opaque_to_end("recipe SQL body")

    def decl_dataset(self) -> None:
        self.ident("dataset name")
        self.expect_kw("SET")
        self.pairs()
        self.end()

    def decl_relationship(self) -> None:
        self.column_path()
        t = self.take()
        if t.kind != "relop":
            raise ParseError(f"expected -> or <->, got {t.text!r}")
        self.column_path()
        self.end()

    def decl_aspect(self) -> None:
        self.ident("aspect name")
        self.expect_kw("WITH")
        self.json_payload("aspect schema")
        self.expect_kw("AS")
        t = self.take()
        if not (t.kind == "ident" and t.up() in ASPECT_KINDS):
            raise ParseError(f"expected MEASUREMENT | FACT | QUERY, got {t.text!r}")
        self.end()

    def decl_function(self) -> None:
        self.ident("function name")
        self.expect_kw("FOR")
        self.ident("dataset name or GLOBAL")
        self.expect_kw("FROM")
        self.string("script path")
        if self.at_kw("ACCEPTS"):
            self.take()
            if self.peek() and self.peek().kind == "json":
                self.json_payload("ACCEPTS schema")
            else:  # schema pointer: name#/segment/segment (placeholder syntax)
                self.ident("schema name")
                self.expect_punct("#")
                if not self.at_punct("/"):
                    raise ParseError("expected JSON pointer after '#'")
                while self.at_punct("/"):
                    self.take()
                    t = self.take()
                    if t.kind not in ("ident", "num"):
                        raise ParseError(f"expected pointer segment, got {t.text!r}")
        self.expect_kw("RETURNS")
        self.json_payload("RETURNS schema")
        self.end()

    def decl_witness(self) -> None:
        self.ident("witness name")
        self.expect_kw("ON")
        self.ident("aspect name")
        self.expect_kw("BY")
        self.expect_punct("(")
        while True:
            t = self.take()
            speaker = t.up() if t.kind == "ident" else None
            if speaker == "FUNCTION":
                self.ident("function name")
            elif speaker not in ("AGENT", "HUMAN"):
                raise ParseError(f"expected FUNCTION <fn> | AGENT | HUMAN, got {t.text!r}")
            if self.at_punct(")"):
                self.take()
                break
            self.expect_punct(",")
        if self.at_kw("DETECTOR"):
            self.take()
            self.ident("detector function name")
        if self.at_kw("THRESHOLD"):
            self.take()
            self.number()
        self.end()

    # -- statements -------------------------------------------------------
    def statement(self) -> None:
        t = self.peek()
        if t is None:
            return
        head = t.up() if t.kind == "ident" else None
        if head == "DECLARE":
            self.take()
            self.declaration()
        elif head == "USE":
            self.take()
            self.ident("dataset name")
            self.end()
        elif head == "GLOSS":
            self.take()
            self.gloss()
        else:
            self.substrate()  # SELECT, CREATE VIEW, DELETE, ... (host SQL)

    def gloss(self) -> None:
        self.ident("aspect name")
        self.expect_kw("ON")
        self.subject()
        self.expect_kw("AS")
        self.json_payload("gloss body")
        self.end()

    def substrate(self) -> None:
        """Opaque host SQL, but GLOSSARY(...) / ATTEST(...) calls are strict."""
        if self.done():
            raise ParseError("empty statement")
        while not self.done():
            if self.at_kw("GLOSSARY") and self.at_punct("(", 1):
                self.take()
                self.take()
                self.subject()
                while self.at_punct(","):
                    self.take()
                    self.named_arg()
                self.expect_punct(")")
            elif self.at_kw("ATTEST") and self.at_punct("(", 1):
                self.take()
                self.take()
                self.attest_arg()
                self.expect_punct(")")
            else:
                self.take()

    def attest_arg(self) -> None:
        """subject.aspect — the aspect is the final dotted segment."""
        n = self.dotted("attest subject", most=4)
        if self.peek() and self.peek().kind == "relop":
            self.take()
            # right side of the pair path plus the trailing aspect segment
            if self.dotted("attest pair right", most=4) < 3:
                raise ParseError("ATTEST on a pair path needs table.column.aspect "
                                 "on the right")
        elif n < 2:
            raise ParseError("ATTEST argument needs subject.aspect")


def parse_statement(toks: list[Tok]) -> None:
    p = P(toks)
    p.statement()
    p.end()


def check_source(src: str) -> list[tuple[str, str | None]]:
    """Parse every statement in src. Returns [(statement_preview, error|None)]."""
    results = []
    try:
        stmts = split_statements(tokenize(src))
    except ParseError as e:
        return [(src.strip()[:60], str(e))]
    for stmt in stmts:
        preview = " ".join(t.text for t in stmt[:8])
        try:
            parse_statement(stmt)
            results.append((preview, None))
        except ParseError as e:
            results.append((preview, str(e)))
    return results
