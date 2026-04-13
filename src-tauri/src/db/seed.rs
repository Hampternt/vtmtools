use sqlx::SqlitePool;

/// Replaces all built-in Dyscrasia entries with the canonical VTM 5e corebook set.
/// Custom entries (is_custom = 1) are never touched.
pub async fn seed_dyscrasias(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM dyscrasias WHERE is_custom = 0")
        .execute(pool)
        .await?;

    let entries: &[(&str, &str, &str, &str)] = &[
        // (resonance_type, name, description, bonus)

        // Phlegmatic
        ("Phlegmatic", "Chill",
         "Add two dice to pools to resist frenzy.",
         "+2 dice to resist frenzy"),
        ("Phlegmatic", "Comfortably Numb",
         "Feel no pain: receive no penalties or other negative effects from pain, both physical and social.",
         "Negate pain penalties"),
        ("Phlegmatic", "Eating Your Emotions",
         "Eat and digest food without becoming nauseous (or slaking Hunger, of course).",
         "Can eat and digest food"),
        ("Phlegmatic", "Given Up",
         "Next feeding on Phlegmatic blood slakes 1 additional Hunger; other blood slakes one fewer.",
         "+1 Hunger slaked from Phlegmatic blood"),
        ("Phlegmatic", "Lone Wolf",
         "Add one die to your tests when alone; subtract one die from tests to assist others or use teamwork. Lasts only one scene.",
         "+1 die alone / \u{2212}1 die teamwork (one scene)"),
        ("Phlegmatic", "Procrastinate",
         "Regain 1 point of Willpower if you delay something important by a day or more. Can only be used once per session.",
         "+1 Willpower for delaying (once per session)"),
        ("Phlegmatic", "Reflection",
         "Gain 1 free experience point toward Auspex or Dominate purchase.",
         "+1 XP toward Auspex or Dominate"),

        // Melancholy
        ("Melancholy", "In Mourning",
         "Add one die to Remorse tests.",
         "+1 die to Remorse tests"),
        ("Melancholy", "Lost Love",
         "Add one die to all pools to resist seduction attempts, including Presence.",
         "+1 die to resist seduction"),
        ("Melancholy", "Lost Relative",
         "Slake 1 additional Hunger when feeding from remaining members of your family.",
         "+1 Hunger slaked from family"),
        ("Melancholy", "Massive Failure",
         "Reroll tests reminiscent of the vessel's failure, excluding Hunger dice showing 1s.",
         "Reroll tests tied to vessel's failure"),
        ("Melancholy", "Nostalgic",
         "Add one die to rolls connecting to a specific nostalgic decade, art form, or social group.",
         "+1 die to nostalgic connections"),
        ("Melancholy", "Recalling",
         "Gain 1 free experience point toward Fortitude or Obfuscate purchase.",
         "+1 XP toward Fortitude or Obfuscate"),

        // Choleric
        ("Choleric", "Bully",
         "+1 damage against weaker foes or bullying targets in social and physical combat.",
         "+1 damage vs. weaker targets"),
        ("Choleric", "Cycle of Violence",
         "Next Choleric feeding slakes one additional Hunger; other blood slakes one fewer.",
         "+1 Hunger slaked from Choleric blood"),
        ("Choleric", "Envy",
         "+1 damage against superior foes (more attractive, talented, wealthy, or higher-status).",
         "+1 damage vs. superior targets"),
        ("Choleric", "Principled",
         "Reroll one roll against perceived ideological enemies, excluding Hunger dice showing 1s.",
         "Reroll vs. ideological enemies"),
        ("Choleric", "Vengeful",
         "Add two dice to one test against the type of target on which the vessel wished revenge.",
         "+2 dice to revenge tests"),
        ("Choleric", "Vicious",
         "Reroll Intimidation Skill rolls, excluding Hunger dice showing 1s.",
         "Reroll Intimidation rolls"),
        ("Choleric", "Driving",
         "Gain 1 free experience point toward Celerity or Potence purchase.",
         "+1 XP toward Celerity or Potence"),

        // Sanguine
        ("Sanguine", "Contagious Enthusiasm",
         "Add three dice to one test to convince a target via skin-to-skin contact.",
         "+3 dice to convince via touch (once)"),
        ("Sanguine", "Smell Game",
         "Add three dice to all rolls to detect other Sanguine vessels.",
         "+3 dice to detect Sanguine vessels"),
        ("Sanguine", "High on Life",
         "Use Blush of Life without making a Rouse Check.",
         "Blush of Life without Rouse Check"),
        ("Sanguine", "Manic High",
         "Add one die to all tests until failure; then subtract two dice.",
         "+1 die until failure, then \u{2212}2 dice"),
        ("Sanguine", "True Love",
         "Slake 1 additional Hunger when feeding from the vessel's true love.",
         "+1 Hunger slaked from true love"),
        ("Sanguine", "Stirring",
         "Gain 1 free experience point toward Blood Sorcery or Presence purchase.",
         "+1 XP toward Blood Sorcery or Presence"),
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
