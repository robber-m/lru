use argh::FromArgs;
use walkdir::WalkDir;
use chrono::prelude::*;
use chrono::Duration;
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

#[derive(FromArgs)]
/// Turn your filesystem into an LRU cache by running this program periodically. When run, if the
/// filesystem for the provided path has fewer than --target-available-space free bytes, delete
/// files in least-recently-accessed order until the target is reached.
struct Args {
    #[argh(switch)]
    /// if provided, do not remove any files and instead print file paths which would be removed if
    /// the program were to be run with the given arguments
    dry_run : bool,

    #[argh(option, short = 't')]
    /// the minimum empty filesystem space in bytes to leave available for use
    target_available_space : u64,

    #[argh(option, short = 'o', default = "0")]
    /// only delete files that were last accessed more than --older-than minutes ago
    older_than : i64,

    #[argh(positional)]
    /// the top-level directory at which to recursively reclaim files when the filesystem capacity
    /// exceeds the target
    path : PathBuf,

    #[argh(switch, short = 'v')]
    /// enable verbose logging
    verbose : bool,
}

fn main() {
    let args: Args = argh::from_env();
    let current_available_space = fs2::available_space(&args.path).unwrap();
    let older_than_time = Local::now() - Duration::minutes(args.older_than);

    let mut n_bytes_deleted = 0;
    if current_available_space < args.target_available_space {
        let mut files_to_delete = BinaryHeap::<FileInfo>::new();
        let mut aggregate_heap_file_size = 0;
        let max_n_bytes_to_delete = args.target_available_space - current_available_space;

        for entry in WalkDir::new(&args.path)
            .into_iter()
            .filter_map(|entry| entry.ok()) {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let accessed = metadata.accessed().unwrap().into();
                    if accessed < older_than_time && (aggregate_heap_file_size < max_n_bytes_to_delete || accessed <= files_to_delete.peek().unwrap().accessed) {
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

        // re-query available space in case our capacity has been reduced since we started running the program
        let n_bytes_to_delete = args.target_available_space as i64 - fs2::available_space(&args.path).unwrap() as i64;
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
                if args.dry_run {
                    n_bytes_deleted += file.size;
                    println!("{} {}", file.accessed.format("%m/%d/%Y %T"), file.path.display());
                } else if remove_file(&file.path).is_ok() && args.verbose {
                    n_bytes_deleted += file.size;
                    println!("Deleted {} {}", file.accessed.format("%m/%d/%Y %T"), file.path.display());
                }
            }
        }
    }

    if args.verbose {
        println!("Deleted {} bytes", n_bytes_deleted);
    }
}
