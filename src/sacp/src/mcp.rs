use crate::{HasCounterpart, JrRole, SendsTo, UntypedMessage};

/// The ACP client role (e.g., an IDE or editor).
///
/// Clients initiate communication with agents, sending requests like
/// `initialize`, `prompt`, and `tools/call`.
///
/// The default counterpart for a client is [`AcpAgent`].
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct McpClientRole;

impl JrRole for McpClientRole {}

/// The ACP agent role.
///
/// Agents respond to client requests and can send requests back to clients
/// (e.g., `sampling/createMessage` for LLM calls).
///
/// The default counterpart for an agent is [`AcpClient`].
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct McpServerRole;

impl JrRole for McpServerRole {}

impl HasCounterpart<McpServerRole> for McpClientRole {}

impl HasCounterpart<McpClientRole> for McpServerRole {}

impl SendsTo<McpClientRole, UntypedMessage> for McpServerRole {}

impl SendsTo<McpServerRole, UntypedMessage> for McpClientRole {}
