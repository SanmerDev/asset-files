mod token;

use crate::token::Token;
use actix_files::Files;
use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::MultipartForm;
use actix_web::{delete, get, middleware, put, web, App, HttpResponse, HttpServer, Responder};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use std::{fs, io};

const ROOT_DIR: &str = "/app/data";
const AUTH_JSON: &str = "/etc/asset-files/auth.json";

macro_rules! skip_err {
    ($value:expr) => {
        match $value {
            Ok(v) => v,
            Err(_) => continue,
        }
    };
}

macro_rules! skip_none {
    ($value:expr) => {
        match $value {
            Some(v) => v,
            None => continue,
        }
    };
}

macro_rules! invalid_data {
    ($error:expr) => {
        io::Error::new(io::ErrorKind::InvalidData, $error)
    };
}

macro_rules! json {
    ($content:expr) => {
        HttpResponse::Ok()
            .content_type("application/json")
            .body($content)
    };
}

#[derive(MultipartForm)]
struct UploadForm {
    #[multipart(rename = "file")]
    files: Vec<TempFile>,
}

#[derive(Serialize, Deserialize)]
struct FileItem {
    name: String,
    size: u64,
    timestamp: u128,
}

impl FileItem {
    fn raed<P: AsRef<Path>>(p: P) -> io::Result<Self> {
        let metadata = fs::metadata(&p)?;
        let time = metadata.modified()?;
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(|e| invalid_data!(e))?;
        let timestamp = duration.as_millis();

        let path = p.as_ref();
        let name = match path.file_name() {
            Some(v) => v.to_str().map_or(timestamp.to_string(), |v| v.to_string()),
            None => return Err(invalid_data!("Unnamed")),
        };

        Ok(Self {
            name: name.to_string(),
            size: metadata.len(),
            timestamp,
        })
    }
}

#[put("/cp")]
async fn upload(MultipartForm(form): MultipartForm<UploadForm>) -> impl Responder {
    let root_dir = Path::new(ROOT_DIR);
    let mut values: Vec<FileItem> = Vec::new();
    for file in form.files {
        let name = skip_none!(file.file_name);
        let to = root_dir.join(name);
        let _size = skip_err!(fs::copy(file.file, &to));
        let item = skip_err!(FileItem::raed(to));
        values.push(item);
    }

    values.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    json!(serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_owned()))
}

#[get("/ls")]
async fn list() -> impl Responder {
    let entries = match fs::read_dir(ROOT_DIR) {
        Ok(v) => v,
        Err(_) => return HttpResponse::NoContent().finish(),
    };

    let mut values: Vec<FileItem> = Vec::new();
    for entry in entries.flat_map(|v| v.ok()) {
        let path = entry.path();
        let item = skip_err!(FileItem::raed(path));
        values.push(item);
    }

    values.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    json!(serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_owned()))
}

#[delete("/rm/{name}")]
async fn delete(name: web::Path<String>) -> impl Responder {
    let path = name.into_inner();
    let root_dir = Path::new(ROOT_DIR);
    let file = root_dir.join(&path);
    if !file.is_file() {
        return HttpResponse::NotFound();
    }

    match fs::remove_file(file) {
        Ok(_) => HttpResponse::NoContent(),
        Err(_) => HttpResponse::Forbidden(),
    }
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    fs::create_dir_all(ROOT_DIR).ok();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    HttpServer::new(|| {
        let tokens = Arc::new(Token::load_map(AUTH_JSON));
        App::new()
            .service(
                web::scope("/api")
                    .service(upload)
                    .service(list)
                    .service(delete),
            )
            .service(Files::new("/", ROOT_DIR))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::new(
                "%{token-name}i %a %r %s %b %{Referer}i %{User-Agent}i %T",
            ))
            .wrap(HttpAuthentication::with_fn(move |r, c| {
                let tokens = tokens.to_owned();
                token::validator(tokens, r, c)
            }))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
