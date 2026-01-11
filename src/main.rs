use std::env::current_dir;
use std::error::Error;
use std::fmt::Display;
use std::fs::{File, OpenOptions, read_dir};
use std::io::{BufReader, BufWriter, Read, Stdout, stdout};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{cursor, execute};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const HASH_DATA_FILENAME: &str = "hash.json";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct FileEntry {
    file_name: String,
    file_size: u64,
    hash: String,
    modified: u64,
}

#[derive(Debug, Default)]
struct AbortError {}

impl Display for AbortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Program Aborted")
    }
}

impl Error for AbortError {}

fn main() {
    let starting_dir = match current_dir() {
        Ok(current_dir) => current_dir,
        Err(err) => {
            println!("{err:?}");
            return;
        }
    };

    let mut data_file = load_current_hash_data(&starting_dir)
        .expect("Should be able to read hash data file if it exists");

    let mut out: Stdout = stdout();

    let scan_result = scan_folders(&mut out, &starting_dir, &mut data_file);

    _ = disable_raw_mode();
    println!("");

    match scan_result {
        Ok(()) => println!("Done"),
        Err(err) => println!("{err:?}"),
    }

    if let Err(err) = save_hash_data(starting_dir, data_file) {
        println!("{err:?}");
    }
}

fn save_hash_data(starting_dir: PathBuf, data_file: Vec<FileEntry>) -> Result<(), Box<dyn Error>> {
    let hash_data_filename = starting_dir.join(HASH_DATA_FILENAME);

    let hash_data_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(hash_data_filename)?;

    let writer = BufWriter::new(hash_data_file);

    serde_json::to_writer(writer, &data_file)?;

    return Ok(());
}

fn load_current_hash_data(starting_dir: &PathBuf) -> Result<Vec<FileEntry>, Box<dyn Error>> {
    let hash_data_file = starting_dir.join(HASH_DATA_FILENAME);

    if !hash_data_file.exists() {
        return Ok(Vec::default());
    }

    if !hash_data_file.is_file() {
        Err(AbortError::default())?;
    }

    let file = File::open(hash_data_file)?;

    let mut hash_data: Vec<FileEntry> = serde_json::from_reader(file)?;

    if !hash_data.is_sorted_by_key(|entry| &entry.file_name) {
        hash_data.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    }

    return Ok(hash_data);
}

fn scan_folders(
    out: &mut Stdout,
    starting_dir: &PathBuf,
    data_file: &mut Vec<FileEntry>,
) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;

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
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut file_list: Vec<PathBuf> = Vec::default();
    let mut subdirectory_list: Vec<PathBuf> = Vec::default();

    for current_entry in read_dir(current_path)? {
        check_exit_key_pressed()?;

        match current_entry {
            Err(err) => {
                print!("Error reading directory entry: {err:?}");
                execute!(out, cursor::MoveToNextLine(1))?;
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

    for current_file in file_list {
        let file_name = current_file.to_string_lossy().to_string();

        let file = File::open(current_file)?;

        let metadata = file.metadata()?;

        let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();

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

        print!("{file_name} {hash}");
        execute!(out, cursor::MoveToNextLine(1))?;

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

fn hash_file(file: File) -> Result<String, Box<dyn Error>> {
    let mut reader = BufReader::new(file);

    let mut hasher = Sha256::default();

    let mut buffer = [0u8; 8192];
    loop {
        check_exit_key_pressed()?;

        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hex::encode(hasher.finalize().to_vec()))
}

fn check_exit_key_pressed() -> Result<(), Box<dyn Error>> {
    loop {
        if event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code,
                    modifiers: _,
                    kind: _,
                    state: _,
                }) => match code {
                    KeyCode::Char('q') => {
                        Err(AbortError::default())?;
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
