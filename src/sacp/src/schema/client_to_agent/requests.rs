use crate::schema::{
    AuthenticateRequest, AuthenticateResponse, InitializeRequest, InitializeResponse,
    LoadSessionRequest, LoadSessionResponse, NewSessionRequest, NewSessionResponse, PromptRequest,
    PromptResponse, SetSessionModeRequest, SetSessionModeResponse,
};
use serde::Serialize;

use crate::jsonrpc::{JrMessage, JrRequest, JrResponsePayload};
use crate::util::json_cast;

// ============================================================================
// InitializeRequest
// ============================================================================

impl JrMessage for InitializeRequest {
    fn to_untyped_message(&self) -> Result<crate::UntypedMessage, crate::Error> {
        let method = self.method().to_string();
        crate::UntypedMessage::new(&method, self)
    }

    fn method(&self) -> &str {
        "initialize"
    }
}

impl JrRequest for InitializeRequest {
    type Response = InitializeResponse;

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method != "initialize" {
            return None;
        }

        Some(json_cast(params))
    }
}

impl JrResponsePayload for InitializeResponse {
    fn into_json(self, _method: &str) -> Result<serde_json::Value, crate::Error> {
        serde_json::to_value(self).map_err(crate::Error::into_internal_error)
    }

    fn from_value(_method: &str, value: serde_json::Value) -> Result<Self, crate::Error> {
        json_cast(&value)
    }
}

// ============================================================================
// AuthenticateRequest
// ============================================================================

impl JrMessage for AuthenticateRequest {
    fn to_untyped_message(&self) -> Result<crate::UntypedMessage, crate::Error> {
        let method = self.method().to_string();
        crate::UntypedMessage::new(&method, self)
    }

    fn method(&self) -> &str {
        "authenticate"
    }
}

impl JrRequest for AuthenticateRequest {
    type Response = AuthenticateResponse;

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method != "authenticate" {
            return None;
        }
        Some(json_cast(params))
    }
}

impl JrResponsePayload for AuthenticateResponse {
    fn into_json(self, _method: &str) -> Result<serde_json::Value, crate::Error> {
        serde_json::to_value(self).map_err(crate::Error::into_internal_error)
    }

    fn from_value(_method: &str, value: serde_json::Value) -> Result<Self, crate::Error> {
        json_cast(&value)
    }
}

// ============================================================================
// LoadSessionRequest
// ============================================================================

impl JrMessage for LoadSessionRequest {
    fn to_untyped_message(&self) -> Result<crate::UntypedMessage, crate::Error> {
        let method = self.method().to_string();
        crate::UntypedMessage::new(&method, self)
    }

    fn method(&self) -> &str {
        "session/load"
    }
}

impl JrRequest for LoadSessionRequest {
    type Response = LoadSessionResponse;

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method != "session/load" {
            return None;
        }
        Some(json_cast(params))
    }
}

impl JrResponsePayload for LoadSessionResponse {
    fn into_json(self, _method: &str) -> Result<serde_json::Value, crate::Error> {
        serde_json::to_value(self).map_err(crate::Error::into_internal_error)
    }

    fn from_value(_method: &str, value: serde_json::Value) -> Result<Self, crate::Error> {
        json_cast(&value)
    }
}

// ============================================================================
// NewSessionRequest
// ============================================================================

impl JrMessage for NewSessionRequest {
    fn to_untyped_message(&self) -> Result<crate::UntypedMessage, crate::Error> {
        let method = self.method().to_string();
        crate::UntypedMessage::new(&method, self)
    }

    fn method(&self) -> &str {
        "session/new"
    }
}

impl JrRequest for NewSessionRequest {
    type Response = NewSessionResponse;

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method != "session/new" {
            return None;
        }
        Some(json_cast(params))
    }
}

impl JrResponsePayload for NewSessionResponse {
    fn into_json(self, _method: &str) -> Result<serde_json::Value, crate::Error> {
        serde_json::to_value(self).map_err(crate::Error::into_internal_error)
    }

    fn from_value(_method: &str, value: serde_json::Value) -> Result<Self, crate::Error> {
        json_cast(&value)
    }
}

// ============================================================================
// PromptRequest
// ============================================================================

impl JrMessage for PromptRequest {
    fn to_untyped_message(&self) -> Result<crate::UntypedMessage, crate::Error> {
        let method = self.method().to_string();
        crate::UntypedMessage::new(&method, self)
    }

    fn method(&self) -> &str {
        "session/prompt"
    }
}

impl JrRequest for PromptRequest {
    type Response = PromptResponse;

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method != "session/prompt" {
            return None;
        }
        Some(json_cast(params))
    }
}

impl JrResponsePayload for PromptResponse {
    fn into_json(self, _method: &str) -> Result<serde_json::Value, crate::Error> {
        serde_json::to_value(self).map_err(crate::Error::into_internal_error)
    }

    fn from_value(_method: &str, value: serde_json::Value) -> Result<Self, crate::Error> {
        json_cast(&value)
    }
}

// ============================================================================
// SetSessionModeRequest
// ============================================================================

impl JrMessage for SetSessionModeRequest {
    fn to_untyped_message(&self) -> Result<crate::UntypedMessage, crate::Error> {
        let method = self.method().to_string();
        crate::UntypedMessage::new(&method, self)
    }

    fn method(&self) -> &str {
        "session/set_mode"
    }
}

impl JrRequest for SetSessionModeRequest {
    type Response = SetSessionModeResponse;

    fn parse_request(method: &str, params: &impl Serialize) -> Option<Result<Self, crate::Error>> {
        if method != "session/set_mode" {
            return None;
        }
        Some(json_cast(params))
    }
}

impl JrResponsePayload for SetSessionModeResponse {
    fn into_json(self, _method: &str) -> Result<serde_json::Value, crate::Error> {
        serde_json::to_value(self).map_err(crate::Error::into_internal_error)
    }

    fn from_value(_method: &str, value: serde_json::Value) -> Result<Self, crate::Error> {
        json_cast(&value)
    }
}
