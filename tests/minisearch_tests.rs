//! MiniSearch tests - TDD style

use minirust_search::{MiniSearch, MiniSearchOptions, SearchOptions};
use std::collections::HashMap;

fn create_test_index() -> MiniSearch {
    let options = MiniSearchOptions::new(&["title", "text"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "Moby Dick", "Call me Ishmael. Some years ago..."));
    ms.add(doc(2, "Zen and the Art of Motorcycle Maintenance", "I can see by my watch..."));
    ms.add(doc(3, "Neuromancer", "The sky above the port was the color of television..."));

    ms
}

fn doc(id: u32, title: &str, text: &str) -> HashMap<String, String> {
    let mut d = HashMap::new();
    d.insert("id".to_string(), id.to_string());
    d.insert("title".to_string(), title.to_string());
    d.insert("text".to_string(), text.to_string());
    d
}

// === Constructor and Options ===

#[test]
fn test_new_with_fields() {
    let options = MiniSearchOptions::new(&["title", "body"]);
    let ms = MiniSearch::new(options);

    assert_eq!(ms.document_count(), 0);
    assert_eq!(ms.term_count(), 0);
}

#[test]
fn test_custom_id_field() {
    let mut options = MiniSearchOptions::new(&["name"]);
    options.id_field = "custom_id".to_string();

    let mut ms = MiniSearch::new(options);

    let mut doc = HashMap::new();
    doc.insert("custom_id".to_string(), "abc".to_string());
    doc.insert("name".to_string(), "Test".to_string());

    ms.add(doc);
    assert!(ms.has("abc"));
}

#[test]
fn test_store_fields() {
    let mut options = MiniSearchOptions::new(&["title"]);
    options.store_fields = vec!["title".to_string(), "author".to_string()];

    let mut ms = MiniSearch::new(options);

    let mut doc = HashMap::new();
    doc.insert("id".to_string(), "1".to_string());
    doc.insert("title".to_string(), "Test Book".to_string());
    doc.insert("author".to_string(), "John Doe".to_string());

    ms.add(doc);

    let stored = ms.get_stored_fields("1").unwrap();
    assert_eq!(stored.get("title").unwrap(), "Test Book");
    assert_eq!(stored.get("author").unwrap(), "John Doe");
}

// === Adding Documents ===

#[test]
fn test_add_single_document() {
    let options = MiniSearchOptions::new(&["title"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "Hello World", ""));

    assert_eq!(ms.document_count(), 1);
    assert!(ms.has("1"));
}

#[test]
fn test_add_multiple_documents() {
    let options = MiniSearchOptions::new(&["title"]);
    let mut ms = MiniSearch::new(options);

    ms.add_all(vec![
        doc(1, "First", ""),
        doc(2, "Second", ""),
        doc(3, "Third", ""),
    ]);

    assert_eq!(ms.document_count(), 3);
}

#[test]
#[should_panic(expected = "duplicate")]
fn test_add_duplicate_id_panics() {
    let options = MiniSearchOptions::new(&["title"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "First", ""));
    ms.add(doc(1, "Duplicate", "")); // Should panic
}

// === Basic Search ===

#[test]
fn test_search_single_term() {
    let ms = create_test_index();
    let results = ms.search("Ishmael", None);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "1");
}

#[test]
fn test_search_multiple_terms() {
    let ms = create_test_index();
    let results = ms.search("sky television", None);

    assert!(!results.is_empty());
    assert_eq!(results[0].id, "3"); // Neuromancer
}

#[test]
fn test_search_no_results() {
    let ms = create_test_index();
    let results = ms.search("xyznonexistent", None);

    assert!(results.is_empty());
}

#[test]
fn test_search_case_insensitive() {
    let ms = create_test_index();

    let results1 = ms.search("MOBY", None);
    let results2 = ms.search("moby", None);
    let results3 = ms.search("Moby", None);

    assert_eq!(results1.len(), 1);
    assert_eq!(results2.len(), 1);
    assert_eq!(results3.len(), 1);
}

#[test]
fn test_search_returns_score() {
    let ms = create_test_index();
    let results = ms.search("Ishmael", None);

    assert!(!results.is_empty());
    assert!(results[0].score > 0.0);
}

#[test]
fn test_search_results_sorted_by_score() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    // Doc with term appearing once
    let mut d1 = HashMap::new();
    d1.insert("id".to_string(), "1".to_string());
    d1.insert("text".to_string(), "hello world".to_string());

    // Doc with term appearing multiple times
    let mut d2 = HashMap::new();
    d2.insert("id".to_string(), "2".to_string());
    d2.insert("text".to_string(), "hello hello hello".to_string());

    ms.add(d1);
    ms.add(d2);

    let results = ms.search("hello", None);
    assert!(results.len() >= 2);
    assert!(results[0].score >= results[1].score);
}

// === Prefix Search ===

#[test]
fn test_prefix_search() {
    let ms = create_test_index();

    let mut opts = SearchOptions::default();
    opts.prefix = true;

    let results = ms.search("Neu", Some(opts));

    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.id == "3")); // Neuromancer
}

#[test]
fn test_prefix_search_partial_word() {
    let ms = create_test_index();

    let mut opts = SearchOptions::default();
    opts.prefix = true;

    let results = ms.search("Motor", Some(opts));

    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.id == "2")); // Motorcycle
}

// === Fuzzy Search ===

#[test]
fn test_fuzzy_search() {
    let ms = create_test_index();

    let mut opts = SearchOptions::default();
    opts.fuzzy = Some(1); // max 1 edit

    let results = ms.search("Mobi", Some(opts)); // Missing 'y'

    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.id == "1")); // Moby Dick
}

#[test]
fn test_fuzzy_search_typo() {
    let ms = create_test_index();

    let mut opts = SearchOptions::default();
    opts.fuzzy = Some(1);

    let results = ms.search("Mobx", Some(opts)); // Typo

    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.id == "1"));
}

// === Field Boosting ===

#[test]
fn test_field_boost() {
    let options = MiniSearchOptions::new(&["title", "text"]);
    let mut ms = MiniSearch::new(options);

    // "test" appears in title of doc 1, in text of doc 2
    let mut d1 = HashMap::new();
    d1.insert("id".to_string(), "1".to_string());
    d1.insert("title".to_string(), "test".to_string());
    d1.insert("text".to_string(), "other content".to_string());

    let mut d2 = HashMap::new();
    d2.insert("id".to_string(), "2".to_string());
    d2.insert("title".to_string(), "other title".to_string());
    d2.insert("text".to_string(), "test content here".to_string());

    ms.add(d1);
    ms.add(d2);

    // Boost title field
    let mut opts = SearchOptions::default();
    opts.boost.insert("title".to_string(), 10.0);

    let results = ms.search("test", Some(opts));

    assert!(results.len() >= 2);
    assert_eq!(results[0].id, "1"); // Title match should rank higher
}

// === Combine Modes ===

#[test]
fn test_search_combine_or() {
    let ms = create_test_index();

    let mut opts = SearchOptions::default();
    opts.combine_with = minirust_search::CombineWith::Or;

    let results = ms.search("Moby watch", Some(opts));

    // Should match both docs (Moby Dick and Zen)
    assert!(results.len() >= 2);
}

#[test]
fn test_search_combine_and() {
    let ms = create_test_index();

    let mut opts = SearchOptions::default();
    opts.combine_with = minirust_search::CombineWith::And;

    // "me" appears in both Moby Dick ("Call me Ishmael") and Zen ("I can see by my watch")
    // but "Ishmael" only appears in Moby Dick
    let results = ms.search("me Ishmael", Some(opts));

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "1");
}

// === Remove Documents ===

#[test]
fn test_discard_document() {
    let mut ms = create_test_index();

    ms.discard("1");

    assert!(!ms.has("1"));
    assert_eq!(ms.document_count(), 2);

    // Discarded doc should not appear in search
    let results = ms.search("Ishmael", None);
    assert!(results.is_empty());
}

#[test]
fn test_discard_all() {
    let mut ms = create_test_index();

    ms.discard_all(&["1", "2"]);

    assert_eq!(ms.document_count(), 1);
    assert!(!ms.has("1"));
    assert!(!ms.has("2"));
    assert!(ms.has("3"));
}

#[test]
fn test_replace_document() {
    let mut ms = create_test_index();

    let mut new_doc = HashMap::new();
    new_doc.insert("id".to_string(), "1".to_string());
    new_doc.insert("title".to_string(), "Updated Title".to_string());
    new_doc.insert("text".to_string(), "New content".to_string());

    ms.replace(new_doc);

    // Old content should not match
    let results = ms.search("Ishmael", None);
    assert!(results.is_empty());

    // New content should match
    let results = ms.search("Updated", None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "1");
}

// === Match Info ===

#[test]
fn test_search_result_match_info() {
    let ms = create_test_index();
    let results = ms.search("sky color", None);

    assert!(!results.is_empty());

    let result = &results[0];
    assert!(!result.terms.is_empty());
    assert!(!result.query_terms.is_empty());
    assert!(!result.match_info.is_empty());
}

// === Document Count and Stats ===

#[test]
fn test_term_count() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    let mut d = HashMap::new();
    d.insert("id".to_string(), "1".to_string());
    d.insert("text".to_string(), "one two three".to_string());

    ms.add(d);

    assert!(ms.term_count() >= 3);
}
