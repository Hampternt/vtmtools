pub mod translate;
pub mod types;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::bridge::foundry::types::{ApplyDyscrasiaPayload, FoundryInbound};
use crate::bridge::source::BridgeSource;
use crate::bridge::types::CanonicalCharacter;

/// Stateless adapter for the FoundryVTT WoD5e module. Translates
/// Foundry actor data into the canonical bridge shape and builds
/// outbound messages the module knows how to apply via actor.update().
pub struct FoundrySource;

#[async_trait]
impl BridgeSource for FoundrySource {
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<CanonicalCharacter>, String> {
        let parsed: FoundryInbound = serde_json::from_value(msg).map_err(|e| e.to_string())?;
        let actors = match parsed {
            FoundryInbound::Actors { actors } => actors,
            FoundryInbound::ActorUpdate { actor } => vec![actor],
            FoundryInbound::Hello => return Ok(vec![]),
        };
        Ok(actors.iter().map(translate::to_canonical).collect())
    }

    fn build_set_attribute(
        &self,
        source_id: &str,
        name: &str,
        value: &str,
    ) -> Result<Value, String> {
        // Maps the canonical attribute name to a Foundry operation.
        // Most fields are simple actor.update() calls; resonance is special
        // because WoD5e stores it as an Item document, not a system field.
        // See docs/reference/foundry-vtm5e-paths.md.
        match name {
            "resonance" => Ok(json!({
                "type": "create_item",
                "actor_id": source_id,
                "item_type": "resonance",
                "item_name": value,
                "replace_existing": true,
            })),
            "dyscrasia" => {
                let payload: ApplyDyscrasiaPayload = serde_json::from_str(value)
                    .map_err(|e| format!("foundry/dyscrasia: invalid payload: {e}"))?;
                let merit_description_html =
                    render_merit_description(&payload.description, &payload.bonus);
                let applied_at = chrono::Local::now()
                    .format("%Y-%m-%d %H:%M")
                    .to_string();
                let notes_line = format!(
                    "[{applied_at}] Acquired Dyscrasia: {} ({})",
                    payload.dyscrasia_name, payload.resonance_type
                );
                Ok(json!({
                    "type": "apply_dyscrasia",
                    "actor_id": source_id,
                    "dyscrasia_name": payload.dyscrasia_name,
                    "resonance_type": payload.resonance_type,
                    "merit_description_html": merit_description_html,
                    "notes_line": notes_line,
                    "replace_existing": true,
                }))
            }
            _ => {
                let path = canonical_to_path(name);
                Ok(json!({
                    "type": "update_actor",
                    "actor_id": source_id,
                    "path": path,
                    "value": parse_value(value),
                }))
            }
        }
    }

    fn build_refresh(&self) -> Value {
        json!({ "type": "refresh" })
    }
}

fn canonical_to_path(name: &str) -> String {
    match name {
        "hunger" => "system.hunger.value",
        "humanity" => "system.humanity.value",
        "humanity_stains" => "system.humanity.stains",
        "blood_potency" => "system.blood.potency",
        "health_superficial" => "system.health.superficial",
        "health_aggravated" => "system.health.aggravated",
        "willpower_superficial" => "system.willpower.superficial",
        "willpower_aggravated" => "system.willpower.aggravated",
        // Pass-through for already-qualified paths (e.g. when a tool wants
        // to write a niche field the canonical mapping doesn't cover).
        other if other.starts_with("system.") => other,
        other => other,
    }
    .to_string()
}

fn parse_value(s: &str) -> Value {
    if let Ok(n) = s.parse::<i64>() {
        Value::from(n)
    } else if let Ok(f) = s.parse::<f64>() {
        Value::from(f)
    } else if s == "true" {
        Value::from(true)
    } else if s == "false" {
        Value::from(false)
    } else {
        Value::from(s)
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_merit_description(description: &str, bonus: &str) -> String {
    let desc_p = format!("<p>{}</p>", html_escape(description));
    if bonus.trim().is_empty() {
        desc_p
    } else {
        format!(
            "{desc_p}<p><em>Mechanical bonus:</em> {}</p>",
            html_escape(bonus)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn payload_json(name: &str, res: &str, desc: &str, bonus: &str) -> String {
        json!({
            "dyscrasia_name": name,
            "resonance_type": res,
            "description": desc,
            "bonus": bonus,
        })
        .to_string()
    }

    #[test]
    fn dyscrasia_happy_path_shape() {
        let src = FoundrySource;
        let payload = payload_json(
            "Wax",
            "Choleric",
            "Crystallized blood.",
            "+1 Composure",
        );
        let out = src
            .build_set_attribute("actor-abc", "dyscrasia", &payload)
            .expect("happy path returns Ok");
        assert_eq!(out["type"], "apply_dyscrasia");
        assert_eq!(out["actor_id"], "actor-abc");
        assert_eq!(out["dyscrasia_name"], "Wax");
        assert_eq!(out["resonance_type"], "Choleric");
        assert_eq!(out["replace_existing"], true);
        let html = out["merit_description_html"].as_str().unwrap();
        assert!(html.contains("<p>Crystallized blood.</p>"));
        assert!(html.contains("<p><em>Mechanical bonus:</em> +1 Composure</p>"));
        let line = out["notes_line"].as_str().unwrap();
        let re = regex::Regex::new(
            r"^\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}\] Acquired Dyscrasia: Wax \(Choleric\)$",
        )
        .unwrap();
        assert!(
            re.is_match(line),
            "notes_line did not match expected format: {line}"
        );
    }

    #[test]
    fn dyscrasia_empty_bonus_omits_bonus_block() {
        let src = FoundrySource;
        let payload = payload_json("Custom", "Sanguine", "Some description.", "");
        let out = src
            .build_set_attribute("a", "dyscrasia", &payload)
            .expect("empty bonus is valid");
        let html = out["merit_description_html"].as_str().unwrap();
        assert_eq!(html, "<p>Some description.</p>");
        assert!(!html.contains("Mechanical bonus"));
    }

    #[test]
    fn dyscrasia_html_escapes_dangerous_chars() {
        let src = FoundrySource;
        let payload = payload_json(
            "Test",
            "Phlegmatic",
            "<script>alert(\"x\")</script>",
            "& > <",
        );
        let out = src
            .build_set_attribute("a", "dyscrasia", &payload)
            .expect("html-escape happy path");
        let html = out["merit_description_html"].as_str().unwrap();
        assert!(html.contains("&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;"));
        assert!(html.contains("&amp; &gt; &lt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn dyscrasia_malformed_payload_returns_err() {
        let src = FoundrySource;
        let result = src.build_set_attribute("a", "dyscrasia", "{not valid json");
        assert!(result.is_err(), "malformed payload must return Err, not panic");
        let msg = result.unwrap_err();
        assert!(
            msg.starts_with("foundry/dyscrasia: invalid payload:"),
            "error message must use module-prefixed convention, got: {msg}"
        );
    }
}
