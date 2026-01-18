use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::path::PathBuf;

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
    starting_dir: &PathBuf,
    create: bool,
) -> Result<Vec<FileEntry>, AppError> {
    let hash_data_file = starting_dir.join(HASH_DATA_FILENAME);

    if !hash_data_file.exists() {
        if create {
            return Ok(Vec::default());
        } else {
            return Err(AppError::new(format!(
                "Comparison hash data file not found"
            )))?;
        }
    }

    if !hash_data_file.is_file() {
        Err(AppError::new(format!(
            "Expected {} to be a file",
            hash_data_file.to_string_lossy()
        )))?;
    }

    let file = File::open(hash_data_file).app_err()?;

    let mut hash_data: Vec<FileEntry> = serde_json::from_reader(file).app_err()?;

    if !hash_data.is_sorted_by_key(|entry| &entry.file_name) {
        hash_data.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    }

    return Ok(hash_data);
}

pub fn save_hash_data(starting_dir: &PathBuf, data_file: &Vec<FileEntry>) -> Result<(), AppError> {
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
