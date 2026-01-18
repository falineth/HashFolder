use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppErrorResult};

const HASH_DATA_FILENAME: &str = "hash.json";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub file_name: String,
    pub file_size: u64,
    pub hash: String,
    pub modified: u64,
}

pub fn load_current_hash_data(
    source_path: &Path,
    create: bool,
) -> Result<Vec<FileEntry>, AppError> {
    let hash_data_file_path = get_hash_data_file_path(source_path, create)?;

    if !hash_data_file_path.exists() {
        if create {
            return Ok(Vec::default());
        } else {
            return Err(AppError::new("Comparison hash data file not found".into()));
        }
    }

    if !hash_data_file_path.is_file() {
        Err(AppError::new(format!(
            "Expected {} to be a file",
            hash_data_file_path.to_string_lossy()
        )))?;
    }

    let file = File::open(hash_data_file_path).app_err()?;

    let mut hash_data: Vec<FileEntry> = serde_json::from_reader(file).app_err()?;

    if !hash_data.is_sorted_by_key(|entry| &entry.file_name) {
        hash_data.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    }

    return Ok(hash_data);
}

pub fn get_hash_data_file_path(source_path: &Path, create: bool) -> Result<PathBuf, AppError> {
    if source_path.is_file() {
        return Ok(source_path.to_owned());
    }

    if source_path.is_dir() {
        let data_file_path = source_path.join(HASH_DATA_FILENAME);

        if data_file_path.is_file() || create {
            return Ok(data_file_path);
        } else {
            return Err(AppError::new("Comparison path does not contain hash data file".into()));
        }
    }

    return Err(AppError::new(format!(
        "Comparison path {} not found",
        source_path.to_string_lossy()
    )));
}

pub fn save_hash_data(starting_dir: &Path, data_file: &Vec<FileEntry>) -> Result<(), AppError> {
    let hash_data_filename = starting_dir.join(HASH_DATA_FILENAME);

    let hash_data_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(hash_data_filename)
        .app_err()?;

    let writer = BufWriter::new(hash_data_file);

    serde_json::to_writer(writer, &data_file).app_err()?;

    return Ok(());
}
