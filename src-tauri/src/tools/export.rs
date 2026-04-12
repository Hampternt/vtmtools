use serde_json::{Map, Value};
use std::path::PathBuf;
use tauri::Manager;

pub fn format_to_md(json: &Value) -> String {
    let mut md = String::new();
    md.push_str("# VTM Roll Result\n\n");

    if let Some(obj) = json.as_object() {
        for (key, val) in obj {
            let label = key
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i == 0 {
                        c.to_uppercase().next().unwrap_or(c)
                    } else {
                        c
                    }
                })
                .collect::<String>()
                .replace('_', " ");
            let value_str = match val {
                Value::String(s) => s.clone(),
                Value::Bool(b)   => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::Null      => "—".to_string(),
                _                => val.to_string(),
            };
            md.push_str(&format!("**{label}:** {value_str}\n\n"));
        }
    }

    md
}

#[tauri::command]
pub async fn export_result_to_md(
    app: tauri::AppHandle,
    result: Value,
    dyscrasia: Option<Value>,
) -> Result<String, String> {
    let now = chrono::Local::now();

    let mut map = Map::new();
    if let Some(obj) = result.as_object() {
        map.extend(obj.clone());
    }
    if let Some(d) = dyscrasia {
        map.insert("dyscrasia".to_string(), d);
    }
    map.insert("exported_at".to_string(), Value::String(now.to_rfc2822()));
    let combined = Value::Object(map);

    let md = format_to_md(&combined);

    let export_dir = app
        .path()
        .document_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("vtmtools");
    tokio::fs::create_dir_all(&export_dir).await.map_err(|e| e.to_string())?;

    let filename = format!("resonance_{}.md", now.format("%Y%m%d_%H%M%S"));
    let path = export_dir.join(&filename);
    tokio::fs::write(&path, &md).await.map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_to_md_includes_keys_and_values() {
        let input = json!({ "temperament": "Intense", "is_acute": true });
        let md = format_to_md(&input);
        assert!(md.contains("Intense"), "should contain temperament value");
        assert!(md.contains("true"), "should contain acute value");
        assert!(md.starts_with("# VTM Roll Result"), "should have header");
    }
}
