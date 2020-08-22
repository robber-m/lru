use walkdir::WalkDir;
use chrono::prelude::*;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::cmp::Ordering;
use std::fs::remove_file;
use fs2;

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

struct Args {
    /// if provided, do not remove any files and instead print file paths which would be removed if
    /// the program were to be run with the given arguments
    dry_run : bool,

    /// enable verbose logging
    verbose : bool,

    /// the minimum empty filesystem space in bytes to leave available for use
    target_available_capacity : u64,
}

fn main() {
    let current_available_space = fs2::available_space(".").unwrap();
    // TODO: argument parsing
    let args = Args { dry_run : true, verbose : false, target_available_capacity : current_available_space + 10 * 1024 * 1024 * 1024 };
    // TODO: end argument parsing

    let mut n_bytes_deleted = 0;
    if current_available_space < args.target_available_capacity {
        let mut files_to_delete = BinaryHeap::<FileInfo>::new();
        let mut aggregate_heap_file_size = 0;
        let max_n_bytes_to_delete = args.target_available_capacity - current_available_space;

        for entry in WalkDir::new(".")
            .into_iter()
            .filter_map(|entry| entry.ok()) {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let accessed = metadata.accessed().unwrap().into();
                    if aggregate_heap_file_size < max_n_bytes_to_delete || accessed <= files_to_delete.peek().unwrap().accessed  {
                        // NOTE: if our aggregate heap file size is above capacity, we _must_ have something
                        // in the heap already
                        let file = FileInfo { accessed: accessed, size : metadata.len(), path : entry.into_path() };
                        aggregate_heap_file_size += file.size;
                        files_to_delete.push(file);

                        // NOTE: we should always have at least one file on the heap at this point
                        while aggregate_heap_file_size - files_to_delete.peek().unwrap().size > max_n_bytes_to_delete {
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

        // TODO: don't delete _any_ files that were accessed w/in the last 15-20 minutes
        // TODO: if any files exceed max_file_size, be sure to delete them instead of the
        // less-recently used stuff

        // re-query available space in case our capacity has been reduced since we started running the program
        let n_bytes_to_delete = args.target_available_capacity as i64 - fs2::available_space(".").unwrap() as i64;
        if n_bytes_to_delete > 0 {
            while let Some(file) = files_to_delete.peek() {
                // if the space we need to reclaim has shrunk since we initially queried it (prior
                // to filling up the heap), pop the most-recently-accessed elements until the heap
                // reaches an appropriate size.
                if aggregate_heap_file_size - file.size > n_bytes_to_delete as u64 {
                    aggregate_heap_file_size -= files_to_delete.pop().unwrap().size;
                    // we don't need to delete this file
                } else {
                    break;
                }
            }
            while let Some(file) = files_to_delete.pop() {
                n_bytes_deleted += file.size;
                if args.dry_run {
                    println!("{} {}", file.accessed.format("%m/%d/%Y %T"), file.path.display());
                } else if remove_file(&file.path).is_ok() && args.verbose {
                    println!("Deleted {} {}", file.accessed.format("%m/%d/%Y %T"), file.path.display());
                }
            }
        }
    }

    println!("Deleted {} bytes", n_bytes_deleted);
}
