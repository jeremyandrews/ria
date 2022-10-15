use std::fs;

use walkdir::{DirEntry, WalkDir};

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}

fn main() {
    let walker = WalkDir::new("music").follow_links(true).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let metadata = fs::metadata(entry.as_ref().unwrap().path()).unwrap();

        // Albums are collected together in directories.
        if metadata.is_dir() {
            println!("Directory ({}): {}", entry.as_ref().unwrap().depth(), entry.as_ref().unwrap().path().display());
        // Files may be tracks.
        } else if metadata.is_file() {
            println!("File ({}): {}", entry.as_ref().unwrap().depth(), entry.as_ref().unwrap().path().display());
        }
    }
}
