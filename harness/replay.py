"""§9.1 harness: log replay and pooling simulator.

Replays a sequence of glossql writing statements into derived state:
supersession slots, witness pooling under declared reliabilities, contested
flags, and prefix replay (the AT semantics). This is the executable check
behind SPEC.md §5 and the §10 walkthrough — not an implementation; it dies
with the harness.

Semantics implemented (SPEC.md):
- §3.0 supersession: a claim slot is (subject, aspect, argument-tuple); the
  latest DECLARE wins; RETRACT vacates.
- §3.3 witnesses: keyed (slot, detector, witness); bare DETECTOR x is
  shorthand for WITNESS x (sprint 1 fork B).
- §3.4/§5 pooling: linear pool weighted by declared reliability per
  (detector, witness, aspect); an undeclared producer pools at weight 0.
- §5 contested: a slot with a declaration whose pooled posterior puts its
  mass elsewhere (argmax != declared label).
- AT: prefix replay — state = f(log <= t).
"""

from __future__ import annotations

from glossql_parser import Tok, split_statements, tokenize


def _upper(t: Tok) -> str:
    return t.text.upper() if t.kind == "ident" else ""


class Replayer:
    def __init__(self):
        self.aspects: dict[str, dict] = {}          # name -> {arguments, values, terminal}
        self.slots: dict[tuple, dict] = {}          # slot -> {value, actor}
        self.witnesses: dict[tuple, dict] = {}      # slot -> {(det, wit): {label: p}}
        self.reliabilities: dict[tuple, float] = {} # (det, wit, aspect) -> r
        self.statements_seen = 0

    # -- statement dissection (tokens are already grammar-valid) ----------
    def _call_parts(self, toks: list[Tok], start: int):
        """subject and name:=value pairs from `head ( subject, a := v, ... )`."""
        i = start
        assert toks[i].text == "("
        i += 1
        subj = toks[i].text
        i += 1
        while toks[i].text == ".":
            subj += "." + toks[i + 1].text
            i += 2
        pairs = []
        while toks[i].text != ")":
            assert toks[i].text == ","
            name = toks[i + 1].text
            assert toks[i + 2].kind == "assign"
            i += 3
            if toks[i].text == "(":
                depth, val = 1, "("
                while depth:
                    i += 1
                    val += toks[i].text
                    depth += toks[i].text == "(" and 1 or 0
                    depth -= toks[i].text == ")" and 1 or 0
                i += 1
            else:
                val = toks[i].text
                i += 1
                while i < len(toks) and toks[i].text == ".":
                    val += "." + toks[i + 1].text
                    i += 2
            pairs.append((name, val))
        return subj, pairs, i + 1

    def _paren_list(self, toks: list[Tok], i: int):
        assert toks[i].text == "("
        items, i = [], i + 1
        while toks[i].text != ")":
            if toks[i].kind == "ident":
                items.append(toks[i].text)
            i += 1
        return items, i + 1

    def _split_args_labels(self, aspect: str, pairs):
        decl = self.aspects.get(aspect, {})
        argnames = set(decl.get("arguments", []))
        args, labels = [], {}
        for name, val in pairs:
            if name in argnames or (name == "value") or not _is_num(val):
                args.append((name, val))
            else:
                labels[name] = float(val)
        return tuple(a for a in args if a[0] != "value"), dict(a for a in args), labels

    # -- apply ------------------------------------------------------------
    def apply(self, toks: list[Tok]) -> None:
        self.statements_seen += 1
        head = _upper(toks[0])
        if head == "WITNESS":
            aspect = toks[1].text
            subj, pairs, i = self._call_parts(toks, 2)
            argkey, _, labels = self._split_args_labels(aspect, pairs)
            det, wit = self._actor(toks, i)
            slot = (subj, aspect, argkey)
            self.witnesses.setdefault(slot, {})[(det, wit)] = labels
        elif head == "RETRACT":
            aspect = toks[1].text
            subj, pairs, _ = self._call_parts(toks, 2)
            argkey, _, _ = self._split_args_labels(aspect, pairs)
            self.slots.pop((subj, aspect, argkey), None)
        elif head == "DECLARE":
            cls = _upper(toks[1])
            if cls == "ASPECT":
                self._declare_aspect(toks)
            elif cls == "RELIABILITY":
                self._declare_reliability(toks)
            elif cls not in ("SOURCE", "TABLE", "VIEW", "CONCEPT", "CONVENTION",
                             "METRIC", "VALIDATION", "CYCLE", "HIERARCHY",
                             "SERVING", "RELATIONSHIP", "GROUNDING", "POLICY") \
                    and len(toks) > 2 and toks[2].text == "(":
                aspect = toks[1].text
                subj, pairs, i = self._call_parts(toks, 2)
                argkey, argmap, _ = self._split_args_labels(aspect, pairs)
                actor = f"{_upper(toks[i + 1])} {toks[i + 2].text}"
                self.slots[(subj, aspect, argkey)] = {
                    "value": argmap.get("value"), "actor": actor}
            # named/keyed declarations: recorded implicitly, no derived state here

    def _declare_aspect(self, toks: list[Tok]) -> None:
        name, decl, i = toks[2].text, {}, 3
        while i < len(toks):
            kw = _upper(toks[i])
            if kw in ("ARGUMENTS", "VALUES", "TERMINAL"):
                items, i = self._paren_list(toks, i + 1)
                decl[kw.lower()] = items
            elif kw == "BY":
                break
            else:
                i += 1
        self.aspects[name] = decl

    def _declare_reliability(self, toks: list[Tok]) -> None:
        det, wit, i = toks[3].text, None, 4
        if _upper(toks[i]) == "WITNESS":
            wit = toks[i + 1].text
            i += 2
        assert _upper(toks[i]) == "FOR"
        aspect = toks[i + 1].text
        r = float(toks[i + 2].text)
        self.reliabilities[(det, wit or det, aspect)] = r

    def _actor(self, toks: list[Tok], i: int):
        assert _upper(toks[i]) == "BY" and _upper(toks[i + 1]) == "DETECTOR"
        det = toks[i + 2].text
        wit = det
        if i + 3 < len(toks) and _upper(toks[i + 3]) == "WITNESS":
            wit = toks[i + 4].text
        return det, wit

    # -- derived plane ----------------------------------------------------
    def posterior(self, slot: tuple) -> dict | None:
        pooled: dict[str, float] = {}
        total = 0.0
        for (det, wit), dist in self.witnesses.get(slot, {}).items():
            r = self.reliabilities.get((det, wit, slot[1]), 0.0)
            if r <= 0:
                continue
            for label, p in dist.items():
                pooled[label] = pooled.get(label, 0.0) + r * p
            total += r
        if total == 0:
            return None
        return {k: v / total for k, v in pooled.items()}

    def contested(self, slot: tuple) -> bool | None:
        decl = self.slots.get(slot)
        post = self.posterior(slot)
        if decl is None or post is None or decl["value"] is None:
            return None
        return max(post, key=post.get) != decl["value"]


def _is_num(s: str) -> bool:
    try:
        float(s)
        return True
    except ValueError:
        return False


READ_HEADS = {"SELECT", "GLOSS", "AT", "WITH"}


def replay(source: str, upto: int | None = None) -> Replayer:
    """Replay writing statements (reading statements are skipped — §3.0)."""
    r = Replayer()
    for n, stmt in enumerate(split_statements(tokenize(source))):
        if upto is not None and n >= upto:
            break
        if _upper(stmt[0]) in READ_HEADS:
            continue
        r.apply(stmt)
    return r


def statement_count(source: str) -> int:
    return len(split_statements(tokenize(source)))
