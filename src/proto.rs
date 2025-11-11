//! Generated protobuf bindings.
//!
//! The actual code is produced by `prost-build` during compilation.

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::orderbook::{MessageKind, RouterEnvelope, Snapshot};

    #[test]
    fn router_envelope_roundtrip() {
        let envelope = RouterEnvelope {
            client_id: "client-123".into(),
            kind: MessageKind::Snapshot as i32,
            correlation_id: b"abc123".to_vec(),
            body: Some(super::orderbook::router_envelope::Body::Snapshot(
                Snapshot {
                    level: "L1".into(),
                    payload: b"{\"best_bid\":1}".to_vec(),
                },
            )),
        };

        let mut buf = Vec::new();
        envelope.encode(&mut buf).expect("encode should succeed");
        let decoded = RouterEnvelope::decode(buf.as_slice()).expect("decode should succeed");

        assert_eq!(decoded.client_id, "client-123");
        assert_eq!(decoded.kind, MessageKind::Snapshot as i32);
        assert_eq!(decoded.correlation_id, b"abc123");

        match decoded.body.expect("body missing") {
            super::orderbook::router_envelope::Body::Snapshot(snapshot) => {
                assert_eq!(snapshot.level, "L1");
                assert_eq!(snapshot.payload, b"{\"best_bid\":1}");
            }
            _ => panic!("expected snapshot body"),
        }
    }
}
