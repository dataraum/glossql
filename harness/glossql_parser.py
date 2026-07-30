"""Disposable §9.1 harness: statement-level glossql parser.

Follows grammar.ebnf: statement forms are enforced strictly, clause payloads are
typed per clause (string / ident / number / balanced parens / expression span).
Embedded substrate SQL (DECLARE VIEW bodies, GLOSS queries, reading statements)
is consumed as opaque balanced spans, not parsed.

Known limitations, which are themselves findings about the spec:
- U1: provenance `BY` after an opaque SQL body is ambiguous with GROUP BY /
  ORDER BY; VIEW bodies are therefore parsed from the end (last BY + actor
  class). A reserved delimiter would remove this.
- U5 (new, found while building this): the spec has no reserved-word rule.
  Clause heads (IN, AS, FROM, WHERE, UNIT, ...) are legal inside transported
  expressions, so unparenthesized expression spans that contain them
  mis-delimit. Fixtures avoid the collision; the grammar cannot, as specified.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

TOKEN_RE = re.compile(
    r"""(?P<ws>\s+)
      | (?P<comment>--[^\n]*)
      | (?P<str>'(?:[^']|'')*')
      | (?P<num>\d+(?:\.\d+)?)
      | (?P<ident>[A-Za-z_][A-Za-z0-9_$]*)
      | (?P<assign>:=)
      | (?P<punct>.)
    """,
    re.VERBOSE | re.DOTALL,
)

ACTOR_CLASSES = {"USER", "AGENT", "DETECTOR", "SEED", "CALIBRATION"}
NAMED_CLASSES = {
    "SOURCE", "TABLE", "VIEW", "CONCEPT", "CONVENTION", "METRIC",
    "VALIDATION", "ASPECT", "HIERARCHY", "SERVING",
}
POLICY_KINDS = {"readiness", "contract", "interpretation"}
# First tokens that can open a clause; used to terminate expression spans.
CLAUSE_HEADS = {
    "KIND", "DESCRIPTION", "INDICATORS", "EXCLUDE", "UNIT", "ORDERING",
    "STATEMENT", "AS", "PARAMETER", "ON", "OVER", "TOLERANCE", "SEVERITY",
    "GUIDANCE", "DIRECTIONS", "VALUES", "FROM", "CONNECTION", "VIA", "IN",
    "LEVELS", "WHERE", "BANDS", "WEIGHT", "OVERALL", "BLOCK", "PREFER",
    "DIMENSION", "RESTRICT", "INCLUDE", "CARDINALITY", "REJECTED",
}


@dataclass
class Tok:
    kind: str  # str | num | ident | assign | punct
    text: str

    def up(self) -> str:
        return self.text.upper() if self.kind == "ident" else self.text


class ParseError(Exception):
    pass


def tokenize(src: str) -> list[Tok]:
    toks: list[Tok] = []
    for m in TOKEN_RE.finditer(src):
        kind = m.lastgroup
        if kind in ("ws", "comment"):
            continue
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

    def done(self) -> bool:
        return self.i >= len(self.toks)

    def at_provenance(self) -> bool:
        if not self.at_kw("BY"):
            return False
        nxt = self.peek(1)
        return nxt is not None and nxt.kind == "ident" and nxt.up() in ACTOR_CLASSES

    def balanced_parens(self) -> None:
        self.expect_punct("(")
        depth = 1
        while depth:
            t = self.take()
            if t.kind == "punct" and t.text == "(":
                depth += 1
            elif t.kind == "punct" and t.text == ")":
                depth -= 1

    def qualified(self) -> str:
        name = self.ident("subject")
        if self.peek() and self.peek().kind == "punct" and self.peek().text == ".":
            self.take()
            name += "." + self.ident("column")
        return name

    def value(self) -> None:
        t = self.peek()
        if t is None:
            raise ParseError("expected value")
        if t.kind in ("str", "num"):
            self.take()
        elif t.kind == "punct" and t.text == "(":
            self.balanced_parens()
        elif t.kind == "ident":
            self.qualified()
        else:
            raise ParseError(f"expected value, got {t.text!r}")

    def expr_span(self, stop_extra: set[str] = frozenset()) -> int:
        """Opaque expression: consume until a clause head / provenance / end.
        Leading parenthesized groups and function calls are consumed atomically."""
        consumed = 0
        while not self.done():
            if self.at_provenance():
                break
            t = self.peek()
            if t.kind == "ident" and (t.up() in CLAUSE_HEADS or t.up() in stop_extra):
                break
            if t.kind == "punct" and t.text == "(":
                self.balanced_parens()
            else:
                self.take()
            consumed += 1
        if consumed == 0:
            raise ParseError("expected expression")
        return consumed

    # -- provenance -------------------------------------------------------
    def provenance(self) -> None:
        self.expect_kw("BY")
        t = self.take()
        if not (t.kind == "ident" and t.up() in ACTOR_CLASSES):
            raise ParseError(f"expected actor class, got {t.text!r}")
        actor_class = t.up()
        t = self.take()
        if t.kind not in ("ident", "str"):
            raise ParseError(f"expected actor name, got {t.text!r}")
        if actor_class == "DETECTOR" and self.at_kw("WITNESS"):  # sprint 1 fork B
            self.take()
            self.ident("witness name")
        if self.at_kw("CONFIDENCE"):
            self.take()
            self.number()
        if self.at_kw("EVIDENCE"):
            self.take()
            self.string("evidence ref")
        if not self.done():
            raise ParseError(f"trailing tokens after provenance: {self.peek().text!r}")

    # -- clause machinery -------------------------------------------------
    def clause(self) -> None:
        kw = self.take()
        if kw.kind != "ident":
            raise ParseError(f"expected clause keyword, got {kw.text!r}")
        k = kw.up()
        if k in ("DESCRIPTION", "STATEMENT", "GUIDANCE"):
            self.string(f"{k} payload")
        elif k == "KIND" or k == "SEVERITY" or k == "PREFER" or k == "CONNECTION":
            self.ident(f"{k} payload")
        elif k in ("INDICATORS", "EXCLUDE", "OVER", "DIRECTIONS", "VALUES",
                   "INCLUDE", "LEVELS", "BANDS"):
            self.balanced_parens()
        elif k == "ORDERING":
            if self.peek() and self.peek().kind == "punct" and self.peek().text == "(":
                self.balanced_parens()
            else:
                self.ident("ORDERING payload")
        elif k == "UNIT":
            if self.at_kw("FROM"):
                self.take()
                self.ident("concept")
            else:
                self.string("unit literal")
        elif k == "TOLERANCE":
            self.number()
        elif k == "ON":
            self.expect_kw("CYCLE")
            self.ident("cycle name")
        elif k == "PARAMETER":
            self.ident("parameter name")
            if self.at_kw("GRAIN"):
                self.take()
                self.ident("grain")
            if self.at_kw("DEFAULT"):
                self.take()
                self.value()
        elif k == "AS":
            if self.peek() and self.peek().kind == "str":
                self.take()  # transported recipe body
            else:
                self.expr_span()
        elif k == "FROM":
            t = self.take()
            if t.kind not in ("ident", "str"):
                raise ParseError(f"expected FROM payload, got {t.text!r}")
        elif k == "VIA":
            self.string("VIA payload")
        elif k == "IN":
            self.ident("relation")
        elif k == "WHERE":
            self.expr_span()
        elif k == "DIMENSION":
            self.expect_kw("BUDGET")
            self.number()
        elif k == "RESTRICT":
            for w in ("JOINS", "TO", "DECLARED", "RELATIONSHIPS"):
                self.expect_kw(w)
        elif k == "OVERALL":
            self.expect_kw("THRESHOLD")
            self.number()
        elif k == "BLOCK":
            self.expect_kw("ON")
            self.balanced_parens()
        elif k == "WEIGHT":
            self.ident("aspect")
            self.expect_kw("FOR")
            self.ident("intent")
            self.balanced_parens()
        else:
            raise ParseError(f"unknown clause {kw.text!r}")

    def clauses_then_provenance(self) -> None:
        while not self.done() and not self.at_provenance():
            self.clause()
        self.provenance()

    # -- declarations -----------------------------------------------------
    def named_decl(self, cls: str) -> None:
        self.ident(f"{cls} name")
        if cls == "VIEW":
            self.view_body_then_provenance()
            return
        self.clauses_then_provenance()

    def view_body_then_provenance(self) -> None:
        self.expect_kw("AS")
        # U1: scan from the end for the trailing provenance clause.
        j = len(self.toks) - 1
        while j > self.i:
            t = self.toks[j]
            nxt = self.toks[j + 1] if j + 1 < len(self.toks) else None
            if (t.kind == "ident" and t.up() == "BY" and nxt is not None
                    and nxt.kind == "ident" and nxt.up() in ACTOR_CLASSES):
                break
            j -= 1
        else:
            raise ParseError("VIEW body has no trailing provenance")
        if j <= self.i:
            raise ParseError("empty VIEW body")
        self.i = j
        self.provenance()

    def relationship_decl(self) -> None:
        if self.at_kw("DISJOINT"):
            self.take()
            self.balanced_parens()
            self.provenance()
            return
        self.rel_operand()
        if self.at_kw("REFERENCES"):
            self.take()
            self.rel_operand()
            while self.at_kw("CARDINALITY") or self.at_kw("REJECTED"):
                if self.take().up() == "CARDINALITY":
                    self.ident("cardinality")
            self.provenance()
        elif self.at_kw("PART"):
            self.take()
            self.expect_kw("OF")
            self.ident("whole concept")
            self.provenance()
        elif self.at_kw("RECONCILES"):
            self.take()
            self.expect_kw("WITH")
            self.expr_span(stop_extra={"TOLERANCE"})
            if self.at_kw("TOLERANCE"):
                self.take()
                self.number()
            self.provenance()
        else:
            t = self.peek()
            got = t.text if t else "end of statement"
            raise ParseError(f"expected REFERENCES / PART OF / RECONCILES WITH, got {got!r}")

    def rel_operand(self) -> None:
        self.ident("relationship operand")
        t = self.peek()
        if t and t.kind == "punct" and t.text == ".":
            self.take()
            self.ident("column")
        elif t and t.kind == "punct" and t.text == "(":
            self.balanced_parens()

    def grounding_decl(self) -> None:
        self.ident("concept")
        self.expect_kw("IN")
        self.ident("relation")
        self.expect_kw("AS")
        self.expr_span()
        if self.at_kw("WHERE"):
            self.take()
            self.expr_span()
        self.provenance()

    def reliability_decl(self) -> None:
        t = self.take()
        if not (t.kind == "ident" and t.up() in ACTOR_CLASSES):
            raise ParseError(f"expected actor class, got {t.text!r}")
        actor_class = t.up()
        t = self.take()
        if t.kind not in ("ident", "str"):
            raise ParseError(f"expected actor name, got {t.text!r}")
        if actor_class == "DETECTOR" and self.at_kw("WITNESS"):  # sprint 1 fork B
            self.take()
            self.ident("witness name")
        self.expect_kw("FOR")
        self.ident("aspect")
        self.number()
        self.provenance()

    def policy_decl(self) -> None:
        kind = self.ident("policy kind")
        if kind not in POLICY_KINDS:
            raise ParseError(f"unknown policy kind {kind!r}")
        if (self.peek() and self.peek().kind == "ident"
                and not self.at_kw("FOR") and not self.at_provenance()
                and self.peek().up() not in CLAUSE_HEADS):
            self.take()  # policy name (contract form)
        if self.at_kw("FOR"):
            self.take()
            self.ident("policy key")
        self.clauses_then_provenance()

    def application(self, head: str, provenance_required: bool = True) -> None:
        self.expect_punct("(")
        if self.at_kw("WORKSPACE"):
            self.take()
        else:
            self.qualified()
        while self.peek() and self.peek().kind == "punct" and self.peek().text == ",":
            self.take()
            self.ident("argument name")
            t = self.take()
            if not (t.kind == "assign"):
                raise ParseError(f"expected :=, got {t.text!r}")
            self.value()
        self.expect_punct(")")
        self.provenance()

    # -- statements -------------------------------------------------------
    def statement(self) -> None:
        t = self.peek()
        if t is None:
            return
        head = t.up() if t.kind == "ident" else None
        if head == "DECLARE":
            self.take()
            self.declaration()
        elif head == "WITNESS":
            self.take()
            self.ident("aspect")
            self.application("WITNESS")
        elif head == "RETRACT":
            self.take()
            self.ident("aspect")
            self.application("RETRACT")
        elif head == "OBSERVE":
            self.take()
            self.observation()
        elif head == "GLOSS":
            self.take()
            self.gloss()
        elif head == "AT":
            self.take()
            self.string("timestamp")
            if self.at_kw("GLOSS"):
                self.take()
                self.gloss()
            else:
                self.passthrough()
        else:
            self.passthrough()  # substrate SQL (SELECT ... FROM CONTEXT(...) etc.)

    def declaration(self) -> None:
        t = self.take()
        if t.kind != "ident":
            raise ParseError(f"expected class or aspect after DECLARE, got {t.text!r}")
        cls = t.up()
        if cls == "CYCLE":
            self.expect_kw("FAMILY")
            self.named_decl("CYCLE FAMILY")
        elif cls in NAMED_CLASSES:
            self.named_decl(cls)
        elif cls == "RELATIONSHIP":
            self.relationship_decl()
        elif cls == "GROUNDING":
            self.grounding_decl()
        elif cls == "RELIABILITY":
            self.reliability_decl()
        elif cls == "POLICY":
            self.policy_decl()
        elif self.peek() and self.peek().kind == "punct" and self.peek().text == "(":
            self.application(t.text)  # aspect application
        else:
            raise ParseError(f"unknown declaration class {t.text!r}")

    def observation(self) -> None:
        self.ident("measurement id")
        while self.peek() and self.peek().kind == "punct" and self.peek().text == ",":
            self.take()
            self.ident("measurement id")
        if self.at_kw("ON"):
            self.take()
            if self.peek() and self.peek().kind == "punct" and self.peek().text == "(":
                self.balanced_parens()
            else:
                self.qualified()
        if self.peek() and self.peek().kind == "ident" and not self.at_provenance():
            self.take()  # OBSERVE validation <name>
        if self.at_provenance():  # examples carry no BY — spec inconsistency, tolerated
            self.provenance()
        elif not self.done():
            raise ParseError(f"trailing tokens in OBSERVE: {self.peek().text!r}")

    def gloss(self) -> None:
        t = self.peek()
        if t and t.kind == "punct" and t.text == "(":
            self.balanced_parens()
        else:
            self.qualified()
        if self.at_kw("USING"):
            self.take()
            self.expect_kw("SERVING")
            self.ident("serving policy")
        if self.at_kw("AS"):
            self.take()
            self.expect_kw("PACK")
        if not self.done():
            raise ParseError(f"trailing tokens in GLOSS: {self.peek().text!r}")

    def passthrough(self) -> None:
        while not self.done():
            self.take()


def parse_statement(toks: list[Tok]) -> None:
    p = P(toks)
    p.statement()
    if not p.done():
        raise ParseError(f"trailing tokens: {p.peek().text!r}")


def check_source(src: str) -> list[tuple[str, str | None]]:
    """Parse every statement in src. Returns [(statement_preview, error|None)]."""
    results = []
    for stmt in split_statements(tokenize(src)):
        preview = " ".join(t.text for t in stmt[:8])
        try:
            parse_statement(stmt)
            results.append((preview, None))
        except ParseError as e:
            results.append((preview, str(e)))
    return results
