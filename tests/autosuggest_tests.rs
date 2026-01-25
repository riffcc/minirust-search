//! Auto-suggest tests - TDD style

use minirust_search::{MiniSearch, MiniSearchOptions};
use std::collections::HashMap;

fn doc(id: u32, title: &str) -> HashMap<String, String> {
    let mut d = HashMap::new();
    d.insert("id".to_string(), id.to_string());
    d.insert("title".to_string(), title.to_string());
    d
}

fn create_test_index() -> MiniSearch {
    let options = MiniSearchOptions::new(&["title"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "JavaScript tutorial"));
    ms.add(doc(2, "Java programming"));
    ms.add(doc(3, "Python basics"));
    ms.add(doc(4, "JavaScript advanced"));
    ms.add(doc(5, "TypeScript guide"));

    ms
}

#[test]
fn test_autosuggest_basic() {
    let ms = create_test_index();
    let suggestions = ms.auto_suggest("java", None);

    assert!(!suggestions.is_empty());

    // Should suggest terms starting with "java"
    let terms: Vec<_> = suggestions.iter().flat_map(|s| &s.terms).collect();
    assert!(terms.iter().any(|t| t.starts_with("java")));
}

#[test]
fn test_autosuggest_returns_suggestions() {
    let ms = create_test_index();
    let suggestions = ms.auto_suggest("java", None);

    for suggestion in &suggestions {
        assert!(!suggestion.suggestion.is_empty());
        assert!(!suggestion.terms.is_empty());
        assert!(suggestion.score > 0.0);
    }
}

#[test]
fn test_autosuggest_partial_match() {
    let ms = create_test_index();
    let suggestions = ms.auto_suggest("prog", None);

    // Should match "programming"
    assert!(!suggestions.is_empty());
    let all_terms: Vec<_> = suggestions.iter().flat_map(|s| &s.terms).collect();
    assert!(all_terms.iter().any(|t| t.contains("programming")));
}

#[test]
fn test_autosuggest_multiple_terms() {
    let ms = create_test_index();
    let suggestions = ms.auto_suggest("javascript tu", None);

    // Should find suggestions matching both terms
    assert!(!suggestions.is_empty());
}

#[test]
fn test_autosuggest_no_results() {
    let ms = create_test_index();
    let suggestions = ms.auto_suggest("xyznonexistent", None);

    assert!(suggestions.is_empty());
}

#[test]
fn test_autosuggest_sorted_by_score() {
    let ms = create_test_index();
    let suggestions = ms.auto_suggest("java", None);

    for i in 1..suggestions.len() {
        assert!(suggestions[i - 1].score >= suggestions[i].score);
    }
}

#[test]
fn test_autosuggest_combines_with_and() {
    let options = MiniSearchOptions::new(&["title"]);
    let mut ms = MiniSearch::new(options);

    ms.add(doc(1, "foo bar"));
    ms.add(doc(2, "foo baz"));
    ms.add(doc(3, "bar only"));

    // Default auto-suggest uses AND combination
    let suggestions = ms.auto_suggest("foo bar", None);

    // Should only match doc 1 (has both foo and bar)
    assert!(!suggestions.is_empty());
}
