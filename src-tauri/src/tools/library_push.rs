//! Library push: send a local advantage row → active Foundry world as a
//! world-level Item doc. Composes
//! `bridge::foundry::actions::storyteller::build_create_world_item`. No-op if
//! Foundry isn't connected (silent success — matches bridge_set_attribute
//! semantics; the UI gates the button on connectivity).

use crate::bridge::foundry::actions::storyteller::build_create_world_item;
use crate::bridge::types::SourceKind;
use crate::bridge::{commands::send_to_source_inner, BridgeConn};
use crate::shared::types::AdvantageKind;
use tauri::State;

fn kind_to_featuretype(k: AdvantageKind) -> &'static str {
    match k {
        AdvantageKind::Merit      => "merit",
        AdvantageKind::Flaw       => "flaw",
        AdvantageKind::Background => "background",
        AdvantageKind::Boon       => "boon",
    }
}

/// Look for `level` (preferred) or fall back to `min_level`. Anything not
/// found → 0. Mirrors the existing AdvantagesManager dot-display fallback.
///
/// Note: `NumberFieldValue` is `Single(f64) | Multi(Vec<f64>)` in this
/// codebase (the plan template referenced a `Range(min, _)` variant that
/// does not exist). For `Multi`, the smallest value is used — preserves
/// the plan's "min of a range" semantic.
fn extract_points(properties: &[crate::shared::types::Field]) -> i32 {
    for f in properties {
        if f.name == "level" || f.name == "min_level" {
            if let crate::shared::types::FieldValue::Number { value } = &f.value {
                match value {
                    crate::shared::types::NumberFieldValue::Single(n) => return *n as i32,
                    crate::shared::types::NumberFieldValue::Multi(vs) => {
                        // Empty Multi → 0; otherwise min value (preserves
                        // the plan's "min of a range" semantic).
                        let min = vs.iter().cloned().fold(f64::INFINITY, f64::min);
                        return if min.is_finite() { min as i32 } else { 0 };
                    }
                }
            }
        }
    }
    0
}

#[tauri::command]
pub async fn push_advantage_to_world(
    db: State<'_, crate::DbState>,
    conn: State<'_, BridgeConn>,
    id: i64,
) -> Result<(), String> {
    let all = crate::db::advantage::db_list(&db.0).await?;
    let row = all.iter().find(|r| r.id == id)
        .ok_or_else(|| format!("tools/library_push: advantage {id} not found"))?;

    let payload = build_create_world_item(
        &row.name,
        kind_to_featuretype(row.kind),
        &row.description,
        extract_points(&row.properties),
    )?;

    let text = serde_json::to_string(&payload)
        .map_err(|e| format!("tools/library_push: serialize: {e}"))?;
    send_to_source_inner(&conn.0, SourceKind::Foundry, text).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::{AdvantageKind, Field, FieldValue, NumberFieldValue};

    #[test]
    fn extract_points_reads_level_field() {
        let props = vec![Field {
            name: "level".into(),
            value: FieldValue::Number { value: NumberFieldValue::Single(3.0) },
        }];
        assert_eq!(extract_points(&props), 3);
    }

    #[test]
    fn extract_points_reads_min_level_for_ranged() {
        let props = vec![Field {
            name: "min_level".into(),
            value: FieldValue::Number { value: NumberFieldValue::Single(1.0) },
        }];
        assert_eq!(extract_points(&props), 1);
    }

    #[test]
    fn extract_points_returns_zero_when_absent() {
        assert_eq!(extract_points(&[]), 0);
    }

    #[test]
    fn kind_maps_to_featuretype_one_to_one() {
        assert_eq!(kind_to_featuretype(AdvantageKind::Merit),      "merit");
        assert_eq!(kind_to_featuretype(AdvantageKind::Flaw),       "flaw");
        assert_eq!(kind_to_featuretype(AdvantageKind::Background), "background");
        assert_eq!(kind_to_featuretype(AdvantageKind::Boon),       "boon");
    }
}
