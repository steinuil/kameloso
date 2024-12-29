use super::message::Message;

pub struct MessageBuffer {
    line_buffer: Vec<u8>,
}

impl MessageBuffer {
    pub fn new() -> Self {
        Self {
            line_buffer: Vec::new(),
        }
    }

    pub fn insert(&mut self, buf: &[u8]) -> Vec<Result<Message, serde_json::Error>> {
        match buf.iter().rposition(|c| c == &Message::LINE_SEPARATOR) {
            None => {
                self.line_buffer.extend_from_slice(buf);
                vec![]
            }
            Some(i) => {
                self.line_buffer.extend_from_slice(&buf[..i]);

                let mut messages = vec![];

                for line in self.line_buffer.split(|c| c == &Message::LINE_SEPARATOR) {
                    let msg = Message::from_slice(line);
                    messages.push(msg);
                }

                self.line_buffer.clear();

                if i != buf.len() - 1 {
                    self.line_buffer.extend_from_slice(&buf[i + 1..]);
                }

                messages
            }
        }
    }
}

#[tokio::test]
async fn test_message_buffer_multiple_lines() {
    let mut msg_buf = MessageBuffer::new();

    let messages = msg_buf.insert(
        br#"{"request_id":1,"error":"success"}
            {"request_id":2,"error":"success"}
            {"request_id":3,"error":"error message"}
            {"request_id":4,"error":"test"}"#,
    );

    assert!(matches!(
        &messages[..],
        &[
            Ok(Message::Response {
                request_id: 1,
                data: Ok(serde_json::Value::Null)
            }),
            Ok(Message::Response {
                request_id: 2,
                data: Ok(serde_json::Value::Null)
            }),
            Ok(Message::Response {
                request_id: 3,
                data: Err(_)
            })
        ]
    ));
    let messages = msg_buf.insert(b"\n");

    assert!(matches!(
        &messages[..],
        &[Ok(Message::Response {
            request_id: 4,
            data: Err(_)
        })]
    ));
}
