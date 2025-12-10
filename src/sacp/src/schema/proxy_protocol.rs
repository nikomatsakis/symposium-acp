//! Protocol types for proxy and MCP-over-ACP communication.
//!
//! These types are intended to become part of the ACP protocol specification.

use crate::{JrMessage, JrNotification, JrRequest, JrResponsePayload, UntypedMessage};
use serde::{Deserialize, Serialize};

// =============================================================================
// Successor forwarding protocol
// =============================================================================

/// JSON-RPC method name for successor forwarding.
pub const METHOD_SUCCESSOR_MESSAGE: &str = "_proxy/successor";

/// A message being sent to the successor component.
///
/// Used in `_proxy/successor` when the proxy wants to forward a message downstream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessorMessage<M: JrMessage = UntypedMessage> {
    /// The message to be sent to the successor component.
    #[serde(flatten)]
    pub message: M,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl<M: JrMessage> JrMessage for SuccessorMessage<M> {
    fn to_untyped_message(&self) -> Result<UntypedMessage, crate::Error> {
        UntypedMessage::new(
            METHOD_SUCCESSOR_MESSAGE,
            SuccessorMessage {
                message: self.message.to_untyped_message()?,
                meta: self.meta.clone(),
            },
        )
    }

    fn method(&self) -> &str {
        METHOD_SUCCESSOR_MESSAGE
    }
}

impl<Req: JrRequest> JrRequest for SuccessorMessage<Req> {
    type Response = Req::Response;

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method == METHOD_SUCCESSOR_MESSAGE {
            match crate::util::json_cast::<_, SuccessorMessage<UntypedMessage>>(params) {
                Ok(outer) => match Req::parse_request(&outer.message.method, &outer.message.params)
                {
                    Some(Ok(request)) => Some(Ok(SuccessorMessage {
                        message: request,
                        meta: outer.meta,
                    })),
                    Some(Err(err)) => Some(Err(err)),
                    None => None,
                },
                Err(err) => Some(Err(err)),
            }
        } else {
            None
        }
    }
}

impl<Notif: JrNotification> JrNotification for SuccessorMessage<Notif> {
    fn parse_notification(
        method: &str,
        params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        if method == METHOD_SUCCESSOR_MESSAGE {
            match crate::util::json_cast::<_, SuccessorMessage<UntypedMessage>>(params) {
                Ok(outer) => {
                    match Notif::parse_notification(&outer.message.method, &outer.message.params) {
                        Some(Ok(notification)) => Some(Ok(SuccessorMessage {
                            message: notification,
                            meta: outer.meta,
                        })),
                        Some(Err(err)) => Some(Err(err)),
                        None => None,
                    }
                }
                Err(err) => Some(Err(err)),
            }
        } else {
            None
        }
    }
}

// =============================================================================
// MCP-over-ACP protocol
// =============================================================================

/// JSON-RPC method name for MCP connect requests
pub const METHOD_MCP_CONNECT_REQUEST: &str = "_mcp/connect";

/// Creates a new MCP connection. This is equivalent to "running the command".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConnectRequest {
    /// The ACP URL to connect to (e.g., "acp:uuid")
    pub acp_url: String,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl JrMessage for McpConnectRequest {
    fn to_untyped_message(&self) -> Result<UntypedMessage, crate::Error> {
        UntypedMessage::new(METHOD_MCP_CONNECT_REQUEST, self)
    }

    fn method(&self) -> &str {
        METHOD_MCP_CONNECT_REQUEST
    }
}

impl JrRequest for McpConnectRequest {
    type Response = McpConnectResponse;

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method != METHOD_MCP_CONNECT_REQUEST {
            return None;
        }
        Some(crate::util::json_cast(params))
    }
}

/// Response to an MCP connect request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConnectResponse {
    /// Unique identifier for the established MCP connection
    pub connection_id: String,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl JrResponsePayload for McpConnectResponse {
    fn into_json(self, _method: &str) -> Result<serde_json::Value, crate::Error> {
        serde_json::to_value(self).map_err(crate::Error::into_internal_error)
    }

    fn from_value(_method: &str, value: serde_json::Value) -> Result<Self, crate::Error> {
        serde_json::from_value(value).map_err(|_| crate::Error::invalid_params())
    }
}

/// JSON-RPC method name for MCP disconnect notifications
pub const METHOD_MCP_DISCONNECT_NOTIFICATION: &str = "_mcp/disconnect";

/// Disconnects the MCP connection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct McpDisconnectNotification {
    /// The id of the connection to disconnect.
    pub connection_id: String,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl JrMessage for McpDisconnectNotification {
    fn to_untyped_message(&self) -> Result<UntypedMessage, crate::Error> {
        UntypedMessage::new(METHOD_MCP_DISCONNECT_NOTIFICATION, self)
    }

    fn method(&self) -> &str {
        METHOD_MCP_DISCONNECT_NOTIFICATION
    }
}

impl JrNotification for McpDisconnectNotification {
    fn parse_notification(
        method: &str,
        params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        if method != METHOD_MCP_DISCONNECT_NOTIFICATION {
            return None;
        }
        Some(crate::util::json_cast(params))
    }
}

/// JSON-RPC method name for MCP requests over ACP
pub const METHOD_MCP_MESSAGE: &str = "_mcp/message";

/// An MCP request sent via ACP. This could be an MCP-server-to-MCP-client request
/// (in which case it goes from the ACP client to the ACP agent,
/// note the reversal of roles) or an MCP-client-to-MCP-server request
/// (in which case it goes from the ACP agent to the ACP client).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpOverAcpMessage<M = UntypedMessage> {
    /// id given in response to `_mcp/connect` request.
    pub connection_id: String,

    /// Request to be sent to the MCP server or client.
    #[serde(flatten)]
    pub message: M,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl<M: JrMessage> JrMessage for McpOverAcpMessage<M> {
    fn to_untyped_message(&self) -> Result<UntypedMessage, crate::Error> {
        let message = self.message.to_untyped_message()?;
        UntypedMessage::new(
            METHOD_MCP_MESSAGE,
            McpOverAcpMessage {
                connection_id: self.connection_id.clone(),
                message,
                meta: self.meta.clone(),
            },
        )
    }

    fn method(&self) -> &str {
        METHOD_MCP_MESSAGE
    }
}

impl<R: JrRequest> JrRequest for McpOverAcpMessage<R> {
    type Response = R::Response;

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method == METHOD_MCP_MESSAGE {
            match crate::util::json_cast::<_, McpOverAcpMessage<UntypedMessage>>(params) {
                Ok(outer) => match R::parse_request(&outer.message.method, &outer.message.params) {
                    Some(Ok(request)) => Some(Ok(McpOverAcpMessage {
                        connection_id: outer.connection_id,
                        message: request,
                        meta: outer.meta,
                    })),
                    Some(Err(err)) => Some(Err(err)),
                    None => None,
                },
                Err(err) => Some(Err(err)),
            }
        } else {
            None
        }
    }
}

impl<R: JrNotification> JrNotification for McpOverAcpMessage<R> {
    fn parse_notification(
        method: &str,
        params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        if method == METHOD_MCP_MESSAGE {
            match crate::util::json_cast::<_, McpOverAcpMessage<UntypedMessage>>(params) {
                Ok(outer) => {
                    match R::parse_notification(&outer.message.method, &outer.message.params) {
                        Some(Ok(notification)) => Some(Ok(McpOverAcpMessage {
                            connection_id: outer.connection_id,
                            message: notification,
                            meta: outer.meta,
                        })),
                        Some(Err(err)) => Some(Err(err)),
                        None => None,
                    }
                }
                Err(err) => Some(Err(err)),
            }
        } else {
            None
        }
    }
}
