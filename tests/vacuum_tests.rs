//! Vacuum and discard tests

use minirust_search::{MiniSearch, MiniSearchOptions};
use std::collections::HashMap;

fn doc(id: u32, text: &str) -> HashMap<String, String> {
    let mut d = HashMap::new();
    d.insert("id".to_string(), id.to_string());
    d.insert("text".to_string(), text.to_string());
    d
}

#[test]
fn discard_cleans_immediately() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "hello world"));
    ms.add(doc(2, "goodbye world"));

    assert_eq!(ms.document_count(), 2);
    let initial_terms = ms.term_count();

    ms.discard("1");
    assert_eq!(ms.document_count(), 1);
    // "hello" was unique to doc 1 — should be removed immediately
    assert!(ms.term_count() < initial_terms);
    // No dirt accumulation
    assert_eq!(ms.dirt_count(), 0);
}

#[test]
fn discard_preserves_shared_terms() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "hello world"));
    ms.add(doc(2, "hello universe"));

    ms.discard("1");
    // "hello" is shared — should still be in the index
    let results = ms.search("hello", None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "2");
}

#[test]
fn vacuum_on_clean_index_is_noop() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "hello world"));
    ms.discard("1");

    let terms_before = ms.term_count();
    ms.vacuum();
    assert_eq!(ms.term_count(), terms_before);
    assert_eq!(ms.dirt_count(), 0);
}

#[test]
fn vacuum_on_empty_index() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.vacuum();
    assert_eq!(ms.dirt_count(), 0);
}

#[test]
fn search_after_discard() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "hello world"));
    ms.add(doc(2, "hello universe"));
    ms.add(doc(3, "goodbye world"));

    ms.discard("1");

    let results = ms.search("hello", None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "2");

    let results = ms.search("world", None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "3");
}

#[test]
fn replace_is_clean() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "original content"));
    ms.add(doc(2, "other stuff"));

    let terms_with_original = ms.term_count();
    ms.replace(doc(1, "replacement content"));

    // "original" should be gone, "replacement" should be there
    let results = ms.search("original", None);
    assert!(results.is_empty());
    let results = ms.search("replacement", None);
    assert_eq!(results.len(), 1);
    assert_eq!(ms.dirt_count(), 0);
}

#[test]
fn is_vacuuming() {
    let options = MiniSearchOptions::new(&["text"]);
    let ms = MiniSearch::new(options);
    assert!(!ms.is_vacuuming());
}
