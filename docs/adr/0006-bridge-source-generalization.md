# 0006: Generalize Roll20 bridge into a multi-source `bridge/` layer

**Status:** accepted
**Date:** 2026-04-26

## Context

[ADR 0005](0005-roll20-ws-extension-bridge.md) introduced a Roll20-specific
WebSocket bridge in `src-tauri/src/roll20/` to ingest live character data
from a Chrome extension running on Roll20 pages. As of early 2026, several
forces argued for adding a second VTT integration (FoundryVTT, hosted on
self-hosted servers and on Forge/Molten), and the existing structure had
"Roll20" hardcoded into module names, command names, event namespaces,
state types, and frontend type names — every consumer would need updating
twice if Foundry shipped as a parallel sibling.

Three architecture options were on the table:
- **A.** Stand up a parallel `foundry/` module beside `roll20/`. Smallest
  diff, ugliest long-term — every future VTT pays the same copy/paste cost.
- **B.** Keep the existing `:7423` socket, add a `source` field to
  inbound/outbound JSON, multiplex Roll20 + Foundry on the same listener.
  Modest refactor, but couples the wire protocol shape to Roll20's existing
  shape.
- **C.** Generalize to a `bridge/` layer with a `BridgeSource` trait,
  per-source impls, and port-based routing. Largest refactor, cleanest seam
  for any future VTT.

A separate, load-bearing constraint surfaced during research: browsers
block `ws://localhost` from HTTPS-served pages as mixed content. Roll20's
extension bypasses this because content scripts dial localhost from
extension context (separate origin, exempt from page CSP). Foundry
modules dial from page context — so HTTPS-hosted Foundry instances
(Forge, Molten, any TLS-proxied self-host) cannot reach `ws://`. A
`wss://` listener with a self-signed cert is the only path that works
across hosting models.

A third decision concerned the canonical character schema. Options:
- **i.** Normalize at the bridge into a fixed `Character` struct.
  Frontend sees one shape, but source-specific extras (Roll20 disciplines
  via repeating sections, Foundry items, etc.) get squashed.
- **ii.** Keep raw + tag the source. Frontend reads per-source, no
  information loss, but every consumer pays the per-source switch tax.
- **iii.** Hybrid — canonical fields where they apply universally, plus a
  source-specific `raw` blob for everything else.

## Decision

**Option C + iii**, plus a self-signed wss listener for Foundry:

- Rename `src-tauri/src/roll20/` → `src-tauri/src/bridge/roll20/`. Roll20
  becomes the first `BridgeSource` impl, not a top-level module.
- Add `BridgeSource` trait at `src-tauri/src/bridge/source.rs`. Methods:
  `handle_inbound(msg) -> Vec<CanonicalCharacter>`,
  `build_set_attribute(source_id, name, value) -> Value`,
  `build_refresh() -> Value`. Sources are stateless; shared state
  (characters cache, per-source connection info, outbound channel) lives
  in `BridgeState`.
- Two accept loops, each tied to a fixed source by port — no in-message
  source tag needed:
  - `ws://127.0.0.1:7423` → Roll20. Existing extension protocol unchanged;
    the extension does not need updating.
  - `wss://127.0.0.1:7424` → Foundry. TLS via a self-signed `localhost`
    cert generated on first launch by `rcgen`, persisted in the Tauri app
    data dir, served via `tokio-rustls`.
- Hybrid canonical character schema in `src-tauri/src/bridge/types.rs`:
  ```rust
  struct CanonicalCharacter {
      source: SourceKind,           // Roll20 | Foundry
      source_id: String,
      name: String,
      controlled_by: Option<String>,
      hunger: Option<u8>,
      health: Option<HealthTrack>,
      willpower: Option<HealthTrack>,
      humanity: Option<u8>,
      humanity_stains: Option<u8>,
      blood_potency: Option<u8>,
      raw: serde_json::Value,       // source-specific extras
  }
  ```
  Per-source translators (`bridge/{roll20,foundry}/translate.rs`) populate
  the canonical fields and stuff the source-specific blob (Roll20 attribute
  list / Foundry `actor.system`) into `raw`.
- Replace five `roll20_*` Tauri commands with four `bridge_*`:
  `bridge_get_status`, `bridge_get_characters`, `bridge_set_attribute`,
  `bridge_refresh`. The `set_attribute` `name` argument is opaque to the
  frontend; each source impl translates it into source-specific operations
  (Foundry's `resonance` becomes an Item creation rather than a field
  update, for example).
- Frontend events go from `roll20://*` to a `bridge://*` namespace with
  per-source sub-paths: `bridge://<source>/connected`,
  `bridge://<source>/disconnected`, plus a single
  `bridge://characters-updated` carrying the merged canonical list.
- A new Foundry module package at `vtmtools-bridge/` (manifest URL
  installable, sideloadable as that exact directory name) runs in the GM
  browser only, dials `wss://localhost:7424`, hooks `updateActor` /
  `createActor` / `deleteActor` and pushes canonical actor blobs.

## Consequences

- **Adding a future VTT is a four-file diff:** new `bridge/<vtt>/{mod.rs,
  types.rs, translate.rs}` plus one `sources.insert(...)` line in
  `lib.rs`. No protocol, frontend, or command surface changes needed.
- **Roll20 remains exempt from cert acceptance** — port `:7423` stays plain
  `ws://`, and the existing extension's wire protocol is preserved
  byte-for-byte. The Roll20 path has no observable behavior change.
- **Foundry users accept the localhost cert once per browser** by visiting
  `https://localhost:7424` and clicking through the warning. This is a
  documented one-time UX step in the Foundry module's README.
- **TLS init failure must NOT silently degrade.** A failed cert generation
  produces a clear "Foundry connections disabled this session" log and
  the `:7424` listener is not spawned. Falling back to plain `ws://7424`
  would produce mystery cert errors in the Foundry browser since the
  module dials `wss://`.
- **Two listeners, both localhost.** §8 Network surface now declares
  exactly two listeners (`127.0.0.1:7423` plain and `127.0.0.1:7424` wss),
  both bound to loopback. Neither accepts external traffic.
- **Frontend complexity is bounded** by the canonical fields. Tools that
  need source-specific data (Resonance reading a Roll20 character's
  `resonance` attribute, Campaign reading Roll20 disciplines) cast
  `char.raw` to a per-source helper type. Foundry-side equivalents come
  in as canonical-only until / unless those features are explicitly added.
- **Stale-on-delete:** the merged characters map is insert-only — deleted
  actors stay in vtmtools' view until the next refresh. Same behavior as
  the original Roll20 path. A future "remove" wire message would fix it.
- **ADR 0005 is superseded.** Its core decisions (localhost-only,
  no API key, browser-extension data path for Roll20) are preserved
  inside the new structure; only the layering changes.

## Alternatives considered

- **Option A — parallel `foundry/` module beside `roll20/`.** Rejected.
  Adds a third pipeline for the next VTT and gives no extensibility seam
  beyond duplication. The user's stated preference for "future-proof,
  pluggable" wiring rules this out.
- **Option B — single port with in-message source tag.** Rejected. The
  existing Roll20 extension's wire protocol does NOT carry a source
  field; updating the extension to add one is gratuitous and breaks
  back-compat for any Roll20 user who upgrades the desktop app before
  the extension. Port-based routing achieves the same multiplex without
  protocol changes.
- **Schema option (i) — normalize-only.** Rejected. Loses
  Roll20-specific data (disciplines via repeating sections, dynamic
  attributes the canonical model doesn't enumerate), forcing those
  consumers into special-case backend paths.
- **Schema option (ii) — raw + source tag, no canonical.** Rejected.
  Pushes per-source switching into every frontend consumer; the
  Resonance and Campaign tools both want hunger/health by canonical
  name, not by digging into raw blobs.
- **Drop TLS, require HTTP-only self-hosted Foundry.** Rejected. Cuts off
  every hosted-Foundry user (Forge, Molten, anyone behind a reverse
  proxy with SSL termination). Self-signed cert UX is a one-time pain
  by comparison.
- **OS-trust-store-installed cert instead of browser-accept-once.**
  Rejected. Requires elevated permissions per OS, opaque uninstall,
  and ongoing maintenance for cert rotation. The browser-accept-once
  model is per-browser and self-contained.
- **Rename `roll20_*` Tauri commands but keep them alongside
  `bridge_*` for a deprecation window.** Rejected. The frontend was
  ported in the same change set; carrying duplicate commands adds
  surface area for a deprecation cycle that doesn't apply (the tool is
  single-user, single-machine, with no external consumers depending
  on the IPC names).
