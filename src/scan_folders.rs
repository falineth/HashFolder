use std::fs::{File, OpenOptions, read_dir};
use std::io::{BufReader, Read, Stdout, stdout};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crossterm::{cursor, execute, terminal};
use sha2::{Digest, Sha256};

use crate::errors::{AppError, AppErrorResult};
use crate::hash_data::FileEntry;
use crate::or_else;
use crate::utils::check_exit_key_pressed;

pub fn scan_folder_tree(
    mut data_file: Vec<FileEntry>,
    starting_dir: &Path,
) -> (Option<Vec<FileEntry>>, Option<AppError>) {
    println!("Press Q to stop and save progress");

    let mut out: Stdout = stdout();

    data_file = or_else!(
        scan_for_deleted(data_file),
        err => return (None, Some(err))
    );

    let scan_result = scan_for_new_and_updated(&mut out, starting_dir, &mut data_file);

    _ = terminal::disable_raw_mode();
    println!();

    return (Some(data_file), scan_result.err());
}

fn scan_for_deleted(hash_data: Vec<FileEntry>) -> Result<Vec<FileEntry>, AppError> {
    let mut result: Vec<FileEntry> = Vec::new();

    for file in hash_data.into_iter() {
        check_exit_key_pressed()?;

        if PathBuf::from(&file.file_name).is_file() {
            result.push(file);
        }
    }

    return Ok(result);
}

fn scan_for_new_and_updated(
    out: &mut Stdout,
    starting_dir: &Path,
    data_file: &mut Vec<FileEntry>,
) -> Result<(), AppError> {
    terminal::enable_raw_mode().app_err()?;

    let mut pending_directories_list: Vec<PathBuf> = Vec::default();

    pending_directories_list.push(starting_dir.into());

    loop {
        let current_directory = or_else!(pending_directories_list.pop(), none => return Ok(()));

        let mut subdirectory_list = process_folder(out, current_directory, data_file)?;

        pending_directories_list.append(&mut subdirectory_list);
    }
}

fn process_folder(
    out: &mut Stdout,
    current_path: PathBuf,
    hash_data: &mut Vec<FileEntry>,
) -> Result<Vec<PathBuf>, AppError> {
    let mut file_list: Vec<PathBuf> = Vec::default();
    let mut subdirectory_list: Vec<PathBuf> = Vec::default();

    let dir_reader = or_else!(
        read_dir(&current_path),
        err => {
            println!(
                "Error reading directory {}: {}",
                current_path.to_string_lossy(),
                err
            );
            execute!(out, cursor::MoveToNextLine(1)).app_err()?;
            return Ok(subdirectory_list);
        }
    );

    for current_entry in dir_reader {
        check_exit_key_pressed()?;

        match current_entry {
            Err(err) => {
                print!("Error reading directory entry: {err:?}");
                execute!(out, cursor::MoveToNextLine(1)).app_err()?;
            }
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

    let terminal_width: usize = terminal::size().map(|size| size.0).unwrap_or(75).into();

    for (index, current_file) in file_list.iter().enumerate() {
        let progress = (index + 1) * 100 / file_list.len();

        println!(
            "{progress}% {:1$.1$}",
            current_path.to_string_lossy(),
            terminal_width - 5
        );
        execute!(out, cursor::MoveToPreviousLine(1)).app_err()?;

        let file_name = current_file.to_string_lossy().to_string();

        let file = or_else!(
            OpenOptions::new().read(true).open(current_file),
            err => {
                println!(
                    "Error reading file {}: {}",
                    current_path.to_string_lossy(),
                    err
                );
                execute!(out, cursor::MoveToNextLine(1)).app_err()?;
                continue;
            }
        );

        let metadata = file.metadata().app_err()?;

        let modified = metadata
            .modified()
            .app_err()?
            .duration_since(UNIX_EPOCH)
            .app_err()?
            .as_secs();

        let file_size = metadata.size();

        let entry_position = hash_data.binary_search_by_key(&&file_name, |entry| &entry.file_name);

        if let Ok(entry_position) = entry_position
            && let Some(entry) = hash_data.get(entry_position)
            && entry.file_size == file_size
            && entry.modified == modified
        {
            continue;
        }

        let hash = hash_file(file)?;

        match entry_position {
            Ok(entry_position) => {
                if let Some(entry) = hash_data.get_mut(entry_position) {
                    entry.hash = hash
                }
            }
            Err(entry_position) => {
                hash_data.insert(
                    entry_position,
                    FileEntry {
                        file_name,
                        file_size,
                        modified,
                        hash,
                    },
                );
            }
        }
    }

    return Ok(subdirectory_list);
}

fn hash_file(file: File) -> Result<String, AppError> {
    let mut reader = BufReader::new(file);

    let mut hasher = Sha256::default();

    let mut buffer = [0u8; 8192];
    loop {
        check_exit_key_pressed()?;

        let n = reader.read(&mut buffer).app_err()?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hex::encode(hasher.finalize()))
}
