//! Validation helpers and request DTOs for personal access token endpoints.

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError, ValidationErrors};

lazy_static! {
    static ref NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_-]{3,64}$").unwrap();
    static ref SCOPE_REGEX: Regex = Regex::new(r"^[a-z][a-z-]*:[a-z]+$").unwrap();
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenRequest {
    #[validate(custom(function = "validate_token_name"))]
    pub name: String,
    pub description: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[validate(length(min = 1), custom(function = "validate_scopes_list"))]
    pub scopes: Vec<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTokenRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
    pub scopes: Option<Vec<String>>,
}

impl Validate for UpdateTokenRequest {
    fn validate(&self) -> Result<(), ValidationErrors> {
        if let Some(name) = &self.name {
            validate_token_name(name).map_err(|err| {
                let mut errors = ValidationErrors::new();
                errors.add("name", err);
                errors
            })?;
        }

        if let Some(scopes) = &self.scopes {
            validate_scopes_list(scopes).map_err(|err| {
                let mut errors = ValidationErrors::new();
                errors.add("scopes", err);
                errors
            })?;
        }

        if let Some(status) = &self.status {
            if !matches!(status.as_str(), "active" | "revoked" | "expired") {
                let mut errors = ValidationErrors::new();
                errors.add("status", ValidationError::new("invalid_status"));
                return Err(errors);
            }
        }

        Ok(())
    }
}

pub fn validate_token_name(name: &str) -> Result<(), ValidationError> {
    if NAME_REGEX.is_match(name) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_token_name"))
    }
}

pub fn validate_scope(scope: &str) -> Result<(), ValidationError> {
    if SCOPE_REGEX.is_match(scope) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_scope"))
    }
}

fn validate_scopes_list(scopes: &Vec<String>) -> Result<(), ValidationError> {
    for scope in scopes {
        validate_scope(scope)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_validation_allows_valid_patterns() {
        assert!(validate_token_name("admin-token").is_ok());
        assert!(validate_token_name("A1_foo").is_ok());
        assert!(validate_token_name("no").is_err());
    }

    #[test]
    fn scope_validation() {
        assert!(validate_scope("clusters:read").is_ok());
        assert!(validate_scope("route-configs:read").is_ok());
        assert!(validate_scope("route-configs:write").is_ok());
        assert!(validate_scope("services:read").is_ok());
        assert!(validate_scope("services:write").is_ok());
        assert!(validate_scope("bad_scope").is_err());
        assert!(validate_scope("bad-scope-").is_err());
    }

    #[test]
    fn update_validation_checks_optional_fields() {
        let mut request = UpdateTokenRequest {
            name: Some("new-name".into()),
            description: None,
            status: Some("revoked".into()),
            expires_at: None,
            scopes: Some(vec!["clusters:read".into()]),
        };
        assert!(request.validate().is_ok());

        request.name = Some("!bad".into());
        assert!(request.validate().is_err());

        request.name = Some("good".into());
        request.scopes = Some(vec!["invalid".into()]);
        assert!(request.validate().is_err());

        request.scopes = Some(vec!["clusters:read".into()]);
        request.status = Some("unknown".into());
        assert!(request.validate().is_err());
    }
}
