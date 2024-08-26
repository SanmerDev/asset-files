use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::MultipartForm;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::UNIX_EPOCH;
use std::{fs, io};

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

#[derive(MultipartForm)]
pub struct Upload {
    #[multipart(rename = "file")]
    pub files: Vec<TempFile>,
}

#[derive(Serialize, Deserialize)]
pub struct File {
    pub name: String,
    pub size: u64,
    pub timestamp: u128,
}

#[derive(Serialize, Deserialize)]
pub struct Rename {
    pub from: String,
    pub to: String,
}

pub fn raed<P: AsRef<Path>>(p: P) -> io::Result<File> {
    let metadata = fs::metadata(&p)?;
    let time = metadata.modified()?;
    let duration = time
        .duration_since(UNIX_EPOCH)
        .map_err(|e| invalid_data!(e))?;
    let path = p.as_ref();
    let Some(file_name) = path.file_name() else {
        return Err(invalid_data!("Unnamed"));
    };
    let Some(name) = file_name.to_str() else {
        return Err(invalid_data!("Not UTF-8"));
    };
    Ok(File {
        name: name.to_string(),
        size: metadata.len(),
        timestamp: duration.as_millis(),
    })
}

pub fn list<P: AsRef<Path>>(p: P) -> io::Result<Vec<File>> {
    let entries = fs::read_dir(p)?;
    let mut files: Vec<File> = Vec::new();
    for entry in entries.flat_map(|v| v.ok()) {
        let path = entry.path();
        let item = skip_err!(raed(path));
        files.push(item);
    }
    Ok(files)
}

pub fn create(upload: Upload, root_dir: &Path) -> Vec<File> {
    let mut files: Vec<File> = Vec::new();
    for file in upload.files {
        let name = skip_none!(file.file_name);
        let to = root_dir.join(name);
        let _size = skip_err!(fs::copy(file.file, &to));
        let item = skip_err!(raed(to));
        files.push(item);
    }
    files
}

pub fn rename(rename: &Rename, root_dir: &Path) -> io::Result<File> {
    let from = root_dir.join(&rename.from);
    let to = root_dir.join(&rename.to);
    fs::rename(&from, &to)?;
    raed(to)
}

pub fn rename_all(renames: &[Rename], root_dir: &Path) -> Vec<File> {
    renames.iter().flat_map(|r| rename(r, root_dir)).collect()
}

pub fn delete(name: String, root_dir: &Path) -> io::Result<String> {
    let path = root_dir.join(&name);
    if path.is_file() {
        fs::remove_file(&path)?;
        return Ok(name);
    }
    if path.is_dir() {
        fs::remove_dir_all(&path)?;
        return Ok(name);
    }
    Err(io::Error::new(io::ErrorKind::NotFound, name))
}

pub fn delete_all(names: Vec<String>, root_dir: &Path) -> Vec<String> {
    names
        .into_iter()
        .flat_map(|n| delete(n, root_dir))
        .collect()
}
