use actix_web::dev::ServiceRequest;
use actix_web::error;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

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

fn login_log(name: &String, req: &ServiceRequest) {
    let connection_info = req.connection_info();
    let addr = connection_info.realip_remote_addr().unwrap_or("-");
    tracing::info!("{name} (IP: {addr})");
}

pub async fn validator(
    tokens: Arc<HashMap<String, String>>,
    req: ServiceRequest,
    credentials: Option<BearerAuth>,
) -> Result<ServiceRequest, (error::Error, ServiceRequest)> {
    if tokens.is_empty() {
        return Ok(req);
    }

    let Some(credentials) = credentials else {
        return Err((error::ErrorBadRequest(""), req));
    };

    if let Some(name) = tokens.get(credentials.token()) {
        login_log(name, &req);
        Ok(req)
    } else {
        Err((error::ErrorUnauthorized(""), req))
    }
}
