use crate::schema::SessionNotification;
use serde::Serialize;

use crate::jsonrpc::{JrMessage, JrNotification};

// Agent -> Client notifications
// These are one-way messages that agents send to clients/editors

impl JrMessage for SessionNotification {
    fn to_untyped_message(&self) -> Result<crate::UntypedMessage, crate::Error> {
        let method = self.method().to_string();
        crate::UntypedMessage::new(&method, self)
    }

    fn method(&self) -> &str {
        "session/update"
    }
}

impl JrNotification for SessionNotification {
    fn parse_notification(
        method: &str,
        params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        if method != "session/update" {
            return None;
        }
        Some(crate::util::json_cast(params))
    }
}
