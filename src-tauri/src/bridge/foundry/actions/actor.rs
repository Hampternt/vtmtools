// Foundry actor.* helper builders.
// See docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md.

use serde_json::{json, Value};

use crate::bridge::foundry::types::ApplyDyscrasiaPayload;

pub fn build_update_field(actor_id: &str, path: &str, value: Value) -> Value {
    json!({
        "type": "actor.update_field",
        "actor_id": actor_id,
        "path": path,
        "value": value,
    })
}

pub fn build_create_item_simple(actor_id: &str, item_type: &str, item_name: &str) -> Value {
    json!({
        "type": "actor.create_item_simple",
        "actor_id": actor_id,
        "item_type": item_type,
        "item_name": item_name,
        "replace_existing": true,
    })
}

pub fn build_apply_dyscrasia(actor_id: &str, payload: &str) -> Result<Value, String> {
    let payload: ApplyDyscrasiaPayload = serde_json::from_str(payload)
        .map_err(|e| format!("foundry/actor.apply_dyscrasia: invalid payload: {e}"))?;
    let merit_description_html =
        render_merit_description(&payload.description, &payload.bonus);
    let applied_at = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let notes_line = format!(
        "[{applied_at}] Acquired Dyscrasia: {} ({})",
        payload.dyscrasia_name, payload.resonance_type
    );
    Ok(json!({
        "type": "actor.apply_dyscrasia",
        "actor_id": actor_id,
        "dyscrasia_name": payload.dyscrasia_name,
        "resonance_type": payload.resonance_type,
        "merit_description_html": merit_description_html,
        "notes_line": notes_line,
        "replace_existing": true,
    }))
}

pub fn build_replace_private_notes(actor_id: &str, full_text: &str) -> Value {
    json!({
        "type": "actor.replace_private_notes",
        "actor_id": actor_id,
        "full_text": full_text,
    })
}

pub fn build_append_private_notes_line(actor_id: &str, line: &str) -> Value {
    json!({
        "type": "actor.append_private_notes_line",
        "actor_id": actor_id,
        "line": line,
    })
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
    fn update_field_shape() {
        let out = build_update_field("actor-xyz", "system.hunger.value", json!(3));
        assert_eq!(out["type"], "actor.update_field");
        assert_eq!(out["actor_id"], "actor-xyz");
        assert_eq!(out["path"], "system.hunger.value");
        assert_eq!(out["value"], 3);
    }

    #[test]
    fn create_item_simple_shape() {
        let out = build_create_item_simple("actor-xyz", "resonance", "Choleric");
        assert_eq!(out["type"], "actor.create_item_simple");
        assert_eq!(out["actor_id"], "actor-xyz");
        assert_eq!(out["item_type"], "resonance");
        assert_eq!(out["item_name"], "Choleric");
        assert_eq!(out["replace_existing"], true);
    }

    #[test]
    fn dyscrasia_happy_path_shape() {
        let payload = payload_json("Wax", "Choleric", "Crystallized blood.", "+1 Composure");
        let out = build_apply_dyscrasia("actor-abc", &payload).expect("happy path");
        assert_eq!(out["type"], "actor.apply_dyscrasia");
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
        assert!(re.is_match(line), "notes_line did not match: {line}");
    }

    #[test]
    fn dyscrasia_empty_bonus_omits_bonus_block() {
        let payload = payload_json("Custom", "Sanguine", "Some description.", "");
        let out = build_apply_dyscrasia("a", &payload).expect("empty bonus is valid");
        let html = out["merit_description_html"].as_str().unwrap();
        assert_eq!(html, "<p>Some description.</p>");
        assert!(!html.contains("Mechanical bonus"));
    }

    #[test]
    fn dyscrasia_html_escapes_dangerous_chars() {
        let payload = payload_json(
            "Test",
            "Phlegmatic",
            "<script>alert(\"x\")</script>",
            "& > <",
        );
        let out = build_apply_dyscrasia("a", &payload).expect("html-escape happy path");
        let html = out["merit_description_html"].as_str().unwrap();
        assert!(html.contains("&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;"));
        assert!(html.contains("&amp; &gt; &lt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn append_private_notes_line_shape() {
        let out = build_append_private_notes_line("actor-xyz", "Hello world");
        assert_eq!(out["type"], "actor.append_private_notes_line");
        assert_eq!(out["actor_id"], "actor-xyz");
        assert_eq!(out["line"], "Hello world");
    }

    #[test]
    fn replace_private_notes_shape() {
        let out = build_replace_private_notes("actor-xyz", "All new notes");
        assert_eq!(out["type"], "actor.replace_private_notes");
        assert_eq!(out["actor_id"], "actor-xyz");
        assert_eq!(out["full_text"], "All new notes");
    }

    #[test]
    fn dyscrasia_malformed_payload_returns_err() {
        let result = build_apply_dyscrasia("a", "{not valid json");
        assert!(result.is_err(), "malformed payload must return Err, not panic");
        let msg = result.unwrap_err();
        assert!(
            msg.starts_with("foundry/actor.apply_dyscrasia: invalid payload:"),
            "error message must use module-prefixed convention, got: {msg}"
        );
    }
}
