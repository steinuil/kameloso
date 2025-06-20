use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RawMessage {
    CommandResponse {
        request_id: Option<i64>,
        data: Option<serde_json::Value>,
        error: String,
    },
    Event {
        event: String,
        #[serde(flatten)]
        fields: serde_json::Value,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Response {
        request_id: i64,
        data: Result<serde_json::Value, String>,
    },
    ResponseWithoutId(Result<serde_json::Value, String>),
    Event {
        event: String,
        fields: serde_json::Value,
    },
}

impl Message {
    pub(super) const LINE_SEPARATOR: u8 = b'\n';

    pub(super) fn from_slice(slice: &[u8]) -> Result<Self, serde_json::Error> {
        let raw_msg: RawMessage = serde_json::from_slice(slice)?;
        Ok(raw_msg.into())
    }
}

impl From<RawMessage> for Message {
    fn from(value: RawMessage) -> Self {
        match value {
            RawMessage::CommandResponse {
                request_id: Some(request_id),
                data,
                error,
            } => Message::Response {
                request_id,
                data: if error == "success" {
                    Ok(data.unwrap_or(serde_json::Value::Null))
                } else {
                    Err(error)
                },
            },
            RawMessage::CommandResponse {
                request_id: None,
                data,
                error,
            } => Message::ResponseWithoutId(if error == "success" {
                Ok(data.unwrap_or(serde_json::Value::Null))
            } else {
                Err(error)
            }),
            RawMessage::Event { event, fields } => Message::Event { event, fields },
        }
    }
}
