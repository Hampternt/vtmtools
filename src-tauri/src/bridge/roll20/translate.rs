// Roll20 (Jumpgate VTM5e sheet) attrs → CanonicalCharacter.
//
// Attribute name conventions match the existing Campaign.svelte ATTR map and
// the Backbone names on Jumpgate sheets. The `raw` field carries the full
// Roll20 character struct so source-specific consumers (parseDisciplines etc.)
// can still read every attribute downstream.

use crate::bridge::roll20::types::{Attribute, Character};
use crate::bridge::types::{CanonicalCharacter, HealthTrack, SourceKind};

pub fn to_canonical(raw: &Character) -> CanonicalCharacter {
    let attrs = AttrLookup::new(&raw.attributes);
    CanonicalCharacter {
        source: SourceKind::Roll20,
        source_id: raw.id.clone(),
        name: raw.name.clone(),
        controlled_by: if raw.controlled_by.is_empty() {
            None
        } else {
            Some(raw.controlled_by.clone())
        },
        hunger: attrs.parse_u8("hunger"),
        health: build_health_track(&attrs, "health"),
        willpower: build_health_track(&attrs, "willpower"),
        humanity: attrs.parse_u8("humanity"),
        humanity_stains: attrs.parse_u8("humanity_stains"),
        blood_potency: attrs.parse_u8("blood_potency"),
        raw: serde_json::to_value(raw).unwrap_or(serde_json::Value::Null),
    }
}

fn build_health_track(attrs: &AttrLookup, base: &str) -> Option<HealthTrack> {
    // The base attribute (e.g. "health") carries `max` on the Backbone model;
    // damage is tracked in sibling attributes "<base>_superficial" and
    // "<base>_aggravated". If the base attribute isn't present at all, the
    // sheet doesn't carry health/willpower yet — return None.
    let base_attr = attrs.get(base)?;
    let max = base_attr.max.parse::<u8>().unwrap_or(5);
    let superficial = attrs
        .parse_u8(&format!("{base}_superficial"))
        .unwrap_or(0);
    let aggravated = attrs
        .parse_u8(&format!("{base}_aggravated"))
        .unwrap_or(0);
    Some(HealthTrack {
        max,
        superficial,
        aggravated,
    })
}

struct AttrLookup<'a> {
    attrs: &'a [Attribute],
}

impl<'a> AttrLookup<'a> {
    fn new(attrs: &'a [Attribute]) -> Self {
        Self { attrs }
    }
    fn get(&self, name: &str) -> Option<&'a Attribute> {
        self.attrs.iter().find(|a| a.name == name)
    }
    fn parse_u8(&self, name: &str) -> Option<u8> {
        self.get(name)?.current.parse::<u8>().ok()
    }
}
