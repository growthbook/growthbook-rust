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
    GrowthbookGatewayHttpStatus,
    InvalidResponseValueType,
    GrowthBookAttributeIsNotObject,
    ConfigError,
}

#[derive(Debug)]
pub struct GrowthbookError {
    pub code: GrowthbookErrorCode,
    pub message: String,
    /// The HTTP status code, if this error came from a response.
    pub status: Option<u16>,
}

impl GrowthbookError {
    pub fn new(
        code: GrowthbookErrorCode,
        message: &str,
    ) -> Self {
        GrowthbookError {
            code,
            message: String::from(message),
            status: None,
        }
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
            status: None,
        }
    }
}

impl From<reqwest_middleware::Error> for GrowthbookError {
    fn from(error: reqwest_middleware::Error) -> Self {
        let status = match &error {
            reqwest_middleware::Error::Reqwest(e) => e.status().map(|s| s.as_u16()),
            reqwest_middleware::Error::Middleware(_) => None,
        };
        Self {
            code: GrowthbookErrorCode::GrowthbookGateway,
            message: error.to_string(),
            status,
        }
    }
}

impl From<reqwest::Error> for GrowthbookError {
    fn from(error: reqwest::Error) -> Self {
        let status = error.status().map(|s| s.as_u16());
        Self {
            code: GrowthbookErrorCode::GrowthbookGatewayDeserialize,
            message: error.to_string(),
            status,
        }
    }
}

impl From<VarError> for GrowthbookError {
    fn from(error: VarError) -> Self {
        Self {
            code: GrowthbookErrorCode::MissingEnvironmentVariable,
            message: error.to_string(),
            status: None,
        }
    }
}

impl From<ParseIntError> for GrowthbookError {
    fn from(error: ParseIntError) -> Self {
        Self {
            code: GrowthbookErrorCode::ParseError,
            message: error.to_string(),
            status: None,
        }
    }
}

impl From<serde_json::Error> for GrowthbookError {
    fn from(error: serde_json::Error) -> Self {
        Self {
            code: GrowthbookErrorCode::ParseError,
            message: error.to_string(),
            status: None,
        }
    }
}

impl From<OutOfRangeError> for GrowthbookError {
    fn from(error: OutOfRangeError) -> Self {
        Self {
            code: GrowthbookErrorCode::DurationOutOfRangeError,
            message: error.to_string(),
            status: None,
        }
    }
}

impl From<Response> for GrowthbookError {
    /// Never reads the body (`From` can't be async), so it can't leak it.
    fn from(response: Response) -> Self {
        let status = response.status().as_u16();
        Self {
            code: GrowthbookErrorCode::GrowthbookGatewayHttpStatus,
            message: format!("Failed to get features: unexpected response status {status}"),
            status: Some(status),
        }
    }
}
