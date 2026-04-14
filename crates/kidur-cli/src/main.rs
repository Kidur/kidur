use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use kidur_core::{FieldType, FieldValue, Node, NodeId, SupertagDef};
use kidur_log::{Mutation, MutationLog};
use kidur_supertag::SupertagRegistry;

#[derive(Parser)]
#[command(name = "kidur", version, about = "Kidur — local-first outliner substrate")]
struct Cli {
    /// Data directory (contains kidur.jsonl + supertags/)
    #[arg(long, env = "KIDUR_DATA", default_value = ".")]
    data_dir: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new data directory with supertag definitions
    Init,

    /// Add a node to the log
    Add {
        /// Node content (main text)
        content: String,

        /// Supertag / content type (e.g. handwritten_note, quest, person)
        #[arg(long = "type", short = 't')]
        node_type: Option<String>,

        /// Field values as key=value (repeatable). Type inferred from supertag schema.
        #[arg(long = "field", short = 'f')]
        fields: Vec<String>,

        /// Parent node ID (UUID)
        #[arg(long)]
        parent: Option<String>,
    },

    /// List nodes from the log
    List {
        /// Filter by supertag
        #[arg(long = "type", short = 't')]
        node_type: Option<String>,

        /// Max entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

// --- Embedded supertag definitions (written by `init`) ---

const SUPERTAGS: &[(&str, &str)] = &[
    ("quest.toml", include_str!("../../../supertags/quest.toml")),
    (
        "handwritten_note.toml",
        include_str!("../../../supertags/handwritten_note.toml"),
    ),
    (
        "drawing.toml",
        include_str!("../../../supertags/drawing.toml"),
    ),
    (
        "document.toml",
        include_str!("../../../supertags/document.toml"),
    ),
    (
        "person.toml",
        include_str!("../../../supertags/person.toml"),
    ),
    ("email.toml", include_str!("../../../supertags/email.toml")),
];

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init => cmd_init(&cli.data_dir),
        Command::Add {
            content,
            node_type,
            fields,
            parent,
        } => cmd_add(
            &cli.data_dir,
            &content,
            node_type.as_deref(),
            &fields,
            parent.as_deref(),
        ),
        Command::List { node_type, limit } => cmd_list(&cli.data_dir, node_type.as_deref(), limit),
    }
}

// ─── init ───────────────────────────────────────────────────────────────────

fn cmd_init(data_dir: &Path) -> Result<()> {
    let supertags_dir = data_dir.join("supertags");
    std::fs::create_dir_all(&supertags_dir)
        .with_context(|| format!("creating {}", supertags_dir.display()))?;

    for (filename, content) in SUPERTAGS {
        let path = supertags_dir.join(filename);
        if path.exists() {
            eprintln!("  skip  {filename} (already exists)");
        } else {
            std::fs::write(&path, content)
                .with_context(|| format!("writing {}", path.display()))?;
            eprintln!("  write {filename}");
        }
    }

    let log_path = data_dir.join("kidur.jsonl");
    if !log_path.exists() {
        std::fs::write(&log_path, "")
            .with_context(|| format!("creating {}", log_path.display()))?;
        eprintln!("  write kidur.jsonl");
    }

    eprintln!("\nInitialized at {}", data_dir.display());
    eprintln!("  supertags/ — {} definitions", SUPERTAGS.len());
    eprintln!("  kidur.jsonl — append-only mutation log");
    Ok(())
}

// ─── add ────────────────────────────────────────────────────────────────────

fn cmd_add(
    data_dir: &Path,
    content: &str,
    node_type: Option<&str>,
    raw_fields: &[String],
    parent: Option<&str>,
) -> Result<()> {
    let registry = load_registry(data_dir)?;

    // Look up supertag definition (if specified)
    let def: Option<&SupertagDef> = match node_type {
        Some(tag) => {
            let d = registry
                .get(tag)
                .with_context(|| format!("unknown supertag: {tag}"))?;
            Some(d)
        }
        None => None,
    };

    // Parse --field key=value pairs
    let fields = parse_fields(raw_fields, def)?;

    // Parse parent ID
    let parent_id = parent
        .map(|s| s.parse::<NodeId>())
        .transpose()
        .context("bad --parent UUID")?;

    // Build the node
    let mut node = Node::new(content);
    node.supertag = node_type.map(|s| s.to_string());
    node.fields = fields;
    if let Some(pid) = parent_id {
        node.parent_id = Some(pid);
    }

    // Validate against supertag
    registry
        .validate_node(&node)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Append to log
    let mut log = MutationLog::open(data_dir.join("kidur.jsonl"))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let entry = log
        .append(Mutation::CreateNode { node: node.clone() })
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("{}", node.id);
    eprintln!(
        "  seq={} type={} content={:?}",
        entry.seq,
        node_type.unwrap_or("-"),
        truncate(content, 60)
    );

    Ok(())
}

// ─── list ───────────────────────────────────────────────────────────────────

fn cmd_list(data_dir: &Path, filter_type: Option<&str>, limit: usize) -> Result<()> {
    let log_path = data_dir.join("kidur.jsonl");
    let entries = MutationLog::replay(&log_path).map_err(|e| anyhow::anyhow!("{e}"))?;

    // Build current state by replaying mutations
    let mut nodes: BTreeMap<NodeId, Node> = BTreeMap::new();
    for entry in &entries {
        match &entry.mutation {
            Mutation::CreateNode { node } | Mutation::UpdateNode { node } => {
                nodes.insert(node.id, node.clone());
            }
            Mutation::DeleteNode { id } => {
                nodes.remove(id);
            }
            _ => {}
        }
    }

    // Filter and collect
    let mut results: Vec<&Node> = nodes
        .values()
        .filter(|n| match filter_type {
            Some(t) => n.supertag.as_deref() == Some(t),
            None => true,
        })
        .collect();

    // Sort by created_at descending (newest first)
    results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    results.truncate(limit);

    if results.is_empty() {
        eprintln!("no nodes found");
        return Ok(());
    }

    // Print header
    println!(
        "{:<12} {:<20} {}",
        "ID", "TYPE", "CONTENT"
    );
    println!("{}", "-".repeat(72));

    for node in &results {
        let short_id = &node.id.to_string()[..12];
        let tag = node.supertag.as_deref().unwrap_or("-");
        let content = truncate(&node.content, 38);
        println!("{:<12} {:<20} {}", short_id, tag, content);
    }

    eprintln!("\n{} node(s)", results.len());
    Ok(())
}

// ─── helpers ────────────────────────────────────────────────────────────────

fn load_registry(data_dir: &Path) -> Result<SupertagRegistry> {
    let supertags_dir = data_dir.join("supertags");
    if !supertags_dir.exists() {
        return Ok(SupertagRegistry::empty());
    }
    SupertagRegistry::from_dir(&supertags_dir).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Parse `key=value` strings into typed FieldValues using the supertag schema.
/// Fields not in the schema default to Text (open-world).
fn parse_fields(
    raw: &[String],
    def: Option<&SupertagDef>,
) -> Result<BTreeMap<String, FieldValue>> {
    let mut fields = BTreeMap::new();

    // Build field type lookup from supertag definition
    let field_defs: std::collections::HashMap<&str, &kidur_core::FieldDef> = def
        .map(|d| {
            d.fields
                .iter()
                .map(|fd| (fd.name.as_str(), fd))
                .collect()
        })
        .unwrap_or_default();

    for pair in raw {
        let (key, value) = pair
            .split_once('=')
            .with_context(|| format!("field must be key=value, got: {pair}"))?;

        let field_type = field_defs
            .get(key)
            .map(|fd| &fd.field_type)
            .unwrap_or(&FieldType::Text);

        let fv = parse_field_value(value, field_type)
            .with_context(|| format!("field '{key}': bad value '{value}'"))?;
        fields.insert(key.to_string(), fv);
    }

    Ok(fields)
}

fn parse_field_value(raw: &str, ft: &FieldType) -> Result<FieldValue> {
    Ok(match ft {
        FieldType::Text => FieldValue::Text(raw.to_string()),
        FieldType::RichText => FieldValue::RichText(raw.to_string()),
        FieldType::Number => {
            let n: f64 = raw.parse().context("expected a number")?;
            FieldValue::Number(n)
        }
        FieldType::Bool => {
            let b = match raw {
                "true" | "1" | "yes" => true,
                "false" | "0" | "no" => false,
                _ => bail!("expected true/false/yes/no/1/0"),
            };
            FieldValue::Bool(b)
        }
        FieldType::Enum => FieldValue::Enum(raw.to_string()),
        FieldType::MultiSelect => {
            let items: Vec<String> = raw.split(',').map(|s| s.trim().to_string()).collect();
            FieldValue::MultiSelect(items)
        }
        FieldType::Reference => {
            let id: NodeId = raw.parse().map_err(|e| anyhow::anyhow!("bad UUID: {e}"))?;
            FieldValue::Reference(id)
        }
        FieldType::Timestamp => {
            // Try full ISO 8601 first, then date-only
            let dt = raw
                .parse::<chrono::DateTime<chrono::Utc>>()
                .or_else(|_| {
                    chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d").map(|d| {
                        d.and_hms_opt(0, 0, 0)
                            .expect("midnight is valid")
                            .and_utc()
                    })
                })
                .context("expected ISO date (YYYY-MM-DD or full RFC3339)")?;
            FieldValue::Timestamp(dt)
        }
        FieldType::Email => FieldValue::Email(raw.to_string()),
        FieldType::Url => FieldValue::Url(raw.to_string()),
        FieldType::Geo => {
            let parts: Vec<&str> = raw.splitn(3, ',').collect();
            if parts.len() < 2 {
                bail!("expected lat,lng[,label]");
            }
            let lat: f64 = parts[0].trim().parse().context("bad latitude")?;
            let lng: f64 = parts[1].trim().parse().context("bad longitude")?;
            let label = parts.get(2).map(|s| s.trim().to_string());
            FieldValue::Geo { lat, lng, label }
        }
    })
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.min(s.len())])
    }
}
