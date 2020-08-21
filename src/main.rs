use walkdir::WalkDir;
use chrono::prelude::*;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::cmp::Ordering;

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
    let mut files_to_delete = BinaryHeap::<FileInfo>::new();
    let mut aggregate_heap_file_size = 0;
    let n_bytes_to_delete = 10 * 1024 * 1024; // 10 mb

    for entry in WalkDir::new(".")
        .into_iter()
        .filter_map(|entry| entry.ok()) {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                let accessed = metadata.accessed().unwrap().into();
                if aggregate_heap_file_size < n_bytes_to_delete || accessed > files_to_delete.peek().unwrap().accessed  {
                    // NOTE: if our aggregate heap file size is above capacity, we _must_ have something
                    // in the heap already
                    let file = FileInfo { accessed: accessed, size : metadata.len(), path : entry.into_path() };
                    aggregate_heap_file_size += file.size;
                    files_to_delete.push(file);

                    // NOTE: we should always have at least one file on the heap at this point
                    while aggregate_heap_file_size - files_to_delete.peek().unwrap().size > n_bytes_to_delete {
                        // forget about any newer files that we no longer need to delete now that we have
                        // pushed an older file onto the heap
                        aggregate_heap_file_size -= files_to_delete.pop().unwrap().size;
                    }
                } else {
                    // if our file is newer than the newest thing already on the heap, and our heap
                    // is already at capacity, there's no sense in pushing the file onto the heap
                    // only to remove it immediately afterward
                }
            }
        }
    }

    while let Some(file) = files_to_delete.pop() {
        println!("{} {}", file.accessed.format("%d/%m/%Y %T"), file.path.display());
    }
    println!("Deleting {} bytes", aggregate_heap_file_size);
}
