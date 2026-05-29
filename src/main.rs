mod index;
mod output;
mod query;
mod sync;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt};

/// Zero-config semantic code intelligence for AI coding agents.
#[derive(Parser)]
#[command(name = "codesnap", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build the full index for a project
    Init {
        /// Project path (defaults to current directory)
        #[arg(default_value = ".")]
        path: String,
        /// Force rebuild even if index exists
        #[arg(long)]
        force: bool,
        /// Suppress progress output
        #[arg(long)]
        quiet: bool,
    },
    /// Locate a symbol definition
    Find {
        /// Symbol name to search for
        name: String,
        /// Filter by symbol kind (class, function, method, etc.)
        #[arg(long)]
        kind: Option<String>,
        /// Restrict search to files matching glob
        #[arg(long)]
        file: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Find callers of a symbol
    Callers {
        /// Symbol name
        name: String,
        /// How many levels of transitive callers (default: 1)
        #[arg(long, default_value = "1")]
        depth: usize,
        /// Max results
        #[arg(long, default_value = "50")]
        limit: usize,
        /// Only show test files
        #[arg(long)]
        test_only: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Find what a symbol calls
    Callees {
        /// Symbol name
        name: String,
        /// Transitive depth (default: 1)
        #[arg(long, default_value = "1")]
        depth: usize,
        /// Max results
        #[arg(long, default_value = "50")]
        limit: usize,
        /// Include external/third-party calls
        #[arg(long)]
        external: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Analyze change impact radius
    Impact {
        /// Symbol name to analyze
        name: String,
        /// Max call depth (default: 3)
        #[arg(long, default_value = "3")]
        depth: usize,
        /// Only show affected test files
        #[arg(long)]
        test_only: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Trace call path from A to B
    Trace {
        /// Starting symbol
        from: String,
        /// Target symbol
        to: String,
        /// Max search depth
        #[arg(long, default_value = "7")]
        max_depth: usize,
        /// Show all paths instead of shortest
        #[arg(long)]
        all_paths: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Build relevant code context for a task
    Context {
        /// Task description
        task: String,
        /// Max symbols to include
        #[arg(long, default_value = "30")]
        max_nodes: usize,
        /// Include source code snippets
        #[arg(long)]
        include_code: bool,
        /// Output format
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Show index health and statistics
    Status {
        /// Project path
        #[arg(default_value = ".")]
        path: String,
    },
    /// Verify index freshness
    Check {
        /// Project path
        #[arg(default_value = ".")]
        path: String,
        /// Automatically re-index stale files
        #[arg(long)]
        fix: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .with_timer(fmt::time::uptime())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init { path, force, quiet } => {
            index::Builder::new(&path).force(force).quiet(quiet).build()?;
        }
        Command::Find { name, kind, file, json } => {
            let idx = index::open_current()?;
            let results = query::Finder::new(&idx)
                .kind(kind.as_deref())
                .file_filter(file.as_deref())
                .find(&name)?;
            output::format::print_find_results(&results, json)?;
        }
        Command::Callers { name, depth, limit, test_only, json } => {
            let idx = index::open_current()?;
            let results = query::Finder::new(&idx).find_callers(&name, depth, limit, test_only)?;
            output::format::print_edge_results(&results, "caller", json)?;
        }
        Command::Callees { name, depth, limit, external, json } => {
            let idx = index::open_current()?;
            let results = query::Finder::new(&idx).find_callees(&name, depth, limit, external)?;
            output::format::print_edge_results(&results, "callee", json)?;
        }
        Command::Impact { name, depth, test_only, json } => {
            let idx = index::open_current()?;
            let report = query::Impacter::new(&idx).analyze(&name, depth, test_only)?;
            output::format::print_impact_report(&report, json)?;
        }
        Command::Trace { from, to, max_depth, all_paths, json } => {
            let idx = index::open_current()?;
            let paths = query::Tracer::new(&idx).trace(&from, &to, max_depth, all_paths)?;
            output::format::print_trace_paths(&paths, json)?;
        }
        Command::Context { task, max_nodes, include_code, format } => {
            let idx = index::open_current()?;
            let ctx = query::Finder::new(&idx).build_context(&task, max_nodes, include_code)?;
            output::format::print_context(&ctx, &format)?;
        }
        Command::Status { path } => {
            let s = index::status(&path)?;
            output::format::print_status(&s)?;
        }
        Command::Check { path, fix, json } => {
            let report = sync::Checker::new(&path).fix(fix).run()?;
            output::format::print_check_report(&report, json)?;
        }
    }

    Ok(())
}
