//! Store behavior the language fixes: admission by aspect kind (SPEC.md
//! §5.2), the witness speaker gate and detector eligibility (§7.1),
//! supersession per (subject, aspect, actor kind), the provisional collapse
//! policy (§5.3), and cache semantics (§6).

use glossql_glossary::{Actor, ActorKind, Error, ReadContext, Scope, Store};
use glossql_parser::{Declaration, Gloss, GlossqlParser, Statement};

fn decl(sql: &str) -> Declaration {
    match GlossqlParser::parse_sql(sql)
        .expect("declaration parses")
        .remove(0)
    {
        Statement::Declare(d) => *d,
        other => panic!("not a declaration: {other:?}"),
    }
}

fn gloss(sql: &str) -> Gloss {
    match GlossqlParser::parse_sql(sql)
        .expect("gloss parses")
        .remove(0)
    {
        Statement::Gloss(g) => g,
        other => panic!("not a gloss: {other:?}"),
    }
}

async fn store() -> Store {
    let store = Store::open_memory().await.unwrap();
    let Declaration::Aspect(unit) = decl(
        r#"DECLARE ASPECT unit WITH $${
            "type": "object",
            "required": ["value"],
            "properties": {"value": {"type": "string"}},
            "additionalProperties": false
        }$$ AS FACT;"#,
    ) else {
        unreachable!()
    };
    store.declare_aspect(&unit).await.unwrap();
    store
}

fn agent() -> Actor {
    Actor {
        kind: ActorKind::Agent,
        id: "agent-1".into(),
    }
}

fn human() -> Actor {
    Actor {
        kind: ActorKind::Human,
        id: "philipp".into(),
    }
}

async fn write(store: &Store, actor: &Actor, statement: &str) -> Result<(), Error> {
    let g = gloss(statement);
    store
        .gloss("fin", actor, &g.aspect.value, "orders.amount", &g.body, None)
        .await
}

// -- admission by aspect kind --------------------------------------------

#[tokio::test]
async fn unknown_aspect_is_rejected() {
    let s = store().await;
    let e = write(
        &s,
        &agent(),
        r#"GLOSS nope ON orders.amount AS $${"value": "x"}$$;"#,
    )
    .await
    .unwrap_err();
    assert!(matches!(e, Error::Unknown { what: "aspect", .. }), "{e}");
}

#[tokio::test]
async fn fact_body_must_match_the_with_schema() {
    let s = store().await;
    let e = write(
        &s,
        &agent(),
        r#"GLOSS unit ON orders.amount AS $${"wrong": 1}$$;"#,
    )
    .await
    .unwrap_err();
    assert!(matches!(e, Error::BodyRejected { .. }), "{e}");
    write(
        &s,
        &agent(),
        r#"GLOSS unit ON orders.amount AS $${"value": "EUR"}$$;"#,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn query_gloss_validates_against_the_grounding_schema() {
    let s = store().await;
    let Declaration::Aspect(revenue) = decl(
        r#"DECLARE ASPECT revenue WITH $${"title": "revenue", "x-kind": "measure"}$$ AS QUERY;"#,
    ) else {
        unreachable!()
    };
    s.declare_aspect(&revenue).await.unwrap();
    let e = write(
        &s,
        &agent(),
        r#"GLOSS revenue ON orders.amount AS $${"prose": "no sql"}$$;"#,
    )
    .await
    .unwrap_err();
    assert!(matches!(e, Error::BodyRejected { .. }), "{e}");
    write(
        &s,
        &agent(),
        r#"GLOSS revenue ON orders.amount AS $${"sql": "SELECT amount FROM orders"}$$;"#,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn measurement_aspects_are_never_glossed() {
    let s = store().await;
    let Declaration::Aspect(m) =
        decl(r#"DECLARE ASPECT min_max WITH $${"type": "object"}$$ AS MEASUREMENT;"#)
    else {
        unreachable!()
    };
    s.declare_aspect(&m).await.unwrap();
    let e = write(
        &s,
        &agent(),
        r#"GLOSS min_max ON orders.amount AS $${"min": 1}$$;"#,
    )
    .await
    .unwrap_err();
    assert!(matches!(e, Error::MeasurementGloss(_)), "{e}");
}

// -- the witness speaker gate --------------------------------------------

#[tokio::test]
async fn witness_by_list_gates_actor_kinds() {
    let s = store().await;
    let Declaration::Witness(w) = decl("DECLARE WITNESS unit_w ON unit BY (HUMAN);") else {
        unreachable!()
    };
    s.declare_witness(&w).await.unwrap();
    let e = write(
        &s,
        &agent(),
        r#"GLOSS unit ON orders.amount AS $${"value": "EUR"}$$;"#,
    )
    .await
    .unwrap_err();
    assert!(matches!(e, Error::SpeakerNotAdmitted { .. }), "{e}");
    write(
        &s,
        &human(),
        r#"GLOSS unit ON orders.amount AS $${"value": "EUR"}$$;"#,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn measurement_aspects_take_one_producer_and_no_speaker_gate() {
    let s = store().await;
    let Declaration::Aspect(m) =
        decl(r#"DECLARE ASPECT min_max WITH $${"type": "object"}$$ AS MEASUREMENT;"#)
    else {
        unreachable!()
    };
    s.declare_aspect(&m).await.unwrap();
    let Declaration::Function(f) =
        decl("DECLARE FUNCTION profile_min_max FOR fin FROM 'p.rhai' RETURNS min_max;")
    else {
        unreachable!()
    };
    s.declare_function(&f).await.unwrap();

    // One producer per MEASUREMENT aspect — a second function is refused.
    let Declaration::Function(rival) =
        decl("DECLARE FUNCTION other_min_max FOR fin FROM 'o.rhai' RETURNS min_max;")
    else {
        unreachable!()
    };
    let e = s.declare_function(&rival).await.unwrap_err();
    assert!(matches!(e, Error::MeasurementProducerTaken { .. }), "{e}");

    // Nobody glosses a measurement, so BY is refused on its witness.
    let Declaration::Witness(bad) = decl("DECLARE WITNESS m_w ON min_max BY (AGENT);") else {
        unreachable!()
    };
    let e = s.declare_witness(&bad).await.unwrap_err();
    assert!(matches!(e, Error::MeasurementWitnessSpeakers(_)), "{e}");
}

#[tokio::test]
async fn a_detector_is_a_function_without_returns() {
    let s = store().await;
    let Declaration::Function(f) = decl("DECLARE FUNCTION vibes FOR fin FROM 'v.rhai' RETURNS unit;")
    else {
        unreachable!()
    };
    s.declare_function(&f).await.unwrap();
    // A function that RETURNS an aspect is a voice, never a detector.
    let Declaration::Witness(w) =
        decl("DECLARE WITNESS unit_w ON unit BY (AGENT, HUMAN) DETECTOR vibes;")
    else {
        unreachable!()
    };
    let e = s.declare_witness(&w).await.unwrap_err();
    assert!(matches!(e, Error::DetectorNotEligible { .. }), "{e}");

    // A witness naming neither BY nor DETECTOR declares nothing.
    let Declaration::Witness(empty) = decl("DECLARE WITNESS unit_w ON unit;") else {
        unreachable!()
    };
    let e = s.declare_witness(&empty).await.unwrap_err();
    assert!(matches!(e, Error::WitnessNamesNothing(_)), "{e}");
}

#[tokio::test]
async fn threshold_outside_unit_interval_is_rejected() {
    let s = store().await;
    let Declaration::Witness(w) =
        decl("DECLARE WITNESS unit_w ON unit BY (AGENT, HUMAN) THRESHOLD 1.7;")
    else {
        unreachable!()
    };
    assert!(s.declare_witness(&w).await.is_err());
}

#[tokio::test]
async fn redeclaring_an_aspect_is_content_idempotent_but_refused_once_glossed() {
    let s = store().await;
    // Same content, different whitespace: a no-op, not a replace.
    let Declaration::Aspect(same) = decl(
        r#"DECLARE ASPECT unit WITH $${"type":"object","required":["value"],"properties":{"value":{"type":"string"}},"additionalProperties":false}$$ AS FACT;"#,
    ) else {
        unreachable!()
    };
    s.declare_aspect(&same).await.unwrap();

    // Changing it is fine while nothing is glossed under it…
    let Declaration::Aspect(changed) =
        decl(r#"DECLARE ASPECT unit WITH $${"type": "object"}$$ AS FACT;"#)
    else {
        unreachable!()
    };
    s.declare_aspect(&changed).await.unwrap();

    // …and refused once something is.
    write(
        &s,
        &agent(),
        r#"GLOSS unit ON orders.amount AS $${"value": "EUR"}$$;"#,
    )
    .await
    .unwrap();
    let Declaration::Aspect(again) =
        decl(r#"DECLARE ASPECT unit WITH $${"type": "object", "properties": {}}$$ AS FACT;"#)
    else {
        unreachable!()
    };
    let e = s.declare_aspect(&again).await.unwrap_err();
    assert!(matches!(e, Error::AspectInUse { .. }), "{e}");
}

// -- supersession and collapse -------------------------------------------

#[tokio::test]
async fn supersession_is_per_subject_aspect_actor_kind() {
    let s = store().await;
    write(
        &s,
        &agent(),
        r#"GLOSS unit ON orders.amount AS $${"value": "USD"}$$;"#,
    )
    .await
    .unwrap();
    write(
        &s,
        &agent(),
        r#"GLOSS unit ON orders.amount AS $${"value": "EUR"}$$;"#,
    )
    .await
    .unwrap();
    let rows = s
        .raw_read("fin", &Scope::Subject("orders.amount".into()), None)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1, "agent slot holds one current value");
    assert!(
        rows[0].body.contains("EUR"),
        "latest wins: {}",
        rows[0].body
    );

    write(
        &s,
        &human(),
        r#"GLOSS unit ON orders.amount AS $${"value": "CHF"}$$;"#,
    )
    .await
    .unwrap();
    let rows = s
        .raw_read("fin", &Scope::Subject("orders.amount".into()), None)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2, "human slot is separate");
}

#[tokio::test]
async fn collapse_serves_by_precedence_human_over_agent() {
    let s = store().await;
    let ctx = ReadContext::default();
    write(
        &s,
        &agent(),
        r#"GLOSS unit ON orders.amount AS $${"value": "EUR"}$$;"#,
    )
    .await
    .unwrap();
    let rows = s
        .collapsed_read("fin", &Scope::Subject("orders.amount".into()), None, &ctx)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].value.as_deref().unwrap().contains("EUR"));
    assert_eq!(rows[0].state, "current");

    write(
        &s,
        &human(),
        r#"GLOSS unit ON orders.amount AS $${"value": "USD"}$$;"#,
    )
    .await
    .unwrap();
    let rows = s
        .collapsed_read("fin", &Scope::Subject("orders.amount".into()), None, &ctx)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0].value.as_deref().unwrap().contains("USD"),
        "the human slot outranks the agent slot (ruled 2026-08-04)"
    );
}

// -- functions and the cache ---------------------------------------------

#[tokio::test]
async fn accepts_names_must_be_declared_aspects() {
    let s = store().await;
    let Declaration::Function(good) = decl(
        r#"DECLARE FUNCTION f FOR fin FROM 'f.rhai' ACCEPTS (unit);"#,
    ) else {
        unreachable!()
    };
    s.declare_function(&good).await.unwrap();
    let row = s.function("f", Some("fin")).await.unwrap().unwrap();
    assert_eq!(row.accepts, vec!["unit"]);

    let Declaration::Function(bad) = decl(
        r#"DECLARE FUNCTION g FOR fin FROM 'g.rhai' ACCEPTS (nope);"#,
    ) else {
        unreachable!()
    };
    let e = s.declare_function(&bad).await.unwrap_err();
    assert!(matches!(e, Error::Unknown { what: "aspect", .. }), "{e}");
}

#[tokio::test]
async fn function_scope_gates_visibility() {
    let s = store().await;
    let Declaration::Function(f) =
        decl(r#"DECLARE FUNCTION profile FOR fin FROM 'p.rhai';"#)
    else {
        unreachable!()
    };
    s.declare_function(&f).await.unwrap();
    assert!(s.function("profile", Some("fin")).await.unwrap().is_some());
    assert!(s.function("profile", Some("crm")).await.unwrap().is_none());
}

#[tokio::test]
async fn cache_serves_the_latest_row_per_subject_and_function() {
    let s = store().await;
    s.cache_put("fin", "orders", "profile", r#"{"n": 1}"#, None)
        .await
        .unwrap();
    s.cache_put("fin", "orders", "profile", r#"{"n": 2}"#, None)
        .await
        .unwrap();
    let row = s
        .cache_get("fin", "orders", "profile")
        .await
        .unwrap()
        .unwrap();
    assert!(row.body.contains('2'));
}

#[tokio::test]
async fn forwarded_deletes_only_touch_the_two_relations() {
    let s = store().await;
    write(
        &s,
        &agent(),
        r#"GLOSS unit ON orders.amount AS $${"value": "EUR"}$$;"#,
    )
    .await
    .unwrap();
    let n = s
        .forward_delete(
            "glossary",
            "DELETE FROM glossary WHERE subject = 'orders.amount' AND aspect = 'unit'",
        )
        .await
        .unwrap();
    assert_eq!(n, 1);
    let e = s
        .forward_delete("aspects", "DELETE FROM aspects")
        .await
        .unwrap_err();
    assert!(matches!(e, Error::ForwardRejected(_)), "{e}");
}

#[tokio::test]
async fn a_glossary_delete_invalidates_detector_verdicts() {
    let s = store().await;
    // One detector (no RETURNS), one extraction function (RETURNS) —
    // only the detector's cache is a verdict about slots.
    let Declaration::Function(det) = decl("DECLARE FUNCTION entropy FOR fin FROM 'e.rhai';")
    else {
        unreachable!()
    };
    s.declare_function(&det).await.unwrap();
    let Declaration::Function(prod) =
        decl("DECLARE FUNCTION vibes FOR fin FROM 'v.rhai' RETURNS unit;")
    else {
        unreachable!()
    };
    s.declare_function(&prod).await.unwrap();

    write(
        &s,
        &agent(),
        r#"GLOSS unit ON orders.amount AS $${"value": "EUR"}$$;"#,
    )
    .await
    .unwrap();
    s.cache_put("fin", "orders.amount", "entropy", r#"{"band": "red"}"#, None)
        .await
        .unwrap();
    s.cache_put("fin", "orders.amount", "vibes", r#"{"n": 1}"#, None)
        .await
        .unwrap();

    // Striking the slot invalidates the verdict about it: deletion makes
    // the slot set smaller, never newer, so timestamp freshness alone
    // would keep serving a verdict about slots that no longer exist.
    s.forward_delete(
        "glossary",
        "DELETE FROM glossary WHERE subject = 'orders.amount' AND actor_kind = 'agent'",
    )
    .await
    .unwrap();
    assert!(
        s.cache_get("fin", "orders.amount", "entropy")
            .await
            .unwrap()
            .is_none(),
        "the detector verdict is gone — it recomputes at the next read"
    );
    assert!(
        s.cache_get("fin", "orders.amount", "vibes")
            .await
            .unwrap()
            .is_some(),
        "extraction caches answer to ACCEPTS invalidation, not slot deletion"
    );
}

// -- recipe admission (SPEC.md §3) ----------------------------------------

#[tokio::test]
async fn recipe_redeclare_is_content_idempotent_and_change_is_refused() {
    use glossql_glossary::RecipeAdmission;

    let s = store().await;
    for setup in [
        "DECLARE DATASET fin SET (purpose: 'test');",
        "DECLARE SOURCE erp SET (type: parquet, location: 'lake/erp');",
    ] {
        match decl(setup) {
            Declaration::Dataset(d) => s.declare_dataset(&d).await.unwrap(),
            Declaration::Source(d) => s.declare_source(&d).await.unwrap(),
            other => panic!("unexpected: {other:?}"),
        }
    }
    let recipe = |sql: &str| match decl(sql) {
        Declaration::Recipe(r) => r,
        other => panic!("not a recipe: {other:?}"),
    };

    let v1 = recipe("DECLARE RECIPE orders ON fin FROM erp AS $$SELECT * FROM read_parquet('orders/*.parquet')$$;");
    assert_eq!(s.declare_recipe(&v1).await.unwrap(), RecipeAdmission::Created);
    assert_eq!(
        s.declare_recipe(&v1).await.unwrap(),
        RecipeAdmission::Unchanged
    );

    // A changed SQL is refused outright — replacement is postponed
    // (project lead, 2026-08-04); a different SQL is a different table.
    let v2 = recipe("DECLARE RECIPE orders ON fin FROM erp AS $$SELECT * FROM read_parquet('orders_v2/*.parquet')$$;");
    let e = s.declare_recipe(&v2).await.unwrap_err();
    assert!(
        matches!(e, Error::RecipeChanged { ref table } if table == "orders"),
        "{e}"
    );
    // The unchanged spelling still no-ops.
    assert_eq!(
        s.declare_recipe(&v1).await.unwrap(),
        RecipeAdmission::Unchanged
    );
}

// -- writes invalidate (project lead, 2026-08-04) --------------------------

#[tokio::test]
async fn a_gloss_invalidates_the_caches_of_functions_accepting_its_aspect() {
    let s = store().await;
    let Declaration::Function(f) = decl(
        r#"DECLARE FUNCTION conv FOR GLOBAL FROM 'conv.rhai' ACCEPTS (unit);"#,
    ) else {
        unreachable!()
    };
    s.declare_function(&f).await.unwrap();
    s.cache_put("fin", "orders.amount", "conv", "{}", None)
        .await
        .unwrap();
    s.cache_put("fin", "invoices.total", "conv", "{}", None)
        .await
        .unwrap();

    // At or under the subject: the other table's row survives.
    write(
        &s,
        &agent(),
        r#"GLOSS unit ON orders.amount AS $${"value": "EUR"}$$;"#,
    )
    .await
    .unwrap();
    assert!(
        s.cache_get("fin", "orders.amount", "conv")
            .await
            .unwrap()
            .is_none(),
        "the dependent evidence died with the write"
    );
    assert!(
        s.cache_get("fin", "invoices.total", "conv")
            .await
            .unwrap()
            .is_some()
    );

    // A dataset-level gloss sweeps the dataset.
    let g = gloss(r#"GLOSS unit ON fin AS $${"value": "EUR"}$$;"#);
    s.gloss("fin", &agent(), "unit", "fin", &g.body, None)
        .await
        .unwrap();
    assert!(
        s.cache_get("fin", "invoices.total", "conv")
            .await
            .unwrap()
            .is_none()
    );
}

// -- aspect grain (ruled 2026-08-05) --------------------------------------

#[tokio::test]
async fn grain_gates_glosses_and_bounds_disclosure() {
    let s = store().await;
    let Declaration::Aspect(role) = decl(
        r#"DECLARE ASPECT role WITH $${
            "type": "object", "properties": {"value": {"type": "string"}}
        }$$ AS FACT ON COLUMN;"#,
    ) else {
        unreachable!()
    };
    s.declare_aspect(&role).await.unwrap();
    // A witness puts the aspect on the disclosure grid.
    let Declaration::Witness(w) = decl("DECLARE WITNESS role_w ON role BY (AGENT, HUMAN);")
    else {
        unreachable!()
    };
    s.declare_witness(&w).await.unwrap();

    // In-grain lands; a table subject is refused with the grain named.
    let g = gloss(r#"GLOSS role ON orders.amount AS $${"value": "measure"}$$;"#);
    s.gloss("fin", &agent(), "role", "orders.amount", &g.body, None)
        .await
        .unwrap();
    let e = s
        .gloss("fin", &agent(), "role", "orders", &g.body, None)
        .await
        .unwrap_err();
    assert!(
        matches!(e, Error::GrainRefused { grain: "table", .. }),
        "{e}"
    );

    // Disclosure stays within grain: the unglossed column is a visible
    // absence, the table never shows a role row at all.
    let ctx = ReadContext {
        universe: vec![
            "orders".into(),
            "orders.amount".into(),
            "orders.qty".into(),
        ],
        ..Default::default()
    };
    let rows = s
        .collapsed_read("fin", &Scope::Dataset, None, &ctx)
        .await
        .unwrap();
    let states: Vec<(String, String)> = rows
        .iter()
        .filter(|r| r.aspect == "role")
        .map(|r| (r.subject.clone(), r.state.clone()))
        .collect();
    assert!(
        states.contains(&("orders.amount".into(), "current".into())),
        "{states:?}"
    );
    assert!(
        states.contains(&("orders.qty".into(), "unassessed".into())),
        "{states:?}"
    );
    assert!(
        !states.iter().any(|(subject, _)| subject == "orders"),
        "{states:?}"
    );
}
