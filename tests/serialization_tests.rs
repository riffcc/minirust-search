//! Serialization tests - TDD style

use minirust_search::{MiniSearch, MiniSearchOptions};
use std::collections::HashMap;

fn doc(id: u32, title: &str, text: &str) -> HashMap<String, String> {
    let mut d = HashMap::new();
    d.insert("id".to_string(), id.to_string());
    d.insert("title".to_string(), title.to_string());
    d.insert("text".to_string(), text.to_string());
    d
}

#[test]
fn test_to_json() {
    let options = MiniSearchOptions::new(&["title", "text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "Hello", "World"));
    ms.add(doc(2, "Goodbye", "Universe"));

    let json = ms.to_json();

    assert!(json.contains("documentCount"));
    assert!(json.contains("index"));
}

#[test]
fn test_load_json() {
    let options = MiniSearchOptions::new(&["title", "text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "Hello", "World"));
    ms.add(doc(2, "Goodbye", "Universe"));

    let json = ms.to_json();

    // Load from JSON
    let loaded_options = MiniSearchOptions::new(&["title", "text"]);
    let loaded = MiniSearch::load_json(&json, loaded_options).unwrap();

    assert_eq!(loaded.document_count(), 2);
    assert!(loaded.has("1"));
    assert!(loaded.has("2"));
}

#[test]
fn test_load_json_search_works() {
    let options = MiniSearchOptions::new(&["title"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "JavaScript tutorial", ""));
    ms.add(doc(2, "Python basics", ""));

    let json = ms.to_json();

    let loaded_options = MiniSearchOptions::new(&["title"]);
    let loaded = MiniSearch::load_json(&json, loaded_options).unwrap();

    let results = loaded.search("JavaScript", None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "1");
}

#[test]
fn test_serialization_preserves_stored_fields() {
    let mut options = MiniSearchOptions::new(&["title"]);
    options.store_fields = vec!["title".to_string()];

    let mut ms = MiniSearch::new(options);
    ms.add(doc(1, "Test Title", ""));

    let json = ms.to_json();

    let mut loaded_options = MiniSearchOptions::new(&["title"]);
    loaded_options.store_fields = vec!["title".to_string()];

    let loaded = MiniSearch::load_json(&json, loaded_options).unwrap();

    let stored = loaded.get_stored_fields("1").unwrap();
    assert_eq!(stored.get("title").unwrap(), "Test Title");
}

#[test]
fn test_load_json_invalid() {
    let options = MiniSearchOptions::new(&["title"]);
    let result = MiniSearch::load_json("not valid json", options);

    assert!(result.is_err());
}

#[test]
fn test_serialization_roundtrip_fuzzy() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "", "hello world"));
    ms.add(doc(2, "", "hallo welt"));

    let json = ms.to_json();

    let loaded_options = MiniSearchOptions::new(&["text"]);
    let loaded = MiniSearch::load_json(&json, loaded_options).unwrap();

    // Fuzzy search should work on loaded index
    let mut search_opts = minirust_search::SearchOptions::default();
    search_opts.fuzzy = Some(1);

    let results = loaded.search("hello", Some(search_opts));
    assert!(results.len() >= 2);
}

#[test]
fn test_serialization_after_discard() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "", "hello"));
    ms.add(doc(2, "", "world"));

    ms.discard("1");

    let json = ms.to_json();

    let loaded_options = MiniSearchOptions::new(&["text"]);
    let loaded = MiniSearch::load_json(&json, loaded_options).unwrap();

    assert_eq!(loaded.document_count(), 1);
    assert!(!loaded.has("1"));
    assert!(loaded.has("2"));
}
