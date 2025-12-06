//! Role types for JSON-RPC connections.
//!
//! Roles determine what operations are valid on a connection and how
//! certain operations (like handling unhandled messages) behave.

use std::fmt::Debug;

/// Trait for JSON-RPC connection roles.
///
/// The role determines what operations are valid on a connection and
/// provides role-specific behavior like handling unhandled messages.
pub trait JrRole: Debug + Clone + Send + 'static {}

/// A role that returns an error for unhandled messages.
///
/// This is the default role used when no specific role is provided.
#[derive(Debug, Default, Clone)]
pub struct DefaultRole;

impl JrRole for DefaultRole {}
