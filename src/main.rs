use walkdir::WalkDir;
use chrono::prelude::*;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::cmp::{Reverse, Ordering};

#[derive(PartialEq, Eq)]
struct FileInfo {
    accessed : DateTime<Local>,
    size : u64,
    path : PathBuf
}

impl PartialOrd for FileInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.accessed.cmp(&other.accessed))    
    }
}

impl Ord for FileInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.accessed.cmp(&other.accessed)
    }
}

fn main() {
    let mut heap = BinaryHeap::new();

    for entry in WalkDir::new(".")
        .into_iter()
        .filter_map(|entry| entry.ok()) {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                heap.push(Reverse(FileInfo { accessed: metadata.accessed().unwrap().into(), size : metadata.len(), path : entry.into_path() }) )
            }
        }
    }

    while let Some(Reverse(file)) = heap.pop() {
        println!("{} {}", file.accessed.format("%d/%m/%Y %T"), file.path.display());
    }
}
