use std::env::VarError;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;

use chrono::OutOfRangeError;
use reqwest::Response;

#[derive(Debug)]
pub enum GrowthbookErrorCode {
    GenericError,
    SerdeDeserialize,
    ParseError,
    DurationOutOfRangeError,
    MissingEnvironmentVariable,
    GrowthbookGateway,
    GrowthbookGatewayDeserialize,
    InvalidResponseValueType,
    GrowthBookAttributeIsNotObject,
    ConfigError,
}

#[derive(Debug)]
pub struct GrowthbookError {
    pub code: GrowthbookErrorCode,
    pub message: String,
}

impl GrowthbookError {
    pub fn new(
        code: GrowthbookErrorCode,
        message: &str,
    ) -> Self {
        GrowthbookError { code, message: String::from(message) }
    }
}

impl Display for GrowthbookError {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for GrowthbookError {
    fn description(&self) -> &str {
        &self.message
    }
}

impl From<Box<dyn Error>> for GrowthbookError {
    fn from(error: Box<dyn Error>) -> Self {
        Self {
            code: GrowthbookErrorCode::GenericError,
            message: error.to_string(),
        }
    }
}

impl From<reqwest_middleware::Error> for GrowthbookError {
    fn from(error: reqwest_middleware::Error) -> Self {
        Self {
            code: GrowthbookErrorCode::GrowthbookGateway,
            message: error.to_string(),
        }
    }
}

impl From<reqwest::Error> for GrowthbookError {
    fn from(error: reqwest::Error) -> Self {
        Self {
            code: GrowthbookErrorCode::GrowthbookGatewayDeserialize,
            message: error.to_string(),
        }
    }
}

impl From<VarError> for GrowthbookError {
    fn from(error: VarError) -> Self {
        Self {
            code: GrowthbookErrorCode::MissingEnvironmentVariable,
            message: error.to_string(),
        }
    }
}

impl From<ParseIntError> for GrowthbookError {
    fn from(error: ParseIntError) -> Self {
        Self {
            code: GrowthbookErrorCode::ParseError,
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for GrowthbookError {
    fn from(error: serde_json::Error) -> Self {
        Self {
            code: GrowthbookErrorCode::ParseError,
            message: error.to_string(),
        }
    }
}

impl From<OutOfRangeError> for GrowthbookError {
    fn from(error: OutOfRangeError) -> Self {
        Self {
            code: GrowthbookErrorCode::DurationOutOfRangeError,
            message: error.to_string(),
        }
    }
}

impl From<Response> for GrowthbookError {
    fn from(response: Response) -> Self {
        Self {
            code: GrowthbookErrorCode::GrowthbookGateway,
            message: format!("Failed to get features. StatusCode={}", response.status()),
        }
    }
}
