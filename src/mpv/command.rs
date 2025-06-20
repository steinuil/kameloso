use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RawCommand {
    pub request_id: i64,
    pub command: serde_json::Value,
    pub r#async: bool,
}

impl RawCommand {
    pub fn new(request_id: i64, command: serde_json::Value) -> Self {
        Self {
            request_id,
            command,
            r#async: true,
        }
    }

    pub fn serialize_to_vec(&self) -> Vec<u8> {
        serde_json::to_vec(self)
            .unwrap_or_else(|e| unreachable!("failed to serialize command: {e}"))
    }
}
