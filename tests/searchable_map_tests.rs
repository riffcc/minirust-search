//! SearchableMap tests - TDD style

use minirust_search::SearchableMap;

#[test]
fn test_new_map_is_empty() {
    let map = SearchableMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[test]
fn test_get_or_create_creates_entry() {
    let mut map = SearchableMap::new();

    let data = map.get_or_create("hello");
    assert!(data.is_empty());

    assert_eq!(map.len(), 1);
    assert!(!map.is_empty());
}

#[test]
fn test_get_or_create_returns_existing() {
    let mut map = SearchableMap::new();

    // Create and populate
    {
        let data = map.get_or_create("hello");
        data.insert(0, std::collections::HashMap::from([(1, 2)]));
    }

    // Get again - should return same data
    {
        let data = map.get_or_create("hello");
        assert_eq!(data.get(&0).unwrap().get(&1), Some(&2));
    }

    assert_eq!(map.len(), 1);
}

#[test]
fn test_get_existing() {
    let mut map = SearchableMap::new();
    map.get_or_create("term").insert(0, std::collections::HashMap::from([(1, 5)]));

    let data = map.get("term").unwrap();
    assert_eq!(data.get(&0).unwrap().get(&1), Some(&5));
}

#[test]
fn test_get_nonexistent() {
    let map = SearchableMap::new();
    assert!(map.get("missing").is_none());
}

#[test]
fn test_get_mut() {
    let mut map = SearchableMap::new();
    map.get_or_create("term").insert(0, std::collections::HashMap::from([(1, 1)]));

    if let Some(data) = map.get_mut("term") {
        data.get_mut(&0).unwrap().insert(2, 10);
    }

    let data = map.get("term").unwrap();
    assert_eq!(data.get(&0).unwrap().get(&2), Some(&10));
}

#[test]
fn test_remove() {
    let mut map = SearchableMap::new();
    map.get_or_create("term").insert(0, std::collections::HashMap::from([(1, 1)]));

    let removed = map.remove("term");
    assert!(removed.is_some());
    assert!(map.is_empty());
    assert!(map.get("term").is_none());
}

#[test]
fn test_remove_nonexistent() {
    let mut map = SearchableMap::new();
    assert!(map.remove("missing").is_none());
}

#[test]
fn test_iteration() {
    let mut map = SearchableMap::new();
    map.get_or_create("apple").insert(0, std::collections::HashMap::from([(1, 1)]));
    map.get_or_create("banana").insert(0, std::collections::HashMap::from([(2, 2)]));

    let entries: Vec<_> = map.iter().collect();
    assert_eq!(entries.len(), 2);

    let terms: Vec<_> = entries.iter().map(|(k, _)| k.as_str()).collect();
    assert!(terms.contains(&"apple"));
    assert!(terms.contains(&"banana"));
}

#[test]
fn test_prefix_search() {
    let mut map = SearchableMap::new();
    map.get_or_create("test").insert(0, std::collections::HashMap::from([(1, 1)]));
    map.get_or_create("testing").insert(0, std::collections::HashMap::from([(2, 1)]));
    map.get_or_create("tested").insert(0, std::collections::HashMap::from([(3, 1)]));
    map.get_or_create("other").insert(0, std::collections::HashMap::from([(4, 1)]));

    let view = map.at_prefix("test").unwrap();
    let entries: Vec<_> = view.iter().collect();

    assert_eq!(entries.len(), 3);
    let terms: Vec<_> = entries.iter().map(|(k, _)| k.as_str()).collect();
    assert!(terms.contains(&"test"));
    assert!(terms.contains(&"testing"));
    assert!(terms.contains(&"tested"));
    assert!(!terms.contains(&"other"));
}

#[test]
fn test_fuzzy_search() {
    let mut map = SearchableMap::new();
    map.get_or_create("hello").insert(0, std::collections::HashMap::from([(1, 1)]));
    map.get_or_create("hallo").insert(0, std::collections::HashMap::from([(2, 1)]));
    map.get_or_create("world").insert(0, std::collections::HashMap::from([(3, 1)]));

    let results = map.fuzzy_search("hello", 1);

    assert_eq!(results.len(), 2);
    let terms: Vec<_> = results.iter().map(|(k, _, _)| k.as_str()).collect();
    assert!(terms.contains(&"hello"));
    assert!(terms.contains(&"hallo"));
}

#[test]
fn test_contains_term() {
    let mut map = SearchableMap::new();
    map.get_or_create("exists");

    assert!(map.contains_term("exists"));
    assert!(!map.contains_term("missing"));
}

#[test]
fn test_multiple_fields_and_docs() {
    let mut map = SearchableMap::new();

    // Term "hello" appears in:
    // - field 0, doc 1, freq 2
    // - field 0, doc 2, freq 1
    // - field 1, doc 1, freq 3
    let data = map.get_or_create("hello");
    data.entry(0).or_default().insert(1, 2);
    data.entry(0).or_default().insert(2, 1);
    data.entry(1).or_default().insert(1, 3);

    let retrieved = map.get("hello").unwrap();
    assert_eq!(retrieved.get(&0).unwrap().get(&1), Some(&2));
    assert_eq!(retrieved.get(&0).unwrap().get(&2), Some(&1));
    assert_eq!(retrieved.get(&1).unwrap().get(&1), Some(&3));
}
