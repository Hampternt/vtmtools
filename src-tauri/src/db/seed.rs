use sqlx::SqlitePool;

/// Inserts canonical Dyscrasia entries if the table is empty.
/// NOTE: Verify these entries against the VTM 5e Corebook before shipping.
pub async fn seed_dyscrasias(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dyscrasias WHERE is_custom = 0")
        .fetch_one(pool)
        .await?;

    if count.0 > 0 {
        return Ok(()); // already seeded
    }

    let entries: &[(&str, &str, &str, &str)] = &[
        // (resonance_type, name, description, bonus)
        // Phlegmatic — associated with Fortitude and Auspex
        ("Phlegmatic", "Unshakeable Calm",
         "The vessel exists in a state of profound emotional equilibrium, unmoved by fear or chaos.",
         "+2 dice to Fortitude-related rolls"),
        ("Phlegmatic", "Still Waters",
         "The vessel's thoughts run deep and slow, their aura almost impossible to read.",
         "+2 dice to Auspex-related discipline rolls"),
        // Melancholy — associated with Oblivion and Obfuscate
        ("Melancholy", "Haunted",
         "The vessel is touched by loss so deep it leaves a stain on the soul.",
         "+2 dice to Oblivion-related discipline rolls"),
        ("Melancholy", "Hollow",
         "The vessel moves through the world like a ghost, barely present.",
         "+2 dice to Obfuscate-related discipline rolls"),
        // Choleric — associated with Celerity and Potence
        ("Choleric", "Berserker's Blood",
         "The vessel's rage is so pure it feels like a living thing in the veins.",
         "+2 dice to Potence-related discipline rolls"),
        ("Choleric", "Hair-Trigger",
         "Violence lives in the vessel's reflexes; they move before they think.",
         "+2 dice to Celerity-related discipline rolls"),
        // Sanguine — associated with Presence and Blood Sorcery
        ("Sanguine", "True Believer",
         "The vessel's faith or passion is so absolute it warps the blood.",
         "+2 dice to Presence-related discipline rolls"),
        ("Sanguine", "Ecstatic",
         "The vessel exists in a heightened state of bliss that makes their blood almost luminous.",
         "+2 dice to Blood Sorcery-related discipline rolls"),
    ];

    for (resonance_type, name, description, bonus) in entries {
        sqlx::query(
            "INSERT INTO dyscrasias (resonance_type, name, description, bonus, is_custom)
             VALUES (?, ?, ?, ?, 0)"
        )
        .bind(resonance_type)
        .bind(name)
        .bind(description)
        .bind(bonus)
        .execute(pool)
        .await?;
    }

    Ok(())
}
