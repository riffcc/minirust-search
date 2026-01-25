//! Radix Tree (compressed prefix tree) implementation

use std::collections::BTreeMap;

/// A radix tree node storing values at string keys with path compression.
#[derive(Debug, Clone)]
pub struct RadixTree<T> {
    value: Option<T>,
    children: BTreeMap<String, RadixTree<T>>,
}

impl<T> RadixTree<T> {
    /// Creates a new empty radix tree.
    pub fn new() -> Self {
        RadixTree {
            value: None,
            children: BTreeMap::new(),
        }
    }

    /// Returns true if the tree contains no entries.
    pub fn is_empty(&self) -> bool {
        self.value.is_none() && self.children.is_empty()
    }

    /// Returns the number of entries in the tree.
    pub fn len(&self) -> usize {
        let mut count = if self.value.is_some() { 1 } else { 0 };
        for child in self.children.values() {
            count += child.len();
        }
        count
    }

    /// Inserts a key-value pair, returning the old value if key existed.
    pub fn insert(&mut self, key: &str, value: T) -> Option<T> {
        if key.is_empty() {
            return self.value.replace(value);
        }

        // Find edge sharing a prefix with key
        let mut matching_edge: Option<String> = None;
        let mut match_len = 0;

        for edge in self.children.keys() {
            let common = common_prefix_len(edge, key);
            if common > 0 {
                matching_edge = Some(edge.clone());
                match_len = common;
                break;
            }
        }

        match matching_edge {
            None => {
                // No matching edge - create new child
                let mut child = RadixTree::new();
                child.value = Some(value);
                self.children.insert(key.to_string(), child);
                None
            }
            Some(edge) => {
                if match_len == edge.len() && match_len == key.len() {
                    // Exact match - replace value in child
                    let child = self.children.get_mut(&edge).unwrap();
                    child.value.replace(value)
                } else if match_len == edge.len() {
                    // Edge is prefix of key - recurse
                    let child = self.children.get_mut(&edge).unwrap();
                    child.insert(&key[match_len..], value)
                } else if match_len == key.len() {
                    // Key is prefix of edge - split edge
                    let old_child = self.children.remove(&edge).unwrap();
                    let suffix = edge[match_len..].to_string();

                    let mut new_node = RadixTree::new();
                    new_node.value = Some(value);
                    new_node.children.insert(suffix, old_child);
                    self.children.insert(key.to_string(), new_node);
                    None
                } else {
                    // Partial match - split edge
                    let old_child = self.children.remove(&edge).unwrap();
                    let prefix = edge[..match_len].to_string();
                    let edge_suffix = edge[match_len..].to_string();
                    let key_suffix = key[match_len..].to_string();

                    let mut intermediate = RadixTree::new();
                    intermediate.children.insert(edge_suffix, old_child);

                    let mut new_leaf = RadixTree::new();
                    new_leaf.value = Some(value);
                    intermediate.children.insert(key_suffix, new_leaf);

                    self.children.insert(prefix, intermediate);
                    None
                }
            }
        }
    }

    /// Gets a reference to the value for key.
    pub fn get(&self, key: &str) -> Option<&T> {
        if key.is_empty() {
            return self.value.as_ref();
        }

        for (edge, child) in &self.children {
            let common = common_prefix_len(edge, key);
            if common == edge.len() && common == key.len() {
                return child.value.as_ref();
            } else if common == edge.len() {
                return child.get(&key[common..]);
            }
        }
        None
    }

    /// Gets a mutable reference to the value for key.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut T> {
        if key.is_empty() {
            return self.value.as_mut();
        }

        for (edge, child) in &mut self.children {
            let common = common_prefix_len(edge, key);
            if common == edge.len() && common == key.len() {
                return child.value.as_mut();
            } else if common == edge.len() {
                return child.get_mut(&key[common..]);
            }
        }
        None
    }

    /// Returns true if the tree contains key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Removes key from tree, returning value if it existed.
    pub fn remove(&mut self, key: &str) -> Option<T> {
        if key.is_empty() {
            return self.value.take();
        }

        let mut matching_edge: Option<String> = None;
        for edge in self.children.keys() {
            let common = common_prefix_len(edge, key);
            if common == edge.len() {
                matching_edge = Some(edge.clone());
                break;
            }
        }

        let edge = matching_edge?;
        let edge_len = edge.len();
        let child = self.children.get_mut(&edge)?;

        let result = if edge_len == key.len() {
            child.value.take()
        } else {
            child.remove(&key[edge_len..])
        };

        // Cleanup: remove empty children, merge single-child nodes
        if result.is_some() {
            let child = self.children.get(&edge).unwrap();
            if child.value.is_none() && child.children.is_empty() {
                self.children.remove(&edge);
            } else if child.value.is_none() && child.children.len() == 1 {
                let child = self.children.remove(&edge).unwrap();
                let (child_edge, grandchild) = child.children.into_iter().next().unwrap();
                let merged_edge = format!("{}{}", edge, child_edge);
                self.children.insert(merged_edge, grandchild);
            }
        }

        result
    }

    /// Returns iterator over all (key, value) pairs.
    pub fn iter(&self) -> RadixTreeIter<'_, T> {
        RadixTreeIter::new(self)
    }

    /// Returns a view of the subtree at prefix.
    pub fn at_prefix(&self, prefix: &str) -> Option<PrefixView<'_, T>> {
        self.at_prefix_internal(prefix, String::new())
    }

    fn at_prefix_internal(&self, prefix: &str, accumulated: String) -> Option<PrefixView<'_, T>> {
        if prefix.is_empty() {
            return Some(PrefixView {
                node: self,
                key_prefix: accumulated,
            });
        }

        for (edge, child) in &self.children {
            let common = common_prefix_len(edge, prefix);
            if common == 0 {
                continue;
            }

            if common == edge.len() {
                // Full edge match, continue into child
                let new_acc = format!("{}{}", accumulated, edge);
                return child.at_prefix_internal(&prefix[common..], new_acc);
            } else if common == prefix.len() {
                // Prefix exhausted mid-edge
                let new_acc = format!("{}{}", accumulated, edge);
                return Some(PrefixView {
                    node: child,
                    key_prefix: new_acc,
                });
            }
        }
        None
    }

    /// Fuzzy search: find all entries within max_distance edits.
    /// Returns Vec of (key, value_ref, distance).
    pub fn fuzzy_search(&self, query: &str, max_distance: usize) -> Vec<(String, &T, usize)> {
        let mut results = Vec::new();
        let query_chars: Vec<char> = query.chars().collect();
        let initial_row: Vec<usize> = (0..=query_chars.len()).collect();

        self.fuzzy_internal(&query_chars, max_distance, String::new(), initial_row, &mut results);
        results
    }

    fn fuzzy_internal<'a>(
        &'a self,
        query: &[char],
        max_distance: usize,
        current_key: String,
        prev_row: Vec<usize>,
        results: &mut Vec<(String, &'a T, usize)>,
    ) {
        if let Some(ref value) = self.value {
            let distance = prev_row[query.len()];
            if distance <= max_distance {
                results.push((current_key.clone(), value, distance));
            }
        }

        for (edge, child) in &self.children {
            let mut row = prev_row.clone();

            let mut min_in_row = usize::MAX;
            for ch in edge.chars() {
                let mut new_row = vec![row[0] + 1];
                min_in_row = new_row[0];

                for (j, &query_char) in query.iter().enumerate() {
                    let cost = if ch == query_char { 0 } else { 1 };
                    let val = (row[j] + cost)
                        .min(row[j + 1] + 1)
                        .min(new_row[j] + 1);
                    new_row.push(val);
                    min_in_row = min_in_row.min(val);
                }
                row = new_row;
            }

            if min_in_row <= max_distance {
                let new_key = format!("{}{}", current_key, edge);
                child.fuzzy_internal(query, max_distance, new_key, row, results);
            }
        }
    }
}

impl<T> Default for RadixTree<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// View into a subtree at a prefix.
pub struct PrefixView<'a, T> {
    node: &'a RadixTree<T>,
    key_prefix: String,
}

impl<'a, T> PrefixView<'a, T> {
    /// Iterate over entries in this prefix subtree.
    pub fn iter(&self) -> impl Iterator<Item = (String, &'a T)> {
        let prefix = self.key_prefix.clone();
        // Check if node itself has a value (exact prefix match)
        let self_entry = if let Some(ref val) = self.node.value {
            Some((prefix.clone(), val))
        } else {
            None
        };

        self_entry.into_iter().chain(
            self.node.iter().map(move |(k, v)| {
                (format!("{}{}", prefix, k), v)
            })
        )
    }
}

/// Iterator over radix tree entries.
pub struct RadixTreeIter<'a, T> {
    stack: Vec<(String, &'a RadixTree<T>, std::collections::btree_map::Iter<'a, String, RadixTree<T>>)>,
}

impl<'a, T> RadixTreeIter<'a, T> {
    fn new(root: &'a RadixTree<T>) -> Self {
        let mut iter = RadixTreeIter { stack: Vec::new() };
        iter.stack.push((String::new(), root, root.children.iter()));
        iter
    }
}

impl<'a, T> Iterator for RadixTreeIter<'a, T> {
    type Item = (String, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((prefix, node, mut children)) = self.stack.pop() {
            if let Some((edge, child)) = children.next() {
                // Put current back with remaining children
                self.stack.push((prefix.clone(), node, children));

                let new_key = format!("{}{}", prefix, edge);

                // Push child for later exploration
                self.stack.push((new_key.clone(), child, child.children.iter()));

                // If child has value, return it
                if let Some(ref value) = child.value {
                    return Some((new_key, value));
                }
            }
            // No more children at this level, continue popping
        }
        None
    }
}

/// Returns the byte length of the common prefix between two strings.
fn common_prefix_len(a: &str, b: &str) -> usize {
    let mut len = 0;
    for (ca, cb) in a.chars().zip(b.chars()) {
        if ca != cb {
            break;
        }
        len += ca.len_utf8();
    }
    len
}
