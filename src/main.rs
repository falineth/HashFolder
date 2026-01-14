mod errors;

use std::cmp::Reverse;
use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{File, OpenOptions, read_dir};
use std::io::{BufReader, BufWriter, Read, Stdout, stdout};
use std::mem::take;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

use clap::{Parser, arg, command};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::{cursor, execute, terminal};
use errors::AppErrorResult;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::errors::AppError;

const HASH_DATA_FILENAME: &str = "hash.json";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct FileEntry {
    file_name: String,
    file_size: u64,
    hash: String,
    modified: u64,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Base path to scan
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Skip updating base path hashes
    #[arg(short, long)]
    skip: bool,

    /// Path to compare
    #[arg(short, long)]
    other: Option<PathBuf>,

    /// Minimum duplicate file size to report
    #[arg(short, long)]
    minimum: Option<u64>,
}

fn main() {
    let args = Args::parse();

    let starting_dir = match get_starting_dir(&args) {
        Ok(current_dir) => current_dir,
        Err(err) => {
            println!("{err:?}");
            return;
        }
    };

    if !starting_dir.exists() {
        println!("Path not found: {}", starting_dir.to_string_lossy());
        return;
    }

    if !starting_dir.is_dir() {
        println!(
            "Path is not a directory: {}",
            starting_dir.to_string_lossy()
        );
        return;
    }

    let mut data_file = load_current_hash_data(&starting_dir, true)
        .expect("Should be able to read hash data file if it exists");

    if !args.skip {
        let mut out: Stdout = stdout();

        data_file = match purge_old_files(data_file) {
            Ok(data_file) => data_file,
            Err(err) => {
                println!("{err}");
                return;
            }
        };

        let scan_result = scan_folders(&mut out, &starting_dir, &mut data_file);

        _ = terminal::disable_raw_mode();
        println!("");

        if let Err(err) = save_hash_data(&starting_dir, &data_file) {
            println!("{err}");
        }

        match scan_result {
            Ok(()) => println!("Done"),
            Err(err) => {
                println!("{err}");
                return;
            }
        }
    }

    if let Some(other) = args.other {
        let other_data_file = match load_current_hash_data(&other, false) {
            Ok(other_data_file) => other_data_file,
            Err(err) => {
                println!("{err}");
                return;
            }
        };

        let mut hash_index: HashMap<String, Vec<FileEntry>> =
            HashMap::with_capacity(other_data_file.len() + data_file.len());

        for mut file in data_file {
            let hash = take(&mut file.hash);

            let hash_group = hash_index.entry(hash).or_insert(Vec::default());

            hash_group.push(file);
        }

        for mut file in other_data_file {
            let hash = take(&mut file.hash);

            let hash_group = hash_index.entry(hash).or_insert(Vec::default());

            hash_group.push(file);
        }

        let mut hash_list: Vec<Vec<FileEntry>> = hash_index
            .into_values()
            .filter(|hash| hash.len() > 1)
            .collect();

        hash_list.sort_unstable_by_key(|entry| {
            Reverse(entry.first().map(|file| file.file_size).unwrap_or_default())
        });

        for hash_group in hash_list {
            let size = hash_group
                .first()
                .map(|file| file.file_size)
                .unwrap_or_default();

            if size < args.minimum.unwrap_or(1) {
                continue;
            }

            let (size, unit) = format_file_size(size);

            println!();
            println!("{} files {}{} each", hash_group.len(), size, unit);
            for file in hash_group {
                println!("{}", file.file_name);
            }
        }
    }
}

fn get_starting_dir(args: &Args) -> Result<PathBuf, AppError> {
    if let Some(path) = &args.path {
        return path.canonicalize().app_err();
    }

    return current_dir().app_err();
}

fn purge_old_files(hash_data: Vec<FileEntry>) -> Result<Vec<FileEntry>, AppError> {
    let mut result: Vec<FileEntry> = Vec::new();

    for file in hash_data.into_iter() {
        check_exit_key_pressed()?;

        if PathBuf::from(&file.file_name).is_file() {
            result.push(file);
        }
    }

    return Ok(result);
}

fn format_file_size(size: u64) -> (u64, &'static str) {
    match size {
        ..1_000 => (size, "B"),
        ..1_000_000 => (size / 1_000, "KB"),
        ..1_000_000_000 => (size / 1_000_000, "MB"),
        ..1_000_000_000_000 => (size / 1_000_000_000, "GB"),
        _ => (size / 1_000_000_000_000, "TB"),
    }
}

fn save_hash_data(starting_dir: &PathBuf, data_file: &Vec<FileEntry>) -> Result<(), AppError> {
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

fn load_current_hash_data(
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

fn scan_folders(
    out: &mut Stdout,
    starting_dir: &PathBuf,
    data_file: &mut Vec<FileEntry>,
) -> Result<(), AppError> {
    terminal::enable_raw_mode().app_err()?;

    let mut pending_directories_list: Vec<PathBuf> = Vec::default();

    pending_directories_list.push(starting_dir.clone());

    loop {
        let current_directory = match pending_directories_list.pop() {
            Some(current_directory) => current_directory,
            None => return Ok(()),
        };

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

    let dir_reader = match read_dir(&current_path) {
        Ok(dir) => dir,
        Err(err) => {
            println!(
                "Error reading directory {}: {}",
                current_path.to_string_lossy(),
                err
            );
            execute!(out, cursor::MoveToNextLine(1)).app_err()?;
            return Ok(subdirectory_list);
        }
    };

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

        let file = match OpenOptions::new().read(true).open(current_file) {
            Ok(file) => file,
            Err(err) => {
                println!(
                    "Error reading file {}: {}",
                    current_path.to_string_lossy(),
                    err
                );
                execute!(out, cursor::MoveToNextLine(1)).app_err()?;
                continue;
            }
        };

        let metadata = file.metadata().app_err()?;

        let modified = metadata
            .modified()
            .app_err()?
            .duration_since(UNIX_EPOCH)
            .app_err()?
            .as_secs();

        let file_size = metadata.size();

        let entry_position = hash_data.binary_search_by_key(&&file_name, |entry| &entry.file_name);

        if let Ok(entry_position) = entry_position {
            if let Some(entry) = hash_data.get(entry_position) {
                if entry.file_size == file_size && entry.modified == modified {
                    continue;
                }
            }
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

    Ok(hex::encode(hasher.finalize().to_vec()))
}

fn check_exit_key_pressed() -> Result<(), AppError> {
    loop {
        if event::poll(Duration::ZERO).app_err()? {
            match event::read().app_err()? {
                Event::Key(KeyEvent {
                    code,
                    modifiers: _,
                    kind: _,
                    state: _,
                }) => match code {
                    KeyCode::Char('q') => {
                        Err(AppError::new(format!("Abort key pressed")))?;
                    }
                    _ => (),
                },
                _ => (),
            }
        } else {
            return Ok(());
        }
    }
}
