use crate::schema::CancelNotification;
use serde::Serialize;

use crate::jsonrpc::{JrMessage, JrNotification};
use crate::util::json_cast;

impl JrMessage for CancelNotification {
    fn to_untyped_message(&self) -> Result<crate::UntypedMessage, crate::Error> {
        let method = self.method().to_string();
        crate::UntypedMessage::new(&method, self)
    }

    fn method(&self) -> &str {
        "session/cancel"
    }
}

impl JrNotification for CancelNotification {
    fn parse_notification(
        method: &str,
        params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        if method != "session/cancel" {
            return None;
        }

        Some(json_cast(params))
    }
}
