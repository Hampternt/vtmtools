# Apply Dyscrasia to Foundry Actor

**Date:** 2026-04-26
**Status:** Draft

## Summary

Add a Foundry-only "Apply Dyscrasia" action to the Resonance Roller. After the
GM rolls a resonance, confirms a dyscrasia in the Acute panel, and selects a
Foundry actor as target, one click pushes the dyscrasia to the actor's sheet
as a `feature` Item with `featuretype = "merit"` named `Dyscrasia: <name>`,
and appends a timestamped audit line to the actor's `system.privatenotes`.

Roll20 is intentionally out of scope. The bridge layer's
`BridgeSource` seam is for shared-where-shared-makes-sense features, not a
parity contract — Roll20 and Foundry have intrinsically different capability
ceilings (DOM-scrape extension vs. first-class Foundry Module).

Structurally this spec mirrors
[`2026-04-15-resonance-roll20-writeback-design.md`](2026-04-15-resonance-roll20-writeback-design.md):
same "frontend → bridge → VTT mutation" shape, smaller scope, different VTT.

---

## Data flow

```
Resonance.svelte
  (selectedChar.source === 'foundry' && confirmedDyscrasia !== null)
  → setAttribute('foundry', actorId, 'dyscrasia', JSON.stringify(payload))
    → Tauri: bridge_set_attribute(...) — unchanged generic handler
      → Rust: FoundrySource::build_set_attribute("dyscrasia", value)
              parses payload, stamps applied_at, returns wire JSON
        → wss://localhost:7424
          → Foundry module bridge.js handleInbound("apply_dyscrasia"):
              1. delete prior actor.items where
                 type=feature && system.featuretype=merit
                              && name.startsWith("Dyscrasia: ")
              2. createEmbeddedDocuments("Item", [{ type:"feature",
                 name:"Dyscrasia: <name>",
                 system:{featuretype:"merit", description:<HTML>, points:0} }])
              3. update("system.privatenotes",
                 existing trimmed empty ? line : existing + "\n" + line)
```

---

## §1 Wire protocol

The frontend's payload (carried inside `bridge_set_attribute`'s
`value: String` arg as JSON-encoded text):

```json
{
  "dyscrasia_name": "Wax",
  "resonance_type": "Choleric",
  "description": "Crystallized blood that …",
  "bonus": "+1 to Composure rolls"
}
```

The Foundry-bound wire shape `FoundrySource::build_set_attribute` emits
after parsing the payload, stamping the timestamp, and rendering the merit
description HTML:

```json
{
  "type": "apply_dyscrasia",
  "actor_id": "<foundry actor id>",
  "dyscrasia_name": "Wax",
  "resonance_type": "Choleric",
  "merit_description_html": "<p>Crystallized blood that …</p><p><em>Mechanical bonus:</em> +1 to Composure rolls</p>",
  "notes_line": "[2026-04-26 14:32] Acquired Dyscrasia: Wax (Choleric)",
  "replace_existing": true
}
```

Two pre-rendered string fields (`merit_description_html`, `notes_line`) keep
all formatting decisions on the Rust side, so the Foundry module stays a
thin DB-write executor — no schema-aware rendering in JS.

`replace_existing` is parameterized in the wire shape but hardcoded `true`
in v1. Carrying the field as data lets a future "additive" mode flip it
without protocol changes; cost is one extra JSON key.

### Timestamp stamping location

`applied_at` is stamped inside `FoundrySource::build_set_attribute`'s
`"dyscrasia"` match arm (the only function that already knows we're
processing a dyscrasia operation), then folded into the `notes_line` string.
The generic `bridge_set_attribute` Tauri handler stays oblivious to payload
semantics. Use `chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()` —
the same `chrono` dep already used in `src-tauri/src/tools/export.rs:43`.

---

## §2 Foundry module behavior

`vtmtools-bridge/scripts/bridge.js::handleInbound` gains an
`apply_dyscrasia` branch that performs **two operations sequentially**.
Best-effort, no rollback: V5 GM workflow tolerates partial state and the
operations are idempotent enough to re-apply or hand-fix.

### A) Replace-and-create the merit Item

```js
if (msg.type === "apply_dyscrasia") {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) return;

  // (1) delete prior dyscrasia merits
  const existing = actor.items.filter(
    (i) => i.type === "feature"
        && i.system?.featuretype === "merit"
        && typeof i.name === "string"
        && i.name.startsWith("Dyscrasia: ")
  );
  if (msg.replace_existing && existing.length) {
    await actor.deleteEmbeddedDocuments("Item", existing.map((i) => i.id));
  }

  // (2) create the new dyscrasia merit
  await actor.createEmbeddedDocuments("Item", [{
    type: "feature",
    name: `Dyscrasia: ${msg.dyscrasia_name}`,
    system: {
      featuretype: "merit",
      description: msg.merit_description_html,
      points: 0,
    },
  }]);

  // (3) append timestamped line to private notes
  const current = actor.system?.privatenotes ?? "";
  const next = current.trim() === "" ? msg.notes_line
                                     : `${current}\n${msg.notes_line}`;
  await actor.update({ "system.privatenotes": next });
  return;
}
```

### Load-bearing convention

The replace filter relies on the **`Dyscrasia: ` name prefix** as the sole
discriminator distinguishing tool-managed merits from player-authored ones.
A player-authored merit named e.g. `Dyscrasia: Wax (story note)` would
ALSO be clobbered on the next apply. This is a documented v1 limitation;
fixing it would require persisting a `flags.vtmtools.managed: true` marker
on each created Item and filtering on that flag instead. Out of scope.

### Same-dyscrasia re-apply

Clicking Apply with an unchanged dyscrasia/target combination produces a
delete-then-create on the merit (Foundry-internal `_id` changes) AND a new
duplicate timestamped line in privatenotes. This is **intentional** — the
notes line accumulates as an audit trail of every apply event. The brief
item churn is invisible to the GM.

---

## §3 Merit description HTML rendering

Built in Rust inside `FoundrySource::build_set_attribute` before going on
the wire. Output schema for `merit_description_html`:

```html
<p>{description_text_escaped}</p>
<p><em>Mechanical bonus:</em> {bonus_text_escaped}</p>
```

- HTML-escape both fields (`<`, `>`, `&`, `"`, `'`) so a custom dyscrasia
  containing markup characters can't break the sheet rendering.
- **If `bonus` is empty (after trim), omit the entire second `<p>` block.**
  Built-in dyscrasias always have a bonus, but custom dyscrasias may not —
  the merit should still render cleanly.
- If `description` is empty (after trim), still emit the empty `<p></p>` —
  the WoD5e merit sheet shows the description region whether or not there's
  text. Empty `<p></p>` is the cleanest "blank but present" rendering.

---

## §4 Backend (Rust) changes

### `src-tauri/src/bridge/source.rs`

Widen `build_set_attribute`'s return type so it can surface payload-parse
errors instead of panicking:

```rust
fn build_set_attribute(
    &self, source_id: &str, name: &str, value: &str,
) -> Result<Value, String>;
```

Roll20's existing impl trivially becomes `Ok(json!({...}))`. Foundry's
`"dyscrasia"` branch returns `Err("foundry/dyscrasia: invalid payload …")`
on JSON parse failure. ARCHITECTURE.md §7 forbids `unwrap()` in command
paths; this widening is the idiomatic fix.

### `src-tauri/src/bridge/commands.rs`

`bridge_set_attribute` updates one line: it now `?`s the
`build_set_attribute` result (instead of taking the bare `Value`), so a
source-side build error propagates out as the IPC promise rejection. Use
the same module-prefixed error convention already in use elsewhere in this
file. No signature change to the Tauri command itself.

### `src-tauri/src/bridge/foundry/types.rs`

Add the inbound-from-frontend payload type:

```rust
#[derive(Debug, Deserialize)]
pub struct ApplyDyscrasiaPayload {
    pub dyscrasia_name: String,
    pub resonance_type: String,
    pub description: String,
    pub bonus: String,
}
```

### `src-tauri/src/bridge/foundry/mod.rs`

Extend `build_set_attribute` with a `"dyscrasia"` branch:

```rust
"dyscrasia" => {
    let payload: ApplyDyscrasiaPayload =
        serde_json::from_str(value).map_err(|e|
            format!("foundry/dyscrasia: invalid payload: {e}"))?;
    let merit_description_html = render_merit_description(
        &payload.description, &payload.bonus);
    let applied_at = chrono::Local::now()
        .format("%Y-%m-%d %H:%M").to_string();
    let notes_line = format!(
        "[{applied_at}] Acquired Dyscrasia: {} ({})",
        payload.dyscrasia_name, payload.resonance_type);
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
```

Plus a private helper:

```rust
fn render_merit_description(description: &str, bonus: &str) -> String {
    let desc_p = format!("<p>{}</p>", html_escape(description));
    if bonus.trim().is_empty() {
        desc_p
    } else {
        format!("{desc_p}<p><em>Mechanical bonus:</em> {}</p>",
            html_escape(bonus))
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&#39;")
}
```

The other branches (`"resonance"`, the canonical-path passthrough)
become `Ok(json!({...}))`.

---

## §5 Frontend (Svelte / TS) changes

### `src/lib/components/ResultCard.svelte`

Add an outbound callback prop for confirmed-dyscrasia state:

```ts
let { result, onDyscrasiaConfirmChange }: {
    result: ResonanceRollResult;
    onDyscrasiaConfirmChange?: (d: DyscrasiaEntry | null) => void;
} = $props();
```

Fire it whenever `confirmedDyscrasia` changes (set on AcutePanel confirm,
cleared on result reset). No visual change to the card itself.

### `src/tools/Resonance.svelte`

Add state + handler:

```ts
let confirmedDyscrasia = $state<DyscrasiaEntry | null>(null);
let dyscrasiaApplyState = $state<'idle' | 'applying' | 'applied' | 'error'>('idle');

async function applyDyscrasia() {
    if (!confirmedDyscrasia || !selectedChar) return;
    if (selectedChar.source !== 'foundry') return; // hard guard
    dyscrasiaApplyState = 'applying';
    try {
        const payload = JSON.stringify({
            dyscrasia_name: confirmedDyscrasia.name,
            resonance_type: confirmedDyscrasia.resonanceType,
            description: confirmedDyscrasia.description,
            bonus: confirmedDyscrasia.bonus,
        });
        await setAttribute(selectedChar.source, selectedChar.source_id,
                           'dyscrasia', payload);
        dyscrasiaApplyState = 'applied';
        setTimeout(() => { dyscrasiaApplyState = 'idle'; }, 1800);
    } catch {
        dyscrasiaApplyState = 'error';
        setTimeout(() => { dyscrasiaApplyState = 'idle'; }, 1800);
    }
}
```

Wire the callback into ResultCard:

```svelte
<ResultCard
    {result}
    onDyscrasiaConfirmChange={(d) => { confirmedDyscrasia = d; }}
/>
```

Render a second apply button next to the existing resonance one,
visible only when **both** of these hold:

- `selectedChar?.source === 'foundry'`
- `confirmedDyscrasia !== null`

No separate `connected` gate is needed: `selectedChar` is non-null only
when its source is currently connected, because the `$effect` at
`Resonance.svelte:45-49` clears `selectedKey` whenever the corresponding
character disappears from `bridge.characters` (and characters disappear
on source disconnect). This matches the existing apply-resonance button,
which also gates only on `selectedChar` being set.

Label, mirroring the existing `applyState` pattern:

```
✓ Apply Dyscrasia to <selectedChar.name>     // idle
Applying…                                    // applying
✓ Applied                                    // applied
✗ Failed — retry                             // error
```

The button is hidden, not disabled, when the source isn't Foundry — Roll20
characters have no apply-dyscrasia path (and never will, per VTT-asymmetry
memory). Hiding rather than disabling avoids a confusing "why is this
greyed out" state.

### `src/lib/bridge/api.ts`

No signature change. Reuses the existing `setAttribute` wrapper. The
caller serializes the dyscrasia payload before passing.

---

## §6 What does not change

- `bridge_set_attribute` Tauri command signature — still
  `(source, source_id, name, value: String)`.
- The resonance apply path — already shipped, untouched.
- `DyscrasiaManager.svelte` — no apply affordance there yet (see §7).
- `Roll20Source::build_set_attribute` — gains no `"dyscrasia"` branch;
  if invoked with `name="dyscrasia"` it falls through to the generic
  `set_attribute` and the extension silently no-ops (no Roll20 sheet attr
  named `dyscrasia` exists). Not a regression because no UI ever calls it.
- `actorToWire` in `vtmtools-bridge/scripts/translate.js` — does NOT need
  to start sending `actor.items` or `system.privatenotes` back to the
  Tauri app. The Foundry module reads both locally on receipt for the
  filter and append. The Tauri-side `CanonicalCharacter` cache will
  therefore never observe the merit it just created or the appended
  notes line — that's fine; nothing in vtmtools currently surfaces those
  fields.
- `CanonicalCharacter` schema in `src-tauri/src/bridge/types.rs` — no
  fields added.

---

## §7 Out of scope

- Roll20 dyscrasia application. Acceptable per VTT-asymmetry posture.
- Apply from `DyscrasiaManager` (cherry-pick a non-rolled dyscrasia from
  the catalog and push to a chosen Foundry actor).
- Bulk apply to multiple characters.
- Apply-history UI in vtmtools (the timestamped privatenotes line IS
  the history).
- Editing the merit's `points`, `source.book/page`, or `bonuses` array
  after creation. Created with `points: 0` and unset book/page; the GM can
  hand-edit on the sheet if they want.
- Atomic rollback if item-create succeeds but notes-append fails.
  Best-effort; rare; manually fixable. Same posture as the resonance
  apply path.
- Surfacing apply success via toast or notification. Pre-existing pattern
  in this codebase is silent success on bridge writes; keep it.
- "Disconnected source silent-success" cleanup — pre-existing across all
  bridge writes; not introduced by this spec, not fixed by this spec.
- Persisting a `flags.vtmtools.managed: true` marker on created merits to
  make the replace filter robust against a player authoring a similarly
  named merit. The name-prefix convention is good enough for v1.

---

## §8 Test plan

### Rust unit tests in `src-tauri/src/bridge/foundry/mod.rs`

- **Payload happy path:** given a JSON-encoded `ApplyDyscrasiaPayload` as
  the `value` arg, `build_set_attribute("dyscrasia", "...")` returns
  `Ok(value)` whose object has all expected fields, with `notes_line`
  matching the regex
  `^\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}\] Acquired Dyscrasia: .+ \(.+\)$`.
- **Empty bonus:** `merit_description_html` omits the
  `<p><em>Mechanical bonus:</em> …</p>` block entirely; the description
  `<p>` still appears.
- **HTML-escape:** description containing `<script>alert("x")</script>`
  produces `&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;` in the
  rendered HTML.
- **Malformed payload:** invalid JSON in `value` returns
  `Err("foundry/dyscrasia: invalid payload: …")` — does NOT panic.
- **Roll20 wrapper still compiles:** `Roll20Source::build_set_attribute`
  builds the same legacy shape, now wrapped in `Ok(...)`.

### Manual end-to-end against a live WoD5e world

(No automated test possible — same posture as the existing resonance
apply path.)

1. Connect Foundry via the module; pick a vampire actor in the Resonance
   Roller's target strip.
2. Roll resonance → Intense → confirm a built-in dyscrasia in the Acute
   panel → click "Apply Dyscrasia to <actor>".
3. Verify in Foundry: actor's Features tab shows new `Dyscrasia: <name>`
   merit, description renders both paragraphs.
4. Verify in Foundry: actor's private notes (sheet → biography → private
   notes, or whatever WoD5e's UI calls it) shows the timestamped line.
5. Apply a second different dyscrasia → first merit deleted, second
   created; notes file gets a second line on its own row, first line
   preserved.
6. Pre-populate `system.privatenotes` with multi-line GM notes →
   apply → user text preserved verbatim, new line appended at bottom.
7. Apply with a custom dyscrasia whose `bonus` is empty → merit description
   renders only the description paragraph, no "Mechanical bonus" row.
8. Apply with a custom dyscrasia whose `description` contains `<` and
   `&` → the merit displays the literal characters (escaped), no markup
   leaks into the sheet.
9. Roll20 character selected in the strip → apply-dyscrasia button is not
   rendered at all (verify by inspecting DOM, not just clicking).

### Verification gate

`./scripts/verify.sh` green (typecheck, cargo test, frontend build) before
commit. Per ARCHITECTURE.md §10.
