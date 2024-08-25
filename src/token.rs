use actix_web::dev::ServiceRequest;
use actix_web::error;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

const TOKEN_NAME: &str = "token-name";

#[derive(Clone, Serialize, Deserialize)]
pub struct Token {
    name: String,
    token: String,
}

impl Token {
    pub fn load<P: AsRef<Path>>(p: P) -> Vec<Token> {
        match fs::read(p) {
            Ok(v) => serde_json::from_slice(&v).unwrap_or_else(|_| Vec::new()),
            Err(_) => Vec::new(),
        }
    }

    pub fn load_map<P: AsRef<Path>>(p: P) -> HashMap<String, String> {
        Self::load(p)
            .into_iter()
            .map(|t| (t.token, t.name))
            .collect()
    }
}

pub async fn validator(
    tokens: Arc<HashMap<String, String>>,
    mut req: ServiceRequest,
    credentials: Option<BearerAuth>,
) -> Result<ServiceRequest, (error::Error, ServiceRequest)> {
    if tokens.is_empty() {
        return Ok(req);
    }

    let Some(credentials) = credentials else {
        return Err((error::ErrorBadRequest(""), req));
    };

    let token = credentials.token();
    if tokens.contains_key(token) {
        Ok(match tokens.get(token) {
            None => req,
            Some(name) => {
                let header = req.headers_mut();
                if let Ok(value) = HeaderValue::from_bytes(name.as_bytes()) {
                    header.insert(HeaderName::from_static(TOKEN_NAME), value);
                }
                req
            }
        })
    } else {
        Err((error::ErrorUnauthorized(""), req))
    }
}
