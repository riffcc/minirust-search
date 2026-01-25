//! MiniRust Search - A Rust port of MiniSearch

mod radix_tree;
mod searchable_map;
mod minisearch;

pub use radix_tree::{RadixTree, PrefixView};
pub use searchable_map::SearchableMap;
pub use minisearch::{MiniSearch, MiniSearchOptions, SearchOptions, SearchResult, CombineWith, Suggestion, Query, Document};
