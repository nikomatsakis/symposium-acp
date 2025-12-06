//! Protocol types for proxy and MCP-over-ACP communication.
//!
//! These types are intended to become part of the ACP protocol specification.

use crate::{JrMessage, JrNotification, JrRequest, JrResponsePayload, UntypedMessage};
use serde::{Deserialize, Serialize};

// =============================================================================
// Successor forwarding protocol
// =============================================================================

const SUCCESSOR_REQUEST_METHOD: &str = "_proxy/successor/request";

/// A request being sent to the successor component.
///
/// Used in `_proxy/successor/request` when the proxy wants to forward a request downstream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessorRequest<Req: JrRequest> {
    /// The message to be sent to the successor component.
    #[serde(flatten)]
    pub request: Req,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl<Req: JrRequest> JrMessage for SuccessorRequest<Req> {
    fn to_untyped_message(&self) -> Result<UntypedMessage, crate::Error> {
        UntypedMessage::new(
            SUCCESSOR_REQUEST_METHOD,
            SuccessorRequest {
                request: self.request.to_untyped_message()?,
                meta: self.meta.clone(),
            },
        )
    }

    fn method(&self) -> &str {
        SUCCESSOR_REQUEST_METHOD
    }

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method == SUCCESSOR_REQUEST_METHOD {
            match crate::util::json_cast::<_, SuccessorRequest<UntypedMessage>>(params) {
                Ok(outer) => match Req::parse_request(&outer.request.method, &outer.request.params)
                {
                    Some(Ok(request)) => Some(Ok(SuccessorRequest {
                        request,
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

    fn parse_notification(
        _method: &str,
        _params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        None // Request, not notification
    }
}

impl<Req: JrRequest> JrRequest for SuccessorRequest<Req> {
    type Response = Req::Response;
}

const SUCCESSOR_NOTIFICATION_METHOD: &str = "_proxy/successor/notification";

/// A notification being sent to the successor component.
///
/// Used in `_proxy/successor/notification` when the proxy wants to forward a notification downstream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessorNotification<Req: JrNotification> {
    /// The message to be sent to the successor component.
    #[serde(flatten)]
    pub notification: Req,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl<Req: JrNotification> JrMessage for SuccessorNotification<Req> {
    fn to_untyped_message(&self) -> Result<UntypedMessage, crate::Error> {
        UntypedMessage::new(
            SUCCESSOR_NOTIFICATION_METHOD,
            SuccessorNotification {
                notification: self.notification.to_untyped_message()?,
                meta: self.meta.clone(),
            },
        )
    }

    fn method(&self) -> &str {
        SUCCESSOR_NOTIFICATION_METHOD
    }

    fn parse_request(
        _method: &str,
        _params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        None // Notification, not request
    }

    fn parse_notification(
        method: &str,
        params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        if method == SUCCESSOR_NOTIFICATION_METHOD {
            match crate::util::json_cast::<_, SuccessorNotification<UntypedMessage>>(params) {
                Ok(outer) => match Req::parse_notification(
                    &outer.notification.method,
                    &outer.notification.params,
                ) {
                    Some(Ok(notification)) => Some(Ok(SuccessorNotification {
                        notification,
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

impl<Req: JrNotification> JrNotification for SuccessorNotification<Req> {}

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

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method != METHOD_MCP_CONNECT_REQUEST {
            return None;
        }
        Some(crate::util::json_cast(params))
    }

    fn parse_notification(
        _method: &str,
        _params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        // This is a request, not a notification
        None
    }
}

impl JrRequest for McpConnectRequest {
    type Response = McpConnectResponse;
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

    fn parse_request(
        _method: &str,
        _params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        // This is a notification, not a request
        None
    }

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

impl JrNotification for McpDisconnectNotification {}

/// JSON-RPC method name for MCP requests over ACP
pub const METHOD_MCP_REQUEST: &str = "_mcp/request";

/// An MCP request sent via ACP. This could be an MCP-server-to-MCP-client request
/// (in which case it goes from the ACP client to the ACP agent,
/// note the reversal of roles) or an MCP-client-to-MCP-server request
/// (in which case it goes from the ACP agent to the ACP client).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpOverAcpRequest<R> {
    /// id given in response to `_mcp/connect` request.
    pub connection_id: String,

    /// Request to be sent to the MCP server or client.
    #[serde(flatten)]
    pub request: R,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl<R: JrRequest> JrMessage for McpOverAcpRequest<R> {
    fn to_untyped_message(&self) -> Result<UntypedMessage, crate::Error> {
        let message = self.request.to_untyped_message()?;
        UntypedMessage::new(
            METHOD_MCP_REQUEST,
            McpOverAcpRequest {
                connection_id: self.connection_id.clone(),
                request: message,
                meta: self.meta.clone(),
            },
        )
    }

    fn method(&self) -> &str {
        METHOD_MCP_REQUEST
    }

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method == METHOD_MCP_REQUEST {
            match crate::util::json_cast::<_, McpOverAcpRequest<UntypedMessage>>(params) {
                Ok(outer) => match R::parse_request(&outer.request.method, &outer.request.params) {
                    Some(Ok(request)) => Some(Ok(McpOverAcpRequest {
                        connection_id: outer.connection_id,
                        request,
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

    fn parse_notification(
        _method: &str,
        _params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        None // Request, not notification
    }
}

impl<R: JrRequest> JrRequest for McpOverAcpRequest<R> {
    type Response = R::Response;
}

/// JSON-RPC method name for MCP notifications over ACP
pub const METHOD_MCP_NOTIFICATION: &str = "_mcp/notification";

/// An MCP notification sent via ACP, either from the MCP client (the ACP agent)
/// or the MCP server (the ACP client).
///
/// Delivered via `_mcp/notification` when the MCP client (the ACP agent)
/// sends a notification to the MCP server (the ACP client).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpOverAcpNotification<R> {
    /// id given in response to `_mcp/connect` request.
    pub connection_id: String,

    /// Notification to be sent to the MCP server or client.
    #[serde(flatten)]
    pub notification: R,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl<R: JrMessage> JrMessage for McpOverAcpNotification<R> {
    fn to_untyped_message(&self) -> Result<UntypedMessage, crate::Error> {
        let params = self.notification.to_untyped_message()?;
        UntypedMessage::new(
            METHOD_MCP_NOTIFICATION,
            McpOverAcpNotification {
                connection_id: self.connection_id.clone(),
                notification: params,
                meta: self.meta.clone(),
            },
        )
    }

    fn method(&self) -> &str {
        METHOD_MCP_NOTIFICATION
    }

    fn parse_request(
        _method: &str,
        _params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        None // Notification, not request
    }

    fn parse_notification(
        method: &str,
        params: &impl Serialize,
    ) -> Option<Result<Self, crate::Error>> {
        if method == METHOD_MCP_NOTIFICATION {
            match crate::util::json_cast::<_, McpOverAcpNotification<UntypedMessage>>(params) {
                Ok(outer) => match R::parse_notification(
                    &outer.notification.method,
                    &outer.notification.params,
                ) {
                    Some(Ok(notification)) => Some(Ok(McpOverAcpNotification {
                        connection_id: outer.connection_id,
                        notification,
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

impl<R: JrMessage> JrNotification for McpOverAcpNotification<R> {}
