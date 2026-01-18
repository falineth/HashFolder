mod byte_size;
mod duplicate_report;
mod errors;
mod hash_data;
mod scan_folders;
mod utils;

use std::env::current_dir;
use std::path::PathBuf;

use clap::Parser;
use errors::AppErrorResult;

use crate::byte_size::{ByteSize, ByteSizeValueParser};
use crate::duplicate_report::duplicate_report;
use crate::errors::AppError;
use crate::hash_data::{FileEntry, load_current_hash_data, save_hash_data};
use crate::scan_folders::scan_folder_tree;

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

    /// Show report without other path
    #[arg(short, long)]
    report: bool,

    /// Minimum duplicate file size to report
    #[arg(short, long, value_parser = ByteSizeValueParser::new())]
    minimum: Option<ByteSize>,
}

fn main() {
    let args = Args::parse();

    let starting_dir = or_else!(get_starting_dir(&args), err => {
        println!("{err:?}");
        return;
    });

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
        let (returned_data_file, scan_err) = scan_folder_tree(data_file, &starting_dir);

        if let Some(scan_err) = &scan_err {
            println!("{scan_err}");
        }

        if let Some(returned_data_file) = returned_data_file {
            data_file = returned_data_file;

            if let Err(err) = save_hash_data(&starting_dir, &data_file) {
                println!("{err}");
            }
        } else {
            return;
        }

        if scan_err.is_some() {
            return;
        }
    }

    if args.other.is_some() || args.report {
        let other_data_file = or_else!(
            get_other_data_file(args.other),
            err => {
                println!("{err}");
                return;
            }
        );

        duplicate_report(data_file, other_data_file, args.minimum);
    }
}

fn get_starting_dir(args: &Args) -> Result<PathBuf, AppError> {
    if let Some(path) = &args.path {
        return path.canonicalize().app_err();
    }

    return current_dir().app_err();
}

fn get_other_data_file(other: Option<PathBuf>) -> Result<Option<Vec<FileEntry>>, AppError> {
    let other_path = or_else!(other, none => return Ok(None));

    let other_data_file = load_current_hash_data(&other_path, false)?;

    return Ok(Some(other_data_file));
}
