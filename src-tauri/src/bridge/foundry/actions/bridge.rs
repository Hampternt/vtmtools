// Foundry bridge.* helper builders. These produce outbound wire envelopes
// the desktop sends to the module to control which Foundry collections
// stream over the WebSocket. The `actors` collection is auto-subscribed
// by the module on Hello (preserving today's always-send-actors behavior);
// future tools opt into other collections via `bridge.subscribe`.

use serde_json::{json, Value};

/// Build a `bridge.subscribe { collection }` envelope.
pub fn build_subscribe(collection: &str) -> Value {
    json!({ "type": "bridge.subscribe", "collection": collection })
}

/// Build a `bridge.unsubscribe { collection }` envelope.
pub fn build_unsubscribe(collection: &str) -> Value {
    json!({ "type": "bridge.unsubscribe", "collection": collection })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_envelope_shape() {
        let v = build_subscribe("journal");
        assert_eq!(v["type"], "bridge.subscribe");
        assert_eq!(v["collection"], "journal");
    }

    #[test]
    fn unsubscribe_envelope_shape() {
        let v = build_unsubscribe("scenes");
        assert_eq!(v["type"], "bridge.unsubscribe");
        assert_eq!(v["collection"], "scenes");
    }
}
