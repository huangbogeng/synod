use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{PrincipalId, PrincipalKind, TopicId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    pub id: PrincipalId,
    pub kind: PrincipalKind,
    pub handle: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Topic {
    pub id: TopicId,
    pub key: String,
    pub title: String,
    pub description: String,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTopic {
    pub key: String,
    pub title: String,
    pub description: String,
}

impl CreateTopic {
    pub fn new(
        key: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, ValidationError> {
        let key = key.into();
        let title = title.into().trim().to_owned();
        let description = description.into();

        validate_topic_key(&key)?;
        if title.is_empty() || title.chars().count() > 200 {
            return Err(ValidationError::InvalidTitle);
        }
        if description.chars().count() > 20_000 {
            return Err(ValidationError::DescriptionTooLong);
        }

        Ok(Self {
            key,
            title,
            description,
        })
    }
}

pub fn validate_handle(handle: &str) -> Result<(), ValidationError> {
    let valid_length = (2..=39).contains(&handle.len());
    let valid_chars = handle
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_".contains(&byte));
    let valid_edges = handle
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && handle
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric);

    if valid_length && valid_chars && valid_edges {
        Ok(())
    } else {
        Err(ValidationError::InvalidHandle)
    }
}

fn validate_topic_key(key: &str) -> Result<(), ValidationError> {
    let valid_length = (2..=32).contains(&key.len());
    let valid_chars = key
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    let valid_edges = key
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && key.as_bytes().last().is_some_and(u8::is_ascii_alphanumeric);

    if valid_length && valid_chars && valid_edges {
        Ok(())
    } else {
        Err(ValidationError::InvalidTopicKey)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("handle must be 2-39 lowercase letters, digits, hyphens, or underscores")]
    InvalidHandle,
    #[error("topic key must be 2-32 lowercase letters, digits, or hyphens")]
    InvalidTopicKey,
    #[error("topic title must contain 1-200 characters")]
    InvalidTitle,
    #[error("topic description must not exceed 20000 characters")]
    DescriptionTooLong,
    #[error("issue type is invalid")]
    InvalidIssueType,
    #[error("body must not exceed 100000 characters")]
    BodyTooLong,
    #[error("body must not be empty")]
    EmptyBody,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_input_is_normalized_and_validated() {
        let topic = CreateTopic::new("factor-lab", "  Factor Lab  ", "Research").unwrap();
        assert_eq!(topic.title, "Factor Lab");

        assert_eq!(
            CreateTopic::new("Factor Lab", "Title", "").unwrap_err(),
            ValidationError::InvalidTopicKey
        );
        assert_eq!(
            CreateTopic::new("ok", "   ", "").unwrap_err(),
            ValidationError::InvalidTitle
        );
    }

    #[test]
    fn handles_are_deliberately_simple() {
        assert!(validate_handle("alice-1").is_ok());
        assert!(validate_handle("Alice").is_err());
        assert!(validate_handle("-alice").is_err());
    }
}
