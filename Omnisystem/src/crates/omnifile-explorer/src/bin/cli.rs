//! Omnifile-explorer CLI: creates a couple of virtual files, navigates
//! around, bookmarks a directory, then indexes and searches the files.

use omnifile_explorer::{Explorer, FileIndexer, IndexEntry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let explorer = Explorer::new();
    explorer.create_file("/docs/report.txt".to_string(), 2048)?;
    explorer.create_file("/docs/photo.png".to_string(), 500_000)?;
    explorer.navigate("/docs".to_string());
    explorer.add_bookmark("docs".to_string(), "/docs".to_string())?;

    println!(
        "Explorer at '{}' has {} file(s) and {} bookmark(s)",
        explorer.get_current_path(),
        explorer.file_count(),
        explorer.bookmark_count()
    );

    let indexer = FileIndexer::new();
    indexer.index_file(IndexEntry {
        path: "/docs/report.txt".to_string(),
        name: "report.txt".to_string(),
        size: 2048,
        tags: vec!["document".to_string()],
    })?;
    indexer.index_file(IndexEntry {
        path: "/docs/photo.png".to_string(),
        name: "photo.png".to_string(),
        size: 500_000,
        tags: vec!["image".to_string()],
    })?;

    let matches = indexer.search_by_name("report");
    println!("Search for 'report' found {} match(es)", matches.len());

    Ok(())
}
