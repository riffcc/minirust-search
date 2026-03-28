//! MiniSearch - main full-text search engine

use crate::searchable_map::{FieldTermData, SearchableMap};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

// === Callback Types ===

/// Tokenizer function: (text, field_name) -> tokens
pub type TokenizeFn = Box<dyn Fn(&str, &str) -> Vec<String> + Send + Sync>;

/// Term processor: (term, field_name) -> processed terms (empty to reject)
pub type ProcessTermFn = Box<dyn Fn(&str, &str) -> Vec<String> + Send + Sync>;

/// Result filter: (result) -> include?
pub type FilterFn = Box<dyn Fn(&SearchResult) -> bool + Send + Sync>;

/// Document booster: (id, term, stored_fields) -> boost factor (None to exclude)
pub type BoostDocumentFn =
    Box<dyn Fn(&str, &str, &HashMap<String, String>) -> Option<f64> + Send + Sync>;

// === Core Types ===

/// BM25+ scoring parameters.
#[derive(Debug, Clone)]
pub struct Bm25Params {
    pub k: f64,
    pub b: f64,
    pub d: f64,
}

impl Default for Bm25Params {
    fn default() -> Self {
        Bm25Params {
            k: 1.2,
            b: 0.7,
            d: 0.5,
        }
    }
}

/// How to combine multiple search terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CombineWith {
    #[default]
    Or,
    And,
    AndNot,
}

/// A query that can be a simple term or a combination of queries.
#[derive(Debug, Clone)]
pub enum Query {
    /// A simple term query.
    Term(String),
    /// Multiple queries combined.
    Combined {
        combine_with: CombineWith,
        queries: Vec<Query>,
    },
    /// Match all documents.
    Wildcard,
}

/// Options for creating a MiniSearch index.
pub struct MiniSearchOptions {
    /// Fields to index (required).
    pub fields: Vec<String>,
    /// Field containing document ID.
    pub id_field: String,
    /// Fields to store and return with results.
    pub store_fields: Vec<String>,
    /// Default search options.
    pub search_options: SearchOptions,
    /// Custom tokenizer function.
    pub tokenize: Option<TokenizeFn>,
    /// Custom term processor function.
    pub process_term: Option<ProcessTermFn>,
}

impl MiniSearchOptions {
    /// Create options with the given fields to index.
    pub fn new(fields: &[&str]) -> Self {
        MiniSearchOptions {
            fields: fields.iter().map(|s| s.to_string()).collect(),
            id_field: "id".to_string(),
            store_fields: Vec::new(),
            search_options: SearchOptions::new(),
            tokenize: None,
            process_term: None,
        }
    }
}

/// Options for search queries.
pub struct SearchOptions {
    /// Fields to search (empty = all fields).
    pub fields: Vec<String>,
    /// Field boost factors.
    pub boost: HashMap<String, f64>,
    /// Enable prefix matching.
    pub prefix: bool,
    /// Fuzzy matching: max edit distance.
    pub fuzzy: Option<usize>,
    /// Max edit distance for fuzzy when using fractional values.
    pub max_fuzzy: usize,
    /// How to combine multiple terms.
    pub combine_with: CombineWith,
    /// BM25 parameters.
    pub bm25: Bm25Params,
    /// Weights for match types.
    pub weights: MatchWeights,
    /// Filter function to exclude results.
    pub filter: Option<FilterFn>,
    /// Document boost function.
    pub boost_document: Option<BoostDocumentFn>,
    /// Custom tokenizer for search (overrides index tokenizer).
    pub tokenize: Option<TokenizeFn>,
    /// Custom term processor for search.
    pub process_term: Option<ProcessTermFn>,
}

impl SearchOptions {
    pub fn new() -> Self {
        SearchOptions {
            fields: Vec::new(),
            boost: HashMap::new(),
            prefix: false,
            fuzzy: None,
            max_fuzzy: 6,
            combine_with: CombineWith::Or,
            bm25: Bm25Params::default(),
            weights: MatchWeights::default(),
            filter: None,
            boost_document: None,
            tokenize: None,
            process_term: None,
        }
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Weights for different match types.
#[derive(Debug, Clone)]
pub struct MatchWeights {
    pub exact: f64,
    pub prefix: f64,
    pub fuzzy: f64,
}

impl Default for MatchWeights {
    fn default() -> Self {
        MatchWeights {
            exact: 1.0,
            prefix: 0.375,
            fuzzy: 0.45,
        }
    }
}

/// A search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Document ID.
    pub id: String,
    /// BM25+ score.
    pub score: f64,
    /// Terms from document that matched.
    pub terms: Vec<String>,
    /// Query terms that matched.
    pub query_terms: Vec<String>,
    /// Match info: term -> fields it matched in.
    pub match_info: HashMap<String, Vec<String>>,
    /// Stored fields.
    pub stored_fields: HashMap<String, String>,
}

/// Document trait for generic document support.
pub trait Document {
    /// Returns the document ID.
    fn id(&self) -> &str;
    /// Returns the value of a field.
    fn field(&self, name: &str) -> Option<&str>;
    /// Returns a stored field value.
    fn stored_field(&self, name: &str) -> Option<String> {
        self.field(name).map(|s| s.to_string())
    }
}

impl Document for HashMap<String, String> {
    fn id(&self) -> &str {
        self.get("id").map(|s| s.as_str()).unwrap_or("")
    }

    fn field(&self, name: &str) -> Option<&str> {
        self.get(name).map(|s| s.as_str())
    }
}

/// The main MiniSearch index.
pub struct MiniSearch {
    options: MiniSearchOptions,
    index: SearchableMap,
    document_ids: BTreeMap<u32, String>,
    id_to_short: HashMap<String, u32>,
    field_ids: HashMap<String, u32>,
    field_lengths: HashMap<u32, Vec<u32>>,
    avg_field_lengths: Vec<f64>,
    stored_fields: HashMap<u32, HashMap<String, String>>,
    next_id: u32,
    dirt_count: u32,
    /// Reverse index: doc short_id → set of terms it contributed.
    /// Enables O(terms_in_doc) removal instead of O(all_terms).
    doc_terms: HashMap<u32, HashSet<String>>,
}

impl MiniSearch {
    /// Create a new MiniSearch index with the given options.
    pub fn new(options: MiniSearchOptions) -> Self {
        let num_fields = options.fields.len();
        let field_ids: HashMap<String, u32> = options
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| (f.clone(), i as u32))
            .collect();

        MiniSearch {
            options,
            index: SearchableMap::new(),
            document_ids: BTreeMap::new(),
            id_to_short: HashMap::new(),
            field_ids,
            field_lengths: HashMap::new(),
            avg_field_lengths: vec![0.0; num_fields],
            stored_fields: HashMap::new(),
            next_id: 0,
            dirt_count: 0,
            doc_terms: HashMap::new(),
        }
    }

    // === Tokenization helpers ===

    fn tokenize(&self, text: &str, field: &str) -> Vec<String> {
        if let Some(ref tokenizer) = self.options.tokenize {
            tokenizer(text, field)
        } else {
            default_tokenize(text)
        }
    }

    fn process_term(&self, term: &str, field: &str) -> Vec<String> {
        if let Some(ref processor) = self.options.process_term {
            processor(term, field)
        } else {
            let processed = default_process_term(term);
            if processed.is_empty() {
                vec![]
            } else {
                vec![processed]
            }
        }
    }

    fn tokenize_for_search(&self, text: &str, opts: &SearchOptions) -> Vec<String> {
        if let Some(ref tokenizer) = opts.tokenize {
            tokenizer(text, "")
        } else {
            self.tokenize(text, "")
        }
    }

    fn process_term_for_search(&self, term: &str, opts: &SearchOptions) -> Vec<String> {
        if let Some(ref processor) = opts.process_term {
            processor(term, "")
        } else {
            self.process_term(term, "")
        }
    }

    // === Document Management ===

    /// Add a document to the index.
    pub fn add<D: Document>(&mut self, doc: D) {
        self.add_with_id_field(&doc, &self.options.id_field.clone());
    }

    fn add_with_id_field<D: Document>(&mut self, doc: &D, id_field: &str) {
        let doc_id = if id_field == "id" {
            doc.id().to_string()
        } else {
            doc.field(id_field)
                .expect("document must have id field")
                .to_string()
        };

        if self.id_to_short.contains_key(&doc_id) {
            panic!("duplicate document id: {}", doc_id);
        }

        let short_id = self.next_id;
        self.next_id += 1;

        self.document_ids.insert(short_id, doc_id.clone());
        self.id_to_short.insert(doc_id, short_id);

        // Store fields
        if !self.options.store_fields.is_empty() {
            let mut stored = HashMap::new();
            for field in &self.options.store_fields {
                if let Some(value) = doc.stored_field(field) {
                    stored.insert(field.clone(), value);
                }
            }
            self.stored_fields.insert(short_id, stored);
        }

        // Index each field
        let mut doc_field_lengths = vec![0u32; self.options.fields.len()];

        for field_name in &self.options.fields.clone() {
            let field_id = *self.field_ids.get(field_name).unwrap();
            if let Some(field_value) = doc.field(field_name) {
                let tokens = self.tokenize(field_value, field_name);
                let mut term_counts: HashMap<String, u32> = HashMap::new();

                for token in &tokens {
                    for processed in self.process_term(token, field_name) {
                        if !processed.is_empty() {
                            *term_counts.entry(processed).or_insert(0) += 1;
                        }
                    }
                }

                doc_field_lengths[field_id as usize] = term_counts.len() as u32;

                let doc_term_set = self.doc_terms.entry(short_id).or_default();
                for (term, freq) in term_counts {
                    doc_term_set.insert(term.clone());
                    let term_data = self.index.get_or_create(&term);
                    term_data
                        .entry(field_id)
                        .or_default()
                        .insert(short_id, freq);
                }
            }
        }

        self.field_lengths.insert(short_id, doc_field_lengths);
        self.update_avg_field_lengths();
    }

    /// Add multiple documents.
    pub fn add_all<D: Document>(&mut self, docs: Vec<D>) {
        for doc in docs {
            self.add(doc);
        }
    }

    /// Check if a document exists by ID.
    pub fn has(&self, id: &str) -> bool {
        self.id_to_short.contains_key(id)
    }

    /// Get stored fields for a document.
    pub fn get_stored_fields(&self, id: &str) -> Option<&HashMap<String, String>> {
        let short_id = self.id_to_short.get(id)?;
        self.stored_fields.get(short_id)
    }

    /// Number of indexed documents.
    pub fn document_count(&self) -> usize {
        self.document_ids.len()
    }

    /// Number of indexed terms.
    pub fn term_count(&self) -> usize {
        self.index.len()
    }

    /// Discard a document. Uses the reverse index to clean up only the
    /// terms this document contributed, so cost is O(terms_in_doc) not O(all_terms).
    pub fn discard(&mut self, id: &str) {
        if let Some(short_id) = self.id_to_short.remove(id) {
            self.document_ids.remove(&short_id);
            self.field_lengths.remove(&short_id);
            self.stored_fields.remove(&short_id);

            // Clean up term index using reverse index
            if let Some(terms) = self.doc_terms.remove(&short_id) {
                let terms_vec: Vec<String> = terms.into_iter().collect();
                for term in &terms_vec {
                    let is_empty = if let Some(term_data) = self.index.get_mut(term) {
                        // Remove this doc from all field entries
                        let field_ids: Vec<u32> = term_data.keys().copied().collect();
                        for fid in field_ids {
                            if let Some(doc_freqs) = term_data.get_mut(&fid) {
                                doc_freqs.remove(&short_id);
                                if doc_freqs.is_empty() {
                                    term_data.remove(&fid);
                                }
                            }
                        }
                        term_data.is_empty()
                    } else {
                        false
                    };
                    if is_empty {
                        self.index.remove(term);
                    }
                }
            } else {
                // No reverse index (legacy) — fall back to dirt tracking
                self.dirt_count += 1;
            }

            self.update_avg_field_lengths();
        }
    }

    /// Discard multiple documents.
    pub fn discard_all(&mut self, ids: &[&str]) {
        for id in ids {
            self.discard(id);
        }
    }

    /// Remove a document immediately. Now uses the reverse index via discard().
    pub fn remove<D: Document>(&mut self, doc: D) {
        self.discard(doc.id());
    }

    /// Replace a document (discard old, add new).
    pub fn replace<D: Document>(&mut self, doc: D) {
        let doc_id = doc.id().to_string();
        self.discard(&doc_id);
        self.add(doc);
    }

    // === Search ===

    /// Search the index with a string query.
    pub fn search(&self, query: &str, options: Option<SearchOptions>) -> Vec<SearchResult> {
        let opts = options.unwrap_or_else(|| SearchOptions::new());

        let query_tokens = self.tokenize_for_search(query, &opts);
        let mut query_terms: Vec<String> = Vec::new();

        for token in query_tokens {
            for processed in self.process_term_for_search(&token, &opts) {
                if !processed.is_empty() {
                    query_terms.push(processed);
                }
            }
        }

        if query_terms.is_empty() {
            return Vec::new();
        }

        // Build query tree from terms
        let term_queries: Vec<Query> = query_terms.iter().map(|t| Query::Term(t.clone())).collect();

        let query = Query::Combined {
            combine_with: opts.combine_with,
            queries: term_queries,
        };

        self.search_query(query, Some(opts))
    }

    /// Search with a Query tree.
    pub fn search_query(&self, query: Query, options: Option<SearchOptions>) -> Vec<SearchResult> {
        let opts = options.unwrap_or_else(|| SearchOptions::new());
        let combined = self.execute_query(&query, &opts);

        self.build_results(combined, &opts)
    }

    /// Search with wildcard (match all documents).
    pub fn search_wildcard(&self, options: Option<SearchOptions>) -> Vec<SearchResult> {
        self.search_query(Query::Wildcard, options)
    }

    fn execute_query(&self, query: &Query, opts: &SearchOptions) -> HashMap<u32, TermMatch> {
        match query {
            Query::Term(term) => self.collect_term_matches(term, opts),
            Query::Wildcard => self.collect_wildcard_matches(opts),
            Query::Combined {
                combine_with,
                queries,
            } => {
                let sub_results: Vec<HashMap<u32, TermMatch>> = queries
                    .iter()
                    .map(|q| self.execute_query(q, opts))
                    .filter(|r| !r.is_empty() || *combine_with == CombineWith::And)
                    .collect();

                if sub_results.is_empty() {
                    return HashMap::new();
                }

                self.combine_results(sub_results, *combine_with)
            }
        }
    }

    fn collect_wildcard_matches(&self, opts: &SearchOptions) -> HashMap<u32, TermMatch> {
        let mut matches: HashMap<u32, TermMatch> = HashMap::new();

        for (short_id, doc_id) in &self.document_ids {
            let stored = self
                .stored_fields
                .get(short_id)
                .cloned()
                .unwrap_or_default();

            // Apply document boost if present
            let boost = if let Some(ref boost_fn) = opts.boost_document {
                match boost_fn(doc_id, "*", &stored) {
                    Some(b) => b,
                    None => continue, // Exclude this document
                }
            } else {
                1.0
            };

            matches.insert(
                *short_id,
                TermMatch {
                    score: boost,
                    terms: HashSet::new(),
                    query_terms: HashSet::from(["*".to_string()]),
                    match_info: HashMap::new(),
                },
            );
        }

        matches
    }

    fn collect_term_matches(&self, query_term: &str, opts: &SearchOptions) -> HashMap<u32, TermMatch> {
        let mut matches: HashMap<u32, TermMatch> = HashMap::new();

        // Exact matches
        if let Some(term_data) = self.index.get(query_term) {
            self.add_matches_from_term_data(
                query_term,
                query_term,
                term_data,
                opts,
                opts.weights.exact,
                &mut matches,
            );
        }

        // Prefix matches
        if opts.prefix {
            if let Some(prefix_view) = self.index.at_prefix(query_term) {
                for (term, term_data) in prefix_view.iter() {
                    if term != query_term {
                        let distance = term.len() - query_term.len();
                        let weight = opts.weights.prefix * query_term.len() as f64
                            / (query_term.len() as f64 + 0.3 * distance as f64);
                        self.add_matches_from_term_data(
                            &term,
                            query_term,
                            term_data,
                            opts,
                            weight,
                            &mut matches,
                        );
                    }
                }
            }
        }

        // Fuzzy matches
        if let Some(max_dist) = opts.fuzzy {
            let fuzzy_results = self.index.fuzzy_search(query_term, max_dist);
            for (term, term_data, distance) in fuzzy_results {
                if distance > 0 {
                    let weight = opts.weights.fuzzy * query_term.len() as f64
                        / (query_term.len() as f64 + distance as f64);
                    self.add_matches_from_term_data(
                        &term,
                        query_term,
                        term_data,
                        opts,
                        weight,
                        &mut matches,
                    );
                }
            }
        }

        matches
    }

    fn add_matches_from_term_data(
        &self,
        term: &str,
        query_term: &str,
        term_data: &FieldTermData,
        opts: &SearchOptions,
        weight: f64,
        matches: &mut HashMap<u32, TermMatch>,
    ) {
        let n = self.document_count() as f64;

        for (&field_id, doc_freqs) in term_data {
            let field_name = self.field_name(field_id);

            if !opts.fields.is_empty() && !opts.fields.contains(&field_name) {
                continue;
            }

            let field_boost = opts.boost.get(&field_name).copied().unwrap_or(1.0);
            let df = doc_freqs.len() as f64;
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

            for (&short_id, &tf) in doc_freqs {
                if !self.document_ids.contains_key(&short_id) {
                    continue;
                }

                let doc_id = self.document_ids.get(&short_id).unwrap();
                let stored = self
                    .stored_fields
                    .get(&short_id)
                    .cloned()
                    .unwrap_or_default();

                // Apply document boost
                let doc_boost = if let Some(ref boost_fn) = opts.boost_document {
                    match boost_fn(doc_id, term, &stored) {
                        Some(b) => b,
                        None => continue, // Exclude
                    }
                } else {
                    1.0
                };

                let field_length = self
                    .field_lengths
                    .get(&short_id)
                    .map(|fl| fl[field_id as usize] as f64)
                    .unwrap_or(1.0);
                let avg_length = self.avg_field_lengths[field_id as usize].max(1.0);

                let bm25_score = self.bm25(tf as f64, idf, field_length, avg_length, &opts.bm25);
                let score = bm25_score * weight * field_boost * doc_boost;

                let doc_match = matches.entry(short_id).or_insert_with(|| TermMatch {
                    score: 0.0,
                    terms: HashSet::new(),
                    query_terms: HashSet::new(),
                    match_info: HashMap::new(),
                });

                doc_match.score += score;
                doc_match.terms.insert(term.to_string());
                doc_match.query_terms.insert(query_term.to_string());
                doc_match
                    .match_info
                    .entry(term.to_string())
                    .or_default()
                    .push(field_name.clone());
            }
        }
    }

    fn bm25(
        &self,
        tf: f64,
        idf: f64,
        field_length: f64,
        avg_length: f64,
        params: &Bm25Params,
    ) -> f64 {
        let k = params.k;
        let b = params.b;
        let d = params.d;

        let length_norm = 1.0 - b + b * (field_length / avg_length);
        idf * (d + (tf * (k + 1.0)) / (tf + k * length_norm))
    }

    fn combine_results(
        &self,
        term_results: Vec<HashMap<u32, TermMatch>>,
        combine: CombineWith,
    ) -> HashMap<u32, TermMatch> {
        if term_results.is_empty() {
            return HashMap::new();
        }

        match combine {
            CombineWith::Or => {
                let mut combined: HashMap<u32, TermMatch> = HashMap::new();
                for term_result in term_results {
                    for (doc_id, term_match) in term_result {
                        combined
                            .entry(doc_id)
                            .and_modify(|existing| {
                                existing.score += term_match.score;
                                existing.terms.extend(term_match.terms.clone());
                                existing.query_terms.extend(term_match.query_terms.clone());
                                for (term, fields) in &term_match.match_info {
                                    existing
                                        .match_info
                                        .entry(term.clone())
                                        .or_default()
                                        .extend(fields.clone());
                                }
                            })
                            .or_insert(term_match);
                    }
                }
                combined
            }
            CombineWith::And => {
                let mut iter = term_results.into_iter();
                let mut combined = iter.next().unwrap();
                for term_result in iter {
                    combined.retain(|doc_id, _| term_result.contains_key(doc_id));
                    for (doc_id, term_match) in term_result {
                        if let Some(existing) = combined.get_mut(&doc_id) {
                            existing.score += term_match.score;
                            existing.terms.extend(term_match.terms);
                            existing.query_terms.extend(term_match.query_terms);
                            for (term, fields) in term_match.match_info {
                                existing.match_info.entry(term).or_default().extend(fields);
                            }
                        }
                    }
                }
                combined
            }
            CombineWith::AndNot => {
                let mut iter = term_results.into_iter();
                let mut combined = iter.next().unwrap();
                for term_result in iter {
                    for doc_id in term_result.keys() {
                        combined.remove(doc_id);
                    }
                }
                combined
            }
        }
    }

    fn build_results(
        &self,
        combined: HashMap<u32, TermMatch>,
        opts: &SearchOptions,
    ) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = combined
            .into_iter()
            .filter_map(|(short_id, doc_match)| {
                let doc_id = self.document_ids.get(&short_id)?;
                let stored = self
                    .stored_fields
                    .get(&short_id)
                    .cloned()
                    .unwrap_or_default();

                let result = SearchResult {
                    id: doc_id.clone(),
                    score: doc_match.score,
                    terms: doc_match.terms.into_iter().collect(),
                    query_terms: doc_match.query_terms.into_iter().collect(),
                    match_info: doc_match.match_info,
                    stored_fields: stored,
                };

                // Apply filter
                if let Some(ref filter) = opts.filter {
                    if !filter(&result) {
                        return None;
                    }
                }

                Some(result)
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results
    }

    fn field_name(&self, field_id: u32) -> String {
        self.field_ids
            .iter()
            .find(|(_, id)| **id == field_id)
            .map(|(name, _)| name.clone())
            .unwrap_or_default()
    }

    fn update_avg_field_lengths(&mut self) {
        let doc_count = self.document_ids.len() as f64;
        if doc_count == 0.0 {
            self.avg_field_lengths.fill(0.0);
            return;
        }

        let num_fields = self.options.fields.len();
        let mut totals = vec![0u64; num_fields];

        for (short_id, _) in &self.document_ids {
            if let Some(lengths) = self.field_lengths.get(short_id) {
                for (i, &len) in lengths.iter().enumerate() {
                    totals[i] += len as u64;
                }
            }
        }

        for (i, total) in totals.iter().enumerate() {
            self.avg_field_lengths[i] = *total as f64 / doc_count;
        }
    }

    // === Auto-suggest ===

    /// Generate search suggestions for a partial query.
    pub fn auto_suggest(&self, query: &str, options: Option<SearchOptions>) -> Vec<Suggestion> {
        let mut opts = options.unwrap_or_else(|| SearchOptions::new());
        opts.combine_with = CombineWith::And;
        opts.prefix = true;

        let results = self.search(query, Some(opts));

        let mut suggestion_map: HashMap<Vec<String>, (f64, usize)> = HashMap::new();

        for result in results {
            let mut terms: Vec<String> = result.terms.clone();
            terms.sort();

            let entry = suggestion_map.entry(terms).or_insert((0.0, 0));
            entry.0 += result.score;
            entry.1 += 1;
        }

        let mut suggestions: Vec<Suggestion> = suggestion_map
            .into_iter()
            .map(|(terms, (total_score, count))| {
                let suggestion = terms.join(" ");
                Suggestion {
                    suggestion,
                    terms,
                    score: total_score / count as f64,
                }
            })
            .collect();

        suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        suggestions
    }

    // === Vacuum ===

    pub fn dirt_count(&self) -> u32 {
        self.dirt_count
    }

    pub fn dirt_factor(&self) -> f64 {
        let total = self.document_ids.len() as f64 + self.dirt_count as f64;
        if total == 0.0 {
            0.0
        } else {
            self.dirt_count as f64 / total
        }
    }

    pub fn is_vacuuming(&self) -> bool {
        false
    }

    pub fn vacuum(&mut self) {
        let mut empty_terms: Vec<String> = Vec::new();

        for (term, term_data) in self.index.iter() {
            let mut is_empty = true;
            for (_, doc_freqs) in term_data.iter() {
                for (short_id, _) in doc_freqs.iter() {
                    if self.document_ids.contains_key(short_id) {
                        is_empty = false;
                        break;
                    }
                }
                if !is_empty {
                    break;
                }
            }
            if is_empty {
                empty_terms.push(term.clone());
            }
        }

        for term in empty_terms {
            self.index.remove(&term);
        }

        self.dirt_count = 0;
    }

    // === Serialization ===

    pub fn to_json(&self) -> String {
        let serializable = SerializedIndex {
            document_count: self.document_ids.len(),
            next_id: self.next_id,
            document_ids: self
                .document_ids
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            field_ids: self.field_ids.clone(),
            field_length: self
                .field_lengths
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            average_field_length: self.avg_field_lengths.clone(),
            stored_fields: self
                .stored_fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            dirt_count: self.dirt_count,
            index: self.serialize_index(),
            serialization_version: 2,
        };

        serde_json::to_string(&serializable).unwrap()
    }

    pub fn load_json(json: &str, options: MiniSearchOptions) -> Result<Self, String> {
        let serialized: SerializedIndex =
            serde_json::from_str(json).map_err(|e| e.to_string())?;

        let mut ms = MiniSearch::new(options);

        for (short_id_str, doc_id) in serialized.document_ids {
            let short_id: u32 = short_id_str.parse().map_err(|_| "invalid short id")?;
            ms.document_ids.insert(short_id, doc_id.clone());
            ms.id_to_short.insert(doc_id, short_id);
        }

        ms.next_id = serialized.next_id;
        ms.dirt_count = serialized.dirt_count;
        ms.avg_field_lengths = serialized.average_field_length;

        for (short_id_str, lengths) in serialized.field_length {
            let short_id: u32 = short_id_str.parse().map_err(|_| "invalid short id")?;
            ms.field_lengths.insert(short_id, lengths);
        }

        for (short_id_str, fields) in serialized.stored_fields {
            let short_id: u32 = short_id_str.parse().map_err(|_| "invalid short id")?;
            ms.stored_fields.insert(short_id, fields);
        }

        for (term, field_data) in serialized.index {
            let term_entry = ms.index.get_or_create(&term);
            for (field_id_str, doc_freqs) in field_data {
                let field_id: u32 = field_id_str.parse().map_err(|_| "invalid field id")?;
                let field_entry = term_entry.entry(field_id).or_default();
                for (doc_id_str, freq) in doc_freqs {
                    let doc_id: u32 = doc_id_str.parse().map_err(|_| "invalid doc id")?;
                    field_entry.insert(doc_id, freq);
                }
            }
        }

        Ok(ms)
    }

    fn serialize_index(&self) -> Vec<(String, HashMap<String, HashMap<String, u32>>)> {
        self.index
            .iter()
            .map(|(term, term_data)| {
                let field_data: HashMap<String, HashMap<String, u32>> = term_data
                    .iter()
                    .map(|(field_id, doc_freqs)| {
                        let doc_data: HashMap<String, u32> = doc_freqs
                            .iter()
                            .map(|(doc_id, freq)| (doc_id.to_string(), *freq))
                            .collect();
                        (field_id.to_string(), doc_data)
                    })
                    .collect();
                (term.clone(), field_data)
            })
            .collect()
    }
}

/// A search suggestion.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub suggestion: String,
    pub terms: Vec<String>,
    pub score: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedIndex {
    document_count: usize,
    next_id: u32,
    document_ids: HashMap<String, String>,
    field_ids: HashMap<String, u32>,
    field_length: HashMap<String, Vec<u32>>,
    average_field_length: Vec<f64>,
    stored_fields: HashMap<String, HashMap<String, String>>,
    dirt_count: u32,
    index: Vec<(String, HashMap<String, HashMap<String, u32>>)>,
    serialization_version: u32,
}

struct TermMatch {
    score: f64,
    terms: HashSet<String>,
    query_terms: HashSet<String>,
    match_info: HashMap<String, Vec<String>>,
}

fn default_tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn default_process_term(term: &str) -> String {
    term.to_lowercase()
}
