//! MiniRust Search CLI

use clap::{Parser, Subcommand};
use minirust_search::{MiniSearch, MiniSearchOptions, SearchOptions, CombineWith};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};

#[derive(Parser)]
#[command(name = "mrsearch")]
#[command(about = "MiniRust Search - A Rust port of MiniSearch")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build an index from JSONL input
    Index {
        /// Input file (JSONL format, - for stdin)
        #[arg(short, long, default_value = "-")]
        input: String,

        /// Output index file
        #[arg(short, long)]
        output: String,

        /// Fields to index (comma-separated)
        #[arg(short, long)]
        fields: String,

        /// Fields to store (comma-separated, optional)
        #[arg(short, long)]
        store: Option<String>,

        /// ID field name
        #[arg(long, default_value = "id")]
        id_field: String,
    },

    /// Search an index
    Search {
        /// Index file to search
        #[arg(short, long)]
        index: String,

        /// Query string
        #[arg(short, long)]
        query: String,

        /// Fields to search (comma-separated, optional)
        #[arg(short, long)]
        fields: Option<String>,

        /// Enable prefix matching
        #[arg(short, long)]
        prefix: bool,

        /// Fuzzy matching max edit distance
        #[arg(long)]
        fuzzy: Option<usize>,

        /// Combine mode: or, and, and_not
        #[arg(long, default_value = "or")]
        combine: String,

        /// Maximum results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Output format: json, pretty, compact
        #[arg(long, default_value = "pretty")]
        format: String,
    },

    /// Interactive search REPL
    Repl {
        /// Index file to search
        #[arg(short, long)]
        index: String,

        /// Enable prefix matching
        #[arg(short, long)]
        prefix: bool,

        /// Fuzzy matching max edit distance
        #[arg(long)]
        fuzzy: Option<usize>,
    },

    /// Get auto-suggestions
    Suggest {
        /// Index file
        #[arg(short, long)]
        index: String,

        /// Partial query
        #[arg(short, long)]
        query: String,

        /// Maximum suggestions
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },

    /// Show index statistics
    Stats {
        /// Index file
        #[arg(short, long)]
        index: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index {
            input,
            output,
            fields,
            store,
            id_field,
        } => cmd_index(&input, &output, &fields, store.as_deref(), &id_field),

        Commands::Search {
            index,
            query,
            fields,
            prefix,
            fuzzy,
            combine,
            limit,
            format,
        } => cmd_search(&index, &query, fields.as_deref(), prefix, fuzzy, &combine, limit, &format),

        Commands::Repl {
            index,
            prefix,
            fuzzy,
        } => cmd_repl(&index, prefix, fuzzy),

        Commands::Suggest {
            index,
            query,
            limit,
        } => cmd_suggest(&index, &query, limit),

        Commands::Stats { index } => cmd_stats(&index),
    }
}

fn cmd_index(input: &str, output: &str, fields: &str, store: Option<&str>, id_field: &str) {
    let field_list: Vec<&str> = fields.split(',').map(|s| s.trim()).collect();
    let mut options = MiniSearchOptions::new(&field_list);
    options.id_field = id_field.to_string();

    if let Some(store_fields) = store {
        options.store_fields = store_fields
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
    }

    let mut ms = MiniSearch::new(options);
    let mut count = 0;

    let reader: Box<dyn BufRead> = if input == "-" {
        Box::new(io::stdin().lock())
    } else {
        let file = fs::File::open(input).expect("Failed to open input file");
        Box::new(io::BufReader::new(file))
    };

    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        if line.trim().is_empty() {
            continue;
        }

        let value: Value = serde_json::from_str(&line).expect("Invalid JSON");
        if let Value::Object(obj) = value {
            let doc: HashMap<String, String> = obj
                .into_iter()
                .filter_map(|(k, v)| {
                    let val = match v {
                        Value::String(s) => s,
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => return None,
                    };
                    Some((k, val))
                })
                .collect();

            ms.add(doc);
            count += 1;
        }
    }

    let json = ms.to_json();
    fs::write(output, json).expect("Failed to write output file");

    eprintln!("Indexed {} documents", count);
    eprintln!("Output: {}", output);
}

fn cmd_search(
    index_path: &str,
    query: &str,
    fields: Option<&str>,
    prefix: bool,
    fuzzy: Option<usize>,
    combine: &str,
    limit: usize,
    format: &str,
) {
    let json = fs::read_to_string(index_path).expect("Failed to read index file");

    // We need to reconstruct options - for now just use empty fields
    // The loaded index has the field data
    let options = MiniSearchOptions::new(&[]);
    let ms = MiniSearch::load_json(&json, options).expect("Failed to load index");

    let mut search_opts = SearchOptions::default();
    search_opts.prefix = prefix;
    search_opts.fuzzy = fuzzy;
    search_opts.combine_with = match combine {
        "and" => CombineWith::And,
        "and_not" => CombineWith::AndNot,
        _ => CombineWith::Or,
    };

    if let Some(f) = fields {
        search_opts.fields = f.split(',').map(|s| s.trim().to_string()).collect();
    }

    let results = ms.search(query, Some(search_opts));
    let results: Vec<_> = results.into_iter().take(limit).collect();

    match format {
        "json" => {
            let output: Vec<_> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "score": r.score,
                        "terms": r.terms,
                        "stored": r.stored_fields,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string(&output).unwrap());
        }
        "compact" => {
            for r in &results {
                println!("{}\t{:.4}", r.id, r.score);
            }
        }
        _ => {
            println!("Found {} results:", results.len());
            println!();
            for r in &results {
                println!("ID: {}", r.id);
                println!("Score: {:.4}", r.score);
                if !r.terms.is_empty() {
                    println!("Matched terms: {}", r.terms.join(", "));
                }
                if !r.stored_fields.is_empty() {
                    for (k, v) in &r.stored_fields {
                        let display = if v.len() > 100 {
                            format!("{}...", &v[..100])
                        } else {
                            v.clone()
                        };
                        println!("  {}: {}", k, display);
                    }
                }
                println!();
            }
        }
    }
}

fn cmd_repl(index_path: &str, prefix: bool, fuzzy: Option<usize>) {
    let json = fs::read_to_string(index_path).expect("Failed to read index file");
    let options = MiniSearchOptions::new(&[]);
    let ms = MiniSearch::load_json(&json, options).expect("Failed to load index");

    eprintln!("MiniRust Search REPL");
    eprintln!("Documents: {}", ms.document_count());
    eprintln!("Terms: {}", ms.term_count());
    eprintln!("Type query and press Enter. Ctrl+D to exit.");
    eprintln!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush().unwrap();

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }

        let query = line.trim();
        if query.is_empty() {
            continue;
        }

        let mut search_opts = SearchOptions::default();
        search_opts.prefix = prefix;
        search_opts.fuzzy = fuzzy;

        let results = ms.search(query, Some(search_opts));

        println!("Found {} results:", results.len());
        for r in results.iter().take(10) {
            print!("  {} (score: {:.4})", r.id, r.score);
            if !r.stored_fields.is_empty() {
                if let Some(title) = r.stored_fields.get("title") {
                    let display = if title.len() > 50 {
                        format!("{}...", &title[..50])
                    } else {
                        title.clone()
                    };
                    print!(" - {}", display);
                }
            }
            println!();
        }
        println!();
    }
}

fn cmd_suggest(index_path: &str, query: &str, limit: usize) {
    let json = fs::read_to_string(index_path).expect("Failed to read index file");
    let options = MiniSearchOptions::new(&[]);
    let ms = MiniSearch::load_json(&json, options).expect("Failed to load index");

    let mut search_opts = SearchOptions::default();
    search_opts.prefix = true;

    let suggestions = ms.auto_suggest(query, Some(search_opts));

    for (i, s) in suggestions.iter().take(limit).enumerate() {
        println!("{}. {} (score: {:.4})", i + 1, s.suggestion, s.score);
    }
}

fn cmd_stats(index_path: &str) {
    let json = fs::read_to_string(index_path).expect("Failed to read index file");
    let options = MiniSearchOptions::new(&[]);
    let ms = MiniSearch::load_json(&json, options).expect("Failed to load index");

    println!("Index Statistics:");
    println!("  Documents: {}", ms.document_count());
    println!("  Terms: {}", ms.term_count());
    println!("  Dirt count: {}", ms.dirt_count());
    println!("  Dirt factor: {:.2}%", ms.dirt_factor() * 100.0);
}
