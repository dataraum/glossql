// The typing decision (ruled 2026-08-04): the function's pick fills the
// `type` slot by default; agents and humans supersede it by glossing. Reads
// the candidates measurement from context (ACCEPTS type_candidates,
// type_patterns), picks the best above min_confidence, falls back to
// VARCHAR — v0.3's automatic decision, as a script.
let min_conf = 0.85;
if context.type_patterns != () && "min_confidence" in context.type_patterns {
    min_conf = context.type_patterns.min_confidence;
}
let best = ();
if context.type_candidates != () {
    for c in context.type_candidates.candidates {
        if c.confidence >= min_conf && (best == () || c.confidence > best.confidence) {
            best = c;
        }
    }
}
if best == () {
    #{ value: "VARCHAR" }
} else if "expr" in best {
    #{ value: best.type, expr: best.expr }
} else {
    #{ value: best.type }
}
