//! Advanced features tests - filling gaps from original MiniSearch

use minirust_search::{MiniSearch, MiniSearchOptions, SearchOptions, Document};
use std::collections::HashMap;

// === Custom Tokenization ===

#[test]
fn test_custom_tokenizer() {
    let mut options = MiniSearchOptions::new(&["text"]);
    // Custom tokenizer: split on commas
    options.tokenize = Some(Box::new(|text: &str, _field: &str| {
        text.split(',').map(|s| s.trim().to_string()).collect()
    }));

    let mut ms = MiniSearch::new(options);

    let mut doc = HashMap::new();
    doc.insert("id".to_string(), "1".to_string());
    doc.insert("text".to_string(), "apple, banana, cherry".to_string());
    ms.add(doc);

    // Should find "banana" as a term (not split on spaces)
    let results = ms.search("banana", None);
    assert_eq!(results.len(), 1);

    // Should NOT find "app" with default prefix (since "apple" is the full token)
    let results = ms.search("app", None);
    assert!(results.is_empty());
}

#[test]
fn test_custom_process_term() {
    let mut options = MiniSearchOptions::new(&["text"]);
    // Custom processTerm: remove 's' suffix (crude stemming)
    options.process_term = Some(Box::new(|term: &str, _field: &str| {
        let t = term.to_lowercase();
        if t.ends_with('s') && t.len() > 1 {
            vec![t[..t.len()-1].to_string()]
        } else {
            vec![t]
        }
    }));

    let mut ms = MiniSearch::new(options);

    let mut doc = HashMap::new();
    doc.insert("id".to_string(), "1".to_string());
    doc.insert("text".to_string(), "cats dogs birds".to_string());
    ms.add(doc);

    // "cat" should match "cats" after stemming
    let results = ms.search("cat", None);
    assert_eq!(results.len(), 1);
}

#[test]
fn test_process_term_expansion() {
    let mut options = MiniSearchOptions::new(&["text"]);
    // Expand abbreviations
    options.process_term = Some(Box::new(|term: &str, _field: &str| {
        let t = term.to_lowercase();
        if t == "lb" || t == "lbs" {
            vec!["lb".to_string(), "pound".to_string()]
        } else {
            vec![t]
        }
    }));

    let mut ms = MiniSearch::new(options);

    let mut doc = HashMap::new();
    doc.insert("id".to_string(), "1".to_string());
    doc.insert("text".to_string(), "10 lbs of flour".to_string());
    ms.add(doc);

    // Both "lbs" and "pound" should match
    let results = ms.search("pound", None);
    assert_eq!(results.len(), 1);
}

#[test]
fn test_process_term_rejection() {
    let mut options = MiniSearchOptions::new(&["text"]);
    // Skip short terms (stopwords-like behavior)
    options.process_term = Some(Box::new(|term: &str, _field: &str| {
        let t = term.to_lowercase();
        if t.len() <= 3 {
            vec![] // reject terms with 3 or fewer chars
        } else {
            vec![t]
        }
    }));

    let mut ms = MiniSearch::new(options);

    let mut doc = HashMap::new();
    doc.insert("id".to_string(), "1".to_string());
    doc.insert("text".to_string(), "the big cat is on the mat".to_string());
    ms.add(doc);

    // "the", "big", "cat", "is", "on", "mat" all <= 3 chars, should be skipped
    let results = ms.search("the", None);
    assert!(results.is_empty());

    let results = ms.search("cat", None);
    assert!(results.is_empty());

    // Add a longer term to verify the index works
    let mut doc2 = HashMap::new();
    doc2.insert("id".to_string(), "2".to_string());
    doc2.insert("text".to_string(), "hello world".to_string());
    ms.add(doc2);

    let results = ms.search("hello", None);
    assert_eq!(results.len(), 1);
}

// === Filter and Boost Callbacks ===

#[test]
fn test_filter_results() {
    let mut options = MiniSearchOptions::new(&["title"]);
    options.store_fields = vec!["category".to_string()];

    let mut ms = MiniSearch::new(options);

    let mut d1 = HashMap::new();
    d1.insert("id".to_string(), "1".to_string());
    d1.insert("title".to_string(), "JavaScript Guide".to_string());
    d1.insert("category".to_string(), "programming".to_string());

    let mut d2 = HashMap::new();
    d2.insert("id".to_string(), "2".to_string());
    d2.insert("title".to_string(), "JavaScript Recipes".to_string());
    d2.insert("category".to_string(), "cooking".to_string());

    ms.add(d1);
    ms.add(d2);

    let mut opts = SearchOptions::default();
    opts.filter = Some(Box::new(|result| {
        result.stored_fields.get("category").map_or(false, |c| c == "programming")
    }));

    let results = ms.search("JavaScript", Some(opts));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "1");
}

#[test]
fn test_boost_document() {
    let mut options = MiniSearchOptions::new(&["title"]);
    options.store_fields = vec!["priority".to_string()];

    let mut ms = MiniSearch::new(options);

    let mut d1 = HashMap::new();
    d1.insert("id".to_string(), "1".to_string());
    d1.insert("title".to_string(), "test document".to_string());
    d1.insert("priority".to_string(), "low".to_string());

    let mut d2 = HashMap::new();
    d2.insert("id".to_string(), "2".to_string());
    d2.insert("title".to_string(), "test document".to_string());
    d2.insert("priority".to_string(), "high".to_string());

    ms.add(d1);
    ms.add(d2);

    let mut opts = SearchOptions::default();
    opts.boost_document = Some(Box::new(|_id, _term, stored| {
        if stored.get("priority").map_or(false, |p| p == "high") {
            Some(10.0)
        } else {
            Some(1.0)
        }
    }));

    let results = ms.search("test", Some(opts));
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "2"); // high priority first
}

// === Wildcard Queries ===

#[test]
fn test_wildcard_query() {
    let mut options = MiniSearchOptions::new(&["title"]);
    options.store_fields = vec!["category".to_string()];

    let mut ms = MiniSearch::new(options);

    let mut d1 = HashMap::new();
    d1.insert("id".to_string(), "1".to_string());
    d1.insert("title".to_string(), "First".to_string());
    d1.insert("category".to_string(), "a".to_string());

    let mut d2 = HashMap::new();
    d2.insert("id".to_string(), "2".to_string());
    d2.insert("title".to_string(), "Second".to_string());
    d2.insert("category".to_string(), "b".to_string());

    ms.add(d1);
    ms.add(d2);

    // Wildcard matches all documents
    let results = ms.search_wildcard(None);
    assert_eq!(results.len(), 2);

    // Wildcard with filter
    let mut opts = SearchOptions::default();
    opts.filter = Some(Box::new(|result| {
        result.stored_fields.get("category").map_or(false, |c| c == "a")
    }));

    let results = ms.search_wildcard(Some(opts));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "1");
}

// === Nested Query Trees ===

#[test]
fn test_nested_query_or_and() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    let mut d1 = HashMap::new();
    d1.insert("id".to_string(), "1".to_string());
    d1.insert("text".to_string(), "apple banana".to_string());

    let mut d2 = HashMap::new();
    d2.insert("id".to_string(), "2".to_string());
    d2.insert("text".to_string(), "apple cherry".to_string());

    let mut d3 = HashMap::new();
    d3.insert("id".to_string(), "3".to_string());
    d3.insert("text".to_string(), "banana cherry".to_string());

    ms.add(d1);
    ms.add(d2);
    ms.add(d3);

    // Query: apple AND (banana OR cherry)
    use minirust_search::{Query, CombineWith};

    let query = Query::Combined {
        combine_with: CombineWith::And,
        queries: vec![
            Query::Term("apple".to_string()),
            Query::Combined {
                combine_with: CombineWith::Or,
                queries: vec![
                    Query::Term("banana".to_string()),
                    Query::Term("cherry".to_string()),
                ],
            },
        ],
    };

    let results = ms.search_query(query, None);
    assert_eq!(results.len(), 2); // docs 1 and 2 have apple + (banana or cherry)
}

// === Generic Document Types ===

#[test]
fn test_generic_document() {
    #[derive(Clone)]
    struct Book {
        isbn: String,
        title: String,
        author: String,
    }

    impl Document for Book {
        fn id(&self) -> &str {
            &self.isbn
        }

        fn field(&self, name: &str) -> Option<&str> {
            match name {
                "title" => Some(&self.title),
                "author" => Some(&self.author),
                _ => None,
            }
        }

        fn stored_field(&self, name: &str) -> Option<String> {
            self.field(name).map(|s| s.to_string())
        }
    }

    let options = MiniSearchOptions::new(&["title", "author"]);
    let mut ms = MiniSearch::new(options);

    ms.add(Book {
        isbn: "123".to_string(),
        title: "Rust Programming".to_string(),
        author: "Jane Doe".to_string(),
    });

    let results = ms.search("Rust", None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "123");
}

// === Remove vs Discard ===

#[test]
fn test_remove_immediate() {
    let options = MiniSearchOptions::new(&["text"]);
    let mut ms = MiniSearch::new(options);

    let mut doc = HashMap::new();
    doc.insert("id".to_string(), "1".to_string());
    doc.insert("text".to_string(), "unique term here".to_string());
    ms.add(doc.clone());

    let initial_terms = ms.term_count();

    // remove() should immediately clean up, not lazy like discard()
    ms.remove(doc);

    // Term count should decrease immediately (no vacuum needed)
    assert!(ms.term_count() < initial_terms);
    assert_eq!(ms.dirt_count(), 0); // No dirt, was cleaned immediately
}
