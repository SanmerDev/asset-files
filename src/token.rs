use actix_web::dev::ServiceRequest;
use actix_web::http::Method;
use actix_web::{error, web};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
    req: ServiceRequest,
    credentials: Option<BearerAuth>,
) -> Result<ServiceRequest, (error::Error, ServiceRequest)> {
    let Some(tokens) = req.app_data::<web::Data<HashMap<String, String>>>() else {
        return Ok(req);
    };
    if tokens.is_empty() || req.method() == Method::GET {
        return Ok(req);
    }
    let Some(credentials) = credentials else {
        return Err((error::ErrorBadRequest(""), req));
    };
    if let Some(_name) = tokens.get(credentials.token()) {
        Ok(req)
    } else {
        Err((error::ErrorUnauthorized(""), req))
    }
}
