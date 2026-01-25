//! Radix Tree tests - TDD style

use minirust_search::RadixTree;

#[test]
fn test_new_tree_is_empty() {
    let tree: RadixTree<i32> = RadixTree::new();
    assert!(tree.is_empty());
    assert_eq!(tree.len(), 0);
}

#[test]
fn test_insert_and_get_single() {
    let mut tree = RadixTree::new();
    tree.insert("hello", 1);

    assert_eq!(tree.get("hello"), Some(&1));
    assert_eq!(tree.get("world"), None);
    assert_eq!(tree.len(), 1);
}

#[test]
fn test_insert_multiple_no_overlap() {
    let mut tree = RadixTree::new();
    tree.insert("apple", 1);
    tree.insert("banana", 2);
    tree.insert("cherry", 3);

    assert_eq!(tree.get("apple"), Some(&1));
    assert_eq!(tree.get("banana"), Some(&2));
    assert_eq!(tree.get("cherry"), Some(&3));
    assert_eq!(tree.len(), 3);
}

#[test]
fn test_insert_with_shared_prefix() {
    let mut tree = RadixTree::new();
    tree.insert("test", 1);
    tree.insert("testing", 2);
    tree.insert("tested", 3);
    tree.insert("tester", 4);

    assert_eq!(tree.get("test"), Some(&1));
    assert_eq!(tree.get("testing"), Some(&2));
    assert_eq!(tree.get("tested"), Some(&3));
    assert_eq!(tree.get("tester"), Some(&4));
    assert_eq!(tree.len(), 4);
}

#[test]
fn test_insert_shorter_after_longer() {
    let mut tree = RadixTree::new();
    tree.insert("testing", 1);
    tree.insert("test", 2);

    assert_eq!(tree.get("testing"), Some(&1));
    assert_eq!(tree.get("test"), Some(&2));
}

#[test]
fn test_insert_replaces_existing() {
    let mut tree = RadixTree::new();
    assert_eq!(tree.insert("key", 1), None);
    assert_eq!(tree.insert("key", 2), Some(1));
    assert_eq!(tree.get("key"), Some(&2));
}

#[test]
fn test_get_partial_key_returns_none() {
    let mut tree = RadixTree::new();
    tree.insert("testing", 1);

    assert_eq!(tree.get("test"), None);
    assert_eq!(tree.get("testi"), None);
    assert_eq!(tree.get("testin"), None);
}

#[test]
fn test_get_extended_key_returns_none() {
    let mut tree = RadixTree::new();
    tree.insert("test", 1);

    assert_eq!(tree.get("testing"), None);
    assert_eq!(tree.get("tests"), None);
}

#[test]
fn test_contains_key() {
    let mut tree = RadixTree::new();
    tree.insert("exists", 1);

    assert!(tree.contains_key("exists"));
    assert!(!tree.contains_key("missing"));
    assert!(!tree.contains_key("exist")); // partial
    assert!(!tree.contains_key("existss")); // extended
}

#[test]
fn test_remove_existing() {
    let mut tree = RadixTree::new();
    tree.insert("hello", 1);
    tree.insert("help", 2);

    assert_eq!(tree.remove("hello"), Some(1));
    assert_eq!(tree.get("hello"), None);
    assert_eq!(tree.get("help"), Some(&2));
    assert_eq!(tree.len(), 1);
}

#[test]
fn test_remove_nonexistent() {
    let mut tree = RadixTree::new();
    tree.insert("hello", 1);

    assert_eq!(tree.remove("world"), None);
    assert_eq!(tree.get("hello"), Some(&1));
}

#[test]
fn test_remove_with_remaining_children() {
    let mut tree = RadixTree::new();
    tree.insert("test", 1);
    tree.insert("testing", 2);
    tree.insert("tested", 3);

    assert_eq!(tree.remove("test"), Some(1));
    assert_eq!(tree.get("test"), None);
    assert_eq!(tree.get("testing"), Some(&2));
    assert_eq!(tree.get("tested"), Some(&3));
}

#[test]
fn test_empty_key() {
    let mut tree = RadixTree::new();
    tree.insert("", 1);
    tree.insert("a", 2);

    assert_eq!(tree.get(""), Some(&1));
    assert_eq!(tree.get("a"), Some(&2));
    assert_eq!(tree.len(), 2);
}

#[test]
fn test_get_mut() {
    let mut tree = RadixTree::new();
    tree.insert("key", 1);

    if let Some(val) = tree.get_mut("key") {
        *val = 42;
    }

    assert_eq!(tree.get("key"), Some(&42));
}

#[test]
fn test_unicode_keys() {
    let mut tree = RadixTree::new();
    tree.insert("café", 1);
    tree.insert("日本語", 2);
    tree.insert("emoji🎉", 3);

    assert_eq!(tree.get("café"), Some(&1));
    assert_eq!(tree.get("日本語"), Some(&2));
    assert_eq!(tree.get("emoji🎉"), Some(&3));
}

#[test]
fn test_iteration() {
    let mut tree = RadixTree::new();
    tree.insert("a", 1);
    tree.insert("ab", 2);
    tree.insert("abc", 3);
    tree.insert("b", 4);

    let mut entries: Vec<_> = tree.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0], ("a".to_string(), &1));
    assert_eq!(entries[1], ("ab".to_string(), &2));
    assert_eq!(entries[2], ("abc".to_string(), &3));
    assert_eq!(entries[3], ("b".to_string(), &4));
}

#[test]
fn test_prefix_view() {
    let mut tree = RadixTree::new();
    tree.insert("apple", 1);
    tree.insert("application", 2);
    tree.insert("apply", 3);
    tree.insert("banana", 4);

    let view = tree.at_prefix("app").unwrap();
    let mut entries: Vec<_> = view.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].0, "apple");
    assert_eq!(entries[1].0, "application");
    assert_eq!(entries[2].0, "apply");
}

#[test]
fn test_prefix_view_not_found() {
    let mut tree = RadixTree::new();
    tree.insert("hello", 1);

    assert!(tree.at_prefix("world").is_none());
}

#[test]
fn test_prefix_view_exact_match() {
    let mut tree = RadixTree::new();
    tree.insert("test", 1);
    tree.insert("testing", 2);

    let view = tree.at_prefix("test").unwrap();
    let entries: Vec<_> = view.iter().collect();

    assert_eq!(entries.len(), 2);
}

#[test]
fn test_fuzzy_exact_match() {
    let mut tree = RadixTree::new();
    tree.insert("hello", 1);
    tree.insert("world", 2);

    let results = tree.fuzzy_search("hello", 0);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "hello");
    assert_eq!(results[0].1, &1);
    assert_eq!(results[0].2, 0); // distance
}

#[test]
fn test_fuzzy_one_edit() {
    let mut tree = RadixTree::new();
    tree.insert("hello", 1);
    tree.insert("hallo", 2);
    tree.insert("hullo", 3);
    tree.insert("world", 4);

    let results = tree.fuzzy_search("hello", 1);

    let keys: Vec<_> = results.iter().map(|(k, _, _)| k.as_str()).collect();
    assert!(keys.contains(&"hello"));
    assert!(keys.contains(&"hallo"));
    assert!(keys.contains(&"hullo"));
    assert!(!keys.contains(&"world"));
}

#[test]
fn test_fuzzy_insertion_deletion_substitution() {
    let mut tree = RadixTree::new();
    tree.insert("cat", 1);
    tree.insert("cats", 2);  // insertion
    tree.insert("ca", 3);    // deletion
    tree.insert("bat", 4);   // substitution
    tree.insert("car", 5);   // substitution

    let results = tree.fuzzy_search("cat", 1);
    assert!(results.len() >= 4);

    for (key, _, dist) in &results {
        assert!(*dist <= 1, "Key {} has distance {}", key, dist);
    }
}

#[test]
fn test_fuzzy_no_matches() {
    let mut tree = RadixTree::new();
    tree.insert("hello", 1);

    let results = tree.fuzzy_search("xyz", 1);
    assert!(results.is_empty());
}
