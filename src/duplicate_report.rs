use std::cmp::Reverse;
use std::collections::HashMap;
use std::mem::take;

use crate::byte_size::ByteSize;
use crate::hash_data::FileEntry;

pub fn duplicate_report(
    data_file: Vec<FileEntry>,
    other_data_file: Option<Vec<FileEntry>>,
    minimum: Option<ByteSize>,
) {
    let mut hash_index: HashMap<String, Vec<FileEntry>> = HashMap::with_capacity(data_file.len());

    for mut file in data_file {
        let hash = take(&mut file.hash);

        let hash_group = hash_index.entry(hash).or_insert(Vec::default());

        hash_group.push(file);
    }

    if let Some(other_data_file) = other_data_file {
        for mut file in other_data_file {
            let hash = take(&mut file.hash);

            let hash_group = hash_index.entry(hash).or_insert(Vec::default());

            hash_group.push(file);
        }
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

        if size < minimum.unwrap_or(ByteSize::Byte(1)).into() {
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

fn format_file_size(size: u64) -> (u64, &'static str) {
    match size {
        ..1_000 => (size, "B"),
        ..1_000_000 => (size / 1_000, "KB"),
        ..1_000_000_000 => (size / 1_000_000, "MB"),
        ..1_000_000_000_000 => (size / 1_000_000_000, "GB"),
        _ => (size / 1_000_000_000_000, "TB"),
    }
}
