//! Vacuum tests - TDD style

use minirust_search::{MiniSearch, MiniSearchOptions};
use std::collections::HashMap;

fn doc(id: u32, text: &str) -> HashMap<String, String> {
    let mut d = HashMap::new();
    d.insert("id".to_string(), id.to_string());
    d.insert("text".to_string(), text.to_string());
    d
}

#[test]
fn test_dirt_count_after_discard() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "hello world"));
    ms.add(doc(2, "goodbye world"));

    assert_eq!(ms.dirt_count(), 0);

    ms.discard("1");
    assert_eq!(ms.dirt_count(), 1);

    ms.discard("2");
    assert_eq!(ms.dirt_count(), 2);
}

#[test]
fn test_dirt_factor() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "one"));
    ms.add(doc(2, "two"));
    ms.add(doc(3, "three"));
    ms.add(doc(4, "four"));

    // Discard 2 of 4 = 50% dirt factor
    ms.discard("1");
    ms.discard("2");

    assert_eq!(ms.dirt_count(), 2);
    // dirt_factor = dirt_count / (document_count + dirt_count) = 2 / 4 = 0.5
    assert!((ms.dirt_factor() - 0.5).abs() < 0.01);
}

#[test]
fn test_vacuum_clears_dirt() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "hello world"));
    ms.add(doc(2, "goodbye world"));

    ms.discard("1");
    assert_eq!(ms.dirt_count(), 1);

    ms.vacuum();
    assert_eq!(ms.dirt_count(), 0);
}

#[test]
fn test_vacuum_removes_orphaned_terms() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "unique_term"));
    ms.add(doc(2, "other content"));

    let initial_terms = ms.term_count();

    ms.discard("1");
    // Term still in index but doc is gone
    assert_eq!(ms.term_count(), initial_terms);

    ms.vacuum();
    // After vacuum, orphaned term should be removed
    assert!(ms.term_count() < initial_terms);
}

#[test]
fn test_vacuum_on_empty_index() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    // Should not panic
    ms.vacuum();
    assert_eq!(ms.dirt_count(), 0);
}

#[test]
fn test_search_after_vacuum() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "hello world"));
    ms.add(doc(2, "hello universe"));
    ms.add(doc(3, "goodbye world"));

    ms.discard("1");
    ms.vacuum();

    // Search should still work correctly
    let results = ms.search("hello", None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "2");

    let results = ms.search("world", None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "3");
}

#[test]
fn test_is_vacuuming() {
    let options = MiniSearchOptions::new(&["text"]);
    let ms = MiniSearch::new(options);

    // Synchronous vacuum, should not be vacuuming
    assert!(!ms.is_vacuuming());
}
