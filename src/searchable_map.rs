//! SearchableMap - wrapper around RadixTree for MiniSearch term storage

use crate::radix_tree::{RadixTree, PrefixView};
use std::collections::HashMap;

/// Term frequency data: field_id -> (doc_id -> frequency)
pub type FieldTermData = HashMap<u32, HashMap<u32, u32>>;

/// A searchable map for term storage, built on RadixTree.
#[derive(Debug, Clone)]
pub struct SearchableMap {
    tree: RadixTree<FieldTermData>,
}

impl SearchableMap {
    /// Creates a new empty searchable map.
    pub fn new() -> Self {
        SearchableMap {
            tree: RadixTree::new(),
        }
    }

    /// Returns true if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// Returns the number of terms in the map.
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    /// Gets or creates term data for the given term.
    pub fn get_or_create(&mut self, term: &str) -> &mut FieldTermData {
        if self.tree.get(term).is_none() {
            self.tree.insert(term, HashMap::new());
        }
        self.tree.get_mut(term).unwrap()
    }

    /// Gets term data if it exists.
    pub fn get(&self, term: &str) -> Option<&FieldTermData> {
        self.tree.get(term)
    }

    /// Gets mutable term data if it exists.
    pub fn get_mut(&mut self, term: &str) -> Option<&mut FieldTermData> {
        self.tree.get_mut(term)
    }

    /// Removes a term from the map.
    pub fn remove(&mut self, term: &str) -> Option<FieldTermData> {
        self.tree.remove(term)
    }

    /// Returns true if the map contains the term.
    pub fn contains_term(&self, term: &str) -> bool {
        self.tree.contains_key(term)
    }

    /// Returns an iterator over all terms and their data.
    pub fn iter(&self) -> impl Iterator<Item = (String, &FieldTermData)> {
        self.tree.iter()
    }

    /// Returns a view of terms with the given prefix.
    pub fn at_prefix(&self, prefix: &str) -> Option<PrefixView<'_, FieldTermData>> {
        self.tree.at_prefix(prefix)
    }

    /// Performs fuzzy search for terms.
    pub fn fuzzy_search(&self, query: &str, max_distance: usize) -> Vec<(String, &FieldTermData, usize)> {
        self.tree.fuzzy_search(query, max_distance)
    }
}

impl Default for SearchableMap {
    fn default() -> Self {
        Self::new()
    }
}
