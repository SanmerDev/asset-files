use crate::file::{Rename, Upload};
use crate::token::Token;
use actix_files::Files;
use actix_multipart::form::MultipartForm;
use actix_web::{get, middleware, post, put, web, App, HttpResponse, HttpServer, Responder};
use actix_web_httpauth::middleware::HttpAuthentication;
use std::path::PathBuf;
use std::{fs, io};

mod file;
mod token;

#[cfg(not(debug_assertions))]
const ROOT_DIR: &str = "/data";
#[cfg(debug_assertions)]
const ROOT_DIR: &str = "data";
#[cfg(not(debug_assertions))]
const AUTH_JSON: &str = "/etc/asset-files/auth.json";
#[cfg(debug_assertions)]
const AUTH_JSON: &str = "auth.json";

#[get("/files")]
async fn get_files() -> impl Responder {
    let mut files = match file::list(ROOT_DIR) {
        Ok(v) => v,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    if files.is_empty() {
        HttpResponse::NotFound().finish()
    } else {
        files.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        HttpResponse::Ok().json(files)
    }
}

#[post("/create")]
async fn create_files(MultipartForm(upload): MultipartForm<Upload>) -> impl Responder {
    let root_dir = PathBuf::from(ROOT_DIR);
    let files = file::create(upload, &root_dir);
    if files.is_empty() {
        HttpResponse::BadRequest().finish()
    } else {
        HttpResponse::Ok().json(files)
    }
}

#[put("/rename")]
async fn rename_files(renames: web::Json<Vec<Rename>>) -> impl Responder {
    let root_dir = PathBuf::from(ROOT_DIR);
    let renames = renames.into_inner();
    let files = file::rename_all(&renames, &root_dir);
    if files.is_empty() {
        HttpResponse::NotFound().finish()
    } else {
        HttpResponse::Ok().json(files)
    }
}

#[post("/delete")]
async fn delete_files(names: web::Json<Vec<String>>) -> impl Responder {
    let root_dir = PathBuf::from(ROOT_DIR);
    let names = names.into_inner();
    let names = file::delete_all(names, &root_dir);
    if names.is_empty() {
        HttpResponse::NotFound().finish()
    } else {
        HttpResponse::Ok().json(names)
    }
}

#[get("/file/{name}")]
async fn get_file(name: web::Path<String>) -> impl Responder {
    let root_dir = PathBuf::from(ROOT_DIR);
    let path = name.into_inner();
    let path = root_dir.join(&path);
    match file::raed(path) {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    fs::create_dir_all(ROOT_DIR).ok();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let tokens = web::Data::new(Token::load_map(AUTH_JSON));
    HttpServer::new(move || {
        App::new()
            .service(
                web::scope("/api")
                    .app_data(tokens.to_owned())
                    .service(get_files)
                    .service(create_files)
                    .service(rename_files)
                    .service(delete_files)
                    .service(get_file)
                    .wrap(HttpAuthentication::with_fn(token::validator)),
            )
            .service(Files::new("/", ROOT_DIR))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
