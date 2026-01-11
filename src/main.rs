use std::{
    env::current_dir,
    error::Error,
    fs::{File, read_dir},
    io::{BufReader, Read},
    path::PathBuf,
    time::UNIX_EPOCH,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct FileEntry {
    file_name: String,
    hash: String,
    modified: u64,
}

fn main() {
    let mut data_file: Vec<FileEntry> = Vec::default();

    let mut pending_directories_list: Vec<PathBuf> = Vec::default();

    match current_dir() {
        Ok(current_path) => pending_directories_list.push(current_path),
        Err(err) => {
            println!("{err:?}");
            return;
        }
    }

    loop {
        let current_directory = match pending_directories_list.pop() {
            Some(current_directory) => current_directory,
            None => {
                println!("Done");
                return;
            }
        };

        match core(current_directory, &mut data_file) {
            Ok(mut subdirectory_list) => {
                pending_directories_list.append(&mut subdirectory_list);
            }
            Err(error) => {
                println!("{error:?}");
                return;
            }
        }
    }
}

fn core(current_path: PathBuf, data_file: &mut Vec<FileEntry>) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    println!("current_path: {}", current_path.to_string_lossy());

    let mut file_list: Vec<PathBuf> = Vec::default();
    let mut subdirectory_list: Vec<PathBuf> = Vec::default();

    for current_entry in read_dir(current_path)? {
        match current_entry {
            Err(err) => println!("Error reading directory entry: {err:?}"),
            Ok(entry) => {
                let path = entry.path();

                if path.is_dir() {
                    subdirectory_list.push(path);
                } else if path.is_file() {
                    file_list.push(path);
                }
            }
        }
    }

    for current_file in file_list {
        let file_name = current_file.to_string_lossy().to_string();

        let file = File::open(current_file)?;

        let modified = file
            .metadata()?
            .modified()?
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let hash = hash_file(file)?;

        println!("{file_name} {hash}");

        data_file.push(FileEntry {
            file_name,
            modified,
            hash,
        });
    }

    return Ok(subdirectory_list);
}

fn hash_file(file: File) -> Result<String, Box<dyn Error>> {
    let mut reader = BufReader::new(file);

    let mut hasher = Sha256::default();

    let mut buffer = [0u8; 8192];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hex::encode(hasher.finalize().to_vec()))
}
