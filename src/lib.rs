use std::str::FromStr;

use ast_grep_config::{CombinedScan, RuleConfig, SerializableRuleConfig};
use ast_grep_core::{AstGrep, StrDoc};
use ast_grep_language::SupportLang;
use pgrx::{prelude::*, JsonB};

::pgrx::pg_module_magic!();

#[derive(Debug)]
struct PgAstGrepError(String);

impl From<serde_json::Error> for PgAstGrepError {
    fn from(err: serde_json::Error) -> Self {
        PgAstGrepError(format!("JSON error: {}", err))
    }
}

impl From<PgAstGrepError> for Box<dyn std::error::Error> {
    fn from(err: PgAstGrepError) -> Self {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, err.0))
    }
}

#[pg_extern]
pub fn match_ast_rule(
    source_code: String,
    language: String,
    raw_rules: JsonB,
) -> Result<
    TableIterator<
        'static,
        (
            name!(match_text, String),
            name!(start_line, i32),
            name!(start_column, i32),
            name!(end_line, i32),
            name!(end_column, i32),
        ),
    >,
    Box<dyn std::error::Error>,
> {
    // Parse the language
    let lang = SupportLang::from_str(&language)
        .map_err(|e| PgAstGrepError(format!("Language error: {}", e)))?;
    let raw_rules: Vec<SerializableRuleConfig<SupportLang>> = serde_json::from_value(raw_rules.0)?;

    let mut configs = vec![];
    for rule in raw_rules {
        let config = RuleConfig::try_from(rule, &Default::default())
            .map_err(|e| PgAstGrepError(format!("Rule error: {}", e)))?;
        configs.push(config);
    }

    let combined = CombinedScan::new(configs.iter().collect());
    // Create document and parse
    let doc = StrDoc::new(&source_code, lang);
    let root = AstGrep::doc(doc);

    // Scan for matches
    let matches = combined.scan(&root, false).matches;

    // Convert matches to table rows
    let rows = matches
        .into_iter()
        .flat_map(|m| {
            m.1.into_iter()
                .map(|m| {
                    let node = m.get_node();
                    let text = m.text().to_string();
                    (
                        text,
                        node.start_pos().line() as i32,
                        node.start_pos().column(node) as i32,
                        node.end_pos().line() as i32,
                        node.end_pos().column(node) as i32,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    Ok(TableIterator::new(rows))
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use super::*;
    use serde_json;

    #[pg_test]
    fn test_match_ast_rule_typescript() {
        let source = r#"
            function test() {
                console.log("hello");
            }
        "#;

        let rule = serde_json::json!([
            {
                "id": "console-log",
                "language": "typescript",
                "rule": {
                    "pattern": "console.log($$$)"
                }
            }
        ]);

        println!("{}", serde_json::to_string(&rule).unwrap());

        let result = Spi::run(&format!(
            "SELECT match_text FROM match_ast_rule('{}', '{}', '{}'::jsonb)",
            source.replace("'", "''"),
            "typescript",
            serde_json::to_string(&rule).unwrap().replace("'", "''")
        ));

        assert!(result.is_ok());
    }
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
