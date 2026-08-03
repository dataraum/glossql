//! AST snapshots — one representative statement per glossql grammar form.
//! Substrate statements are DataFusion's AST and are covered by behavior
//! tests instead of snapshots.

use glossql_parser::GlossqlParser;

macro_rules! snap {
    ($name:ident, $src:expr) => {
        #[test]
        fn $name() {
            insta::assert_debug_snapshot!(GlossqlParser::parse_sql($src).expect("must parse"));
        }
    };
}

snap!(
    source_decl,
    "DECLARE SOURCE crm SET (type: relational_db, location: 'postgres://crm.internal/prod', via: crm_prod);"
);
snap!(
    recipe_decl,
    "DECLARE RECIPE segments ON fin FROM crm AS $$SELECT id, segment FROM customer_segments$$;"
);
snap!(
    dataset_decl_and_use,
    "DECLARE DATASET fin SET (purpose: 'working-capital analysis');\nUSE fin;"
);
snap!(
    relationship_decls,
    "DECLARE RELATIONSHIP orders.customer_id -> customers.id;\nDECLARE RELATIONSHIP invoices.order_id <-> orders.id;"
);
snap!(
    aspect_decl_fact,
    r#"DECLARE ASPECT unit WITH $${"type": "object", "properties": {"value": {"type": "string"}}}$$ AS FACT;"#
);
snap!(
    aspect_decl_measurement,
    r#"DECLARE ASPECT min_max WITH $${"type": "object", "properties": {"min": {}, "max": {}}}$$ AS MEASUREMENT;"#
);
snap!(
    gloss_fact,
    r#"GLOSS unit ON orders.amount AS $${"value": "EUR", "source_column": "currency_code"}$$;"#
);
snap!(
    gloss_on_pair_path,
    r#"GLOSS fk_note ON orders.customer_id -> customers.id AS $${"value": "2% orphaned rows"}$$;"#
);
snap!(
    gloss_body_with_escapes,
    r#"GLOSS type_patterns ON fin AS $${"expr": "STRPTIME(\"{col}\", '%d.%m.%Y')"}$$;"#
);
snap!(
    function_decl_accepts_aspects,
    r#"DECLARE FUNCTION infer_types FOR GLOBAL FROM 'functions/infer_types.rhai' ACCEPTS (type_patterns, null_values) RETURNS $${"type": "object"}$$;"#
);
snap!(
    function_decl_no_accepts,
    r#"DECLARE FUNCTION profile_min_max FOR fin FROM 'functions/profile_min_max.rhai' RETURNS $${"type": "object", "required": ["value"]}$$;"#
);
snap!(
    witness_decl_full,
    "DECLARE WITNESS behavior_w ON behavior BY (FUNCTION temporal_behavior, AGENT, HUMAN) DETECTOR behavior_entropy THRESHOLD 0.7;"
);
snap!(
    witness_decl_minimal,
    "DECLARE WITNESS min_max_w ON min_max BY (FUNCTION profile_min_max);"
);
snap!(
    extract_two_calls,
    "SELECT outliers(), profile() FROM fin.orders;"
);
