use sqlx::{Row, SqlitePool};
use crate::shared::modifier::{
    ModifierEffect, NewStatusTemplate, StatusTemplate, StatusTemplatePatch,
};

fn row_to_template(r: &sqlx::sqlite::SqliteRow) -> Result<StatusTemplate, String> {
    let effects_json: String = r.get("effects_json");
    let effects: Vec<ModifierEffect> = serde_json::from_str(&effects_json)
        .map_err(|e| format!("db/status_template.list: effects deserialize: {e}"))?;
    let tags_json: String = r.get("tags_json");
    let tags: Vec<String> = serde_json::from_str(&tags_json)
        .map_err(|e| format!("db/status_template.list: tags deserialize: {e}"))?;
    // Item Builder Shell columns (0010). try_get + ok().flatten() tolerates
    // older test schemas that pre-date the columns.
    let wip_int: i64 = r.try_get("wip").unwrap_or(0);
    let confirmed_fields_text: Option<String> =
        r.try_get("confirmed_fields").ok().flatten();
    let confirmed_fields = confirmed_fields_text
        .as_deref()
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok());
    Ok(StatusTemplate {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get("description"),
        effects,
        tags,
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
        wip: wip_int != 0,
        confirmed_fields,
    })
}

pub(crate) async fn db_list(pool: &SqlitePool) -> Result<Vec<StatusTemplate>, String> {
    let rows = sqlx::query(
        "SELECT id, name, description, effects_json, tags_json, created_at, updated_at,
                wip, confirmed_fields
         FROM status_templates ORDER BY id ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/status_template.list: {e}"))?;
    rows.iter().map(row_to_template).collect()
}

pub(crate) async fn db_get(pool: &SqlitePool, id: i64) -> Result<StatusTemplate, String> {
    let row = sqlx::query(
        "SELECT id, name, description, effects_json, tags_json, created_at, updated_at,
                wip, confirmed_fields
         FROM status_templates WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db/status_template.get: {e}"))?
    .ok_or_else(|| "db/status_template.get: not found".to_string())?;
    row_to_template(&row)
}

#[tauri::command]
pub async fn list_status_templates(
    pool: tauri::State<'_, crate::DbState>,
) -> Result<Vec<StatusTemplate>, String> {
    db_list(&pool.0).await
}

pub(crate) async fn db_add(
    pool: &SqlitePool,
    input: NewStatusTemplate,
) -> Result<StatusTemplate, String> {
    // Empty name is rejected for non-WIP saves. Save WIP (wip=true) bypasses
    // this guard so an in-progress draft can be persisted before the name
    // stage has been filled in. The frontend's bodyValid flag drives the
    // Save vs Save WIP label flip and sets `wip` accordingly.
    if !input.wip && input.name.is_empty() {
        return Err("db/status_template.add: empty name".to_string());
    }
    let effects_json = serde_json::to_string(&input.effects)
        .map_err(|e| format!("db/status_template.add: serialize effects: {e}"))?;
    let tags_json = serde_json::to_string(&input.tags)
        .map_err(|e| format!("db/status_template.add: serialize tags: {e}"))?;
    let confirmed_fields_str: Option<String> = match input.confirmed_fields.as_ref() {
        Some(v) => Some(
            serde_json::to_string(v)
                .map_err(|e| format!("db/status_template.add: serialize confirmed_fields: {e}"))?,
        ),
        None => None,
    };

    let result = sqlx::query(
        "INSERT INTO status_templates
            (name, description, effects_json, tags_json, wip, confirmed_fields)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(&effects_json)
    .bind(&tags_json)
    .bind(input.wip as i64)
    .bind(&confirmed_fields_str)
    .execute(pool)
    .await
    .map_err(|e| format!("db/status_template.add: {e}"))?;
    db_get(pool, result.last_insert_rowid()).await
}

pub(crate) async fn db_update(
    pool: &SqlitePool,
    id: i64,
    patch: StatusTemplatePatch,
) -> Result<StatusTemplate, String> {
    let mut current = db_get(pool, id).await
        .map_err(|e| if e.contains("not found") { "db/status_template.update: not found".to_string() } else { format!("db/status_template.update: {e}") })?;

    if let Some(name) = patch.name             { current.name = name; }
    if let Some(desc) = patch.description      { current.description = desc; }
    if let Some(effects) = patch.effects       { current.effects = effects; }
    if let Some(tags) = patch.tags             { current.tags = tags; }

    // Item Builder Shell: `wip` and `confirmed_fields` are REPLACED on every
    // IBS save (no patch-or-skip semantics). When `patch.wip` is None the
    // caller is a legacy non-IBS path; leave both columns alone.
    //
    // When `patch.wip` is Some, the IBS authoritatively describes the
    // current draft state — `wip` is overwritten with the new value and
    // `confirmed_fields` is overwritten with the patch's value (which may
    // be None to clear the column on a "finish the draft" save).
    if let Some(wip) = patch.wip {
        current.wip = wip;
        current.confirmed_fields = patch.confirmed_fields.clone();
    }

    // Guard against promoting a draft to a completed save when the name is
    // still empty. Apply on the MERGED state so a `wip: false`-only patch
    // against an empty-named draft is rejected here (not on the original
    // pre-patch name).
    if !current.wip && current.name.is_empty() {
        return Err("db/status_template.update: empty name".to_string());
    }

    let effects_json = serde_json::to_string(&current.effects)
        .map_err(|e| format!("db/status_template.update: serialize effects: {e}"))?;
    let tags_json = serde_json::to_string(&current.tags)
        .map_err(|e| format!("db/status_template.update: serialize tags: {e}"))?;
    let confirmed_fields_str: Option<String> = match current.confirmed_fields.as_ref() {
        Some(v) => Some(
            serde_json::to_string(v)
                .map_err(|e| format!("db/status_template.update: serialize confirmed_fields: {e}"))?,
        ),
        None => None,
    };

    let result = sqlx::query(
        "UPDATE status_templates
         SET name = ?, description = ?, effects_json = ?, tags_json = ?,
             wip = ?, confirmed_fields = ?,
             updated_at = datetime('now')
         WHERE id = ?"
    )
    .bind(&current.name)
    .bind(&current.description)
    .bind(&effects_json)
    .bind(&tags_json)
    .bind(current.wip as i64)
    .bind(&confirmed_fields_str)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/status_template.update: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/status_template.update: not found".to_string());
    }
    db_get(pool, id).await
}

pub(crate) async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    let result = sqlx::query("DELETE FROM status_templates WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/status_template.delete: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/status_template.delete: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn add_status_template(
    pool: tauri::State<'_, crate::DbState>,
    input: NewStatusTemplate,
) -> Result<StatusTemplate, String> {
    db_add(&pool.0, input).await
}

#[tauri::command]
pub async fn update_status_template(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    patch: StatusTemplatePatch,
) -> Result<StatusTemplate, String> {
    db_update(&pool.0, id, patch).await
}

#[tauri::command]
pub async fn delete_status_template(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::modifier::ModifierKind;
    use sqlx::SqlitePool;

    async fn fresh_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn list_empty_returns_empty_vec() {
        let pool = fresh_pool().await;
        let result = db_list(&pool).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn round_trip_preserves_effects_and_tags() {
        let pool = fresh_pool().await;
        sqlx::query(
            "INSERT INTO status_templates (name, description, effects_json, tags_json)
             VALUES ('Slippery', 'Hard to grapple',
                     '[{\"kind\":\"difficulty\",\"scope\":\"grapple\",\"delta\":2,\"note\":null}]',
                     '[\"Combat\",\"Physical\"]')"
        )
        .execute(&pool).await.unwrap();
        let list = db_list(&pool).await.unwrap();
        assert_eq!(list.len(), 1);
        let t = &list[0];
        assert_eq!(t.name, "Slippery");
        assert_eq!(t.effects.len(), 1);
        assert_eq!(t.effects[0].kind, ModifierKind::Difficulty);
        assert_eq!(t.tags, vec!["Combat".to_string(), "Physical".to_string()]);
    }

    fn sample_new() -> NewStatusTemplate {
        NewStatusTemplate {
            name: "Slippery".to_string(),
            description: "Hard to grapple".to_string(),
            effects: vec![ModifierEffect {
                kind: ModifierKind::Difficulty,
                scope: Some("grapple".to_string()),
                delta: Some(2),
                note: None,
                paths: vec![],
                bonus_name: None,
            }],
            tags: vec!["Combat".to_string()],
            wip: false,
            confirmed_fields: None,
        }
    }

    #[tokio::test]
    async fn add_inserts_and_returns_full_record() {
        let pool = fresh_pool().await;
        let t = db_add(&pool, sample_new()).await.unwrap();
        assert!(t.id > 0);
        assert_eq!(t.name, "Slippery");
        assert_eq!(t.effects.len(), 1);
        assert_eq!(t.tags, vec!["Combat".to_string()]);
    }

    #[tokio::test]
    async fn add_accepts_empty_name_for_wip() {
        // Empty name is a valid WIP-state value; Save WIP must always succeed.
        // Frontend's bodyValid flag (not the backend) drives the Save vs
        // Save WIP label flip, and now sends wip=true explicitly.
        let pool = fresh_pool().await;
        let mut new = sample_new();
        new.name = String::new();
        new.wip = true;
        new.confirmed_fields = Some(vec!["effects".to_string()]);
        let t = db_add(&pool, new).await.unwrap();
        assert!(t.id > 0);
        assert_eq!(t.name, "");
        assert!(t.wip);
        assert_eq!(t.confirmed_fields.as_deref(), Some(["effects".to_string()].as_slice()));
    }

    #[tokio::test]
    async fn add_rejects_empty_name_when_not_wip() {
        // Complement to add_accepts_empty_name_for_wip: a non-WIP save with
        // an empty name is a developer bug — bodyValid would prevent it on
        // the frontend, but the backend must guard regardless.
        let pool = fresh_pool().await;
        let mut new = sample_new();
        new.name = String::new();
        // new.wip defaults to false.
        let err = db_add(&pool, new).await.unwrap_err();
        assert!(err.contains("empty name"), "got: {err}");
    }

    #[tokio::test]
    async fn update_accepts_empty_name_for_wip() {
        // Save WIP path through update: explicit wip=true patch keeps the
        // empty-name guard quiet.
        let pool = fresh_pool().await;
        let t = db_add(&pool, sample_new()).await.unwrap();
        let updated = db_update(&pool, t.id, StatusTemplatePatch {
            name: Some(String::new()),
            description: None,
            effects: None,
            tags: None,
            wip: Some(true),
            confirmed_fields: Some(vec!["effects".to_string()]),
        }).await.unwrap();
        assert_eq!(updated.name, "");
        assert!(updated.wip);
        assert_eq!(updated.confirmed_fields.as_deref(), Some(["effects".to_string()].as_slice()));
    }

    #[tokio::test]
    async fn update_rejects_promoting_draft_to_complete_with_empty_name() {
        // Save WIP an empty-name draft, then attempt to "finish" it by
        // sending wip=false without supplying a name — the merged-state
        // guard must reject so we never end up with a non-WIP empty-name
        // row.
        let pool = fresh_pool().await;
        let mut new = sample_new();
        new.name = String::new();
        new.wip = true;
        let t = db_add(&pool, new).await.unwrap();
        let err = db_update(&pool, t.id, StatusTemplatePatch {
            name: None,
            description: None,
            effects: None,
            tags: None,
            wip: Some(false),
            confirmed_fields: None,
        }).await.unwrap_err();
        assert!(err.contains("empty name"), "got: {err}");
    }

    #[tokio::test]
    async fn update_clears_wip_and_confirmed_fields_on_finish_save() {
        // Promote a WIP draft to a finished row: confirmed_fields must
        // clear to NULL (REPLACE, not COALESCE).
        let pool = fresh_pool().await;
        let mut new = sample_new();
        new.wip = true;
        new.confirmed_fields = Some(vec!["effects".to_string()]);
        let t = db_add(&pool, new).await.unwrap();
        assert!(t.wip);

        let updated = db_update(&pool, t.id, StatusTemplatePatch {
            name: None, description: None, effects: None, tags: None,
            wip: Some(false),
            confirmed_fields: None,
        }).await.unwrap();
        assert!(!updated.wip);
        assert!(updated.confirmed_fields.is_none(),
            "finish save with confirmedFields omitted must clear the column");
    }

    #[tokio::test]
    async fn legacy_update_without_wip_field_leaves_columns_alone() {
        // A non-IBS caller (or unit-test partial patch) that omits `wip`
        // must NOT clobber the stored wip / confirmed_fields columns.
        let pool = fresh_pool().await;
        let mut new = sample_new();
        new.wip = true;
        new.confirmed_fields = Some(vec!["effects".to_string()]);
        let t = db_add(&pool, new).await.unwrap();

        let updated = db_update(&pool, t.id, StatusTemplatePatch {
            name: None,
            description: Some("only changing description".to_string()),
            effects: None, tags: None,
            wip: None,
            confirmed_fields: None,
        }).await.unwrap();
        assert!(updated.wip, "wip must survive a wip-less patch");
        assert_eq!(
            updated.confirmed_fields.as_deref(),
            Some(["effects".to_string()].as_slice()),
            "confirmed_fields must survive a wip-less patch",
        );
        assert_eq!(updated.description, "only changing description");
    }

    #[tokio::test]
    async fn update_applies_partial_patch() {
        let pool = fresh_pool().await;
        let t = db_add(&pool, sample_new()).await.unwrap();
        let updated = db_update(&pool, t.id, StatusTemplatePatch {
            name: Some("Renamed".into()), description: None, effects: None, tags: None,
            wip: None, confirmed_fields: None,
        }).await.unwrap();
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.effects.len(), 1); // untouched
    }

    #[tokio::test]
    async fn update_missing_id_errors() {
        let pool = fresh_pool().await;
        let err = db_update(&pool, 9999, StatusTemplatePatch {
            name: Some("X".into()), description: None, effects: None, tags: None,
            wip: None, confirmed_fields: None,
        }).await.unwrap_err();
        assert!(err.contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn delete_removes_row() {
        let pool = fresh_pool().await;
        let t = db_add(&pool, sample_new()).await.unwrap();
        db_delete(&pool, t.id).await.unwrap();
        assert!(db_list(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_missing_id_errors() {
        let pool = fresh_pool().await;
        let err = db_delete(&pool, 9999).await.unwrap_err();
        assert!(err.contains("not found"), "got: {err}");
    }
}
