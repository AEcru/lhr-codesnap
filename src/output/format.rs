use crate::index::StatusReport;
use crate::query::finder::{FindResult, TaskContext};
use crate::query::impacter::ImpactReport;
use crate::query::tracer::TracePath;
use crate::sync::checker::CheckReport;
use anyhow::Result;

/// Print symbol find results.
pub fn print_find_results(results: &[FindResult], json: bool) -> Result<()> {
    if json {
        let output: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "kind": r.kind,
                    "visibility": r.visibility,
                    "file": r.file,
                    "line": r.line,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        for r in results {
            let vis = if r.visibility.is_empty() {
                String::new()
            } else {
                format!("{} ", r.visibility)
            };
            println!("{} {}{} \u{2192} {}:{}", r.kind, vis, r.name, r.file, r.line);
        }
        if results.is_empty() {
            println!("(no results found)");
        }
    }
    Ok(())
}

/// Print caller/callee edge results.
pub fn print_edge_results(results: &[FindResult], _direction: &str, json: bool) -> Result<()> {
    print_find_results(results, json)
}

/// Print impact analysis report.
pub fn print_impact_report(report: &ImpactReport, json: bool) -> Result<()> {
    if json {
        let output = serde_json::json!({
            "symbol": report.symbol,
            "total_files": report.total_files,
            "direct_files": report.direct_files.iter().map(|f| serde_json::json!({
                "path": f.path, "symbols": f.symbols, "is_test": f.is_test,
            })).collect::<Vec<_>>(),
            "transitive_files": report.transitive_files.iter().map(|f| serde_json::json!({
                "path": f.path, "symbols": f.symbols, "is_test": f.is_test,
            })).collect::<Vec<_>>(),
            "affected_tests": report.affected_tests.iter().map(|f| serde_json::json!({
                "path": f.path, "symbols": f.symbols, "is_test": f.is_test,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("## Impact Analysis: {}", report.symbol);
        println!();

        if !report.direct_files.is_empty() {
            println!("### Direct Callers ({} files)", report.direct_files.len());
            for f in &report.direct_files {
                println!("- {}", f.path);
                if !f.symbols.is_empty() {
                    println!("  via: {}", f.symbols.join(", "));
                }
            }
            println!();
        }

        if !report.transitive_files.is_empty() {
            println!("### Transitive Callers ({} files)", report.transitive_files.len());
            for f in &report.transitive_files {
                println!("- {}", f.path);
                if !f.symbols.is_empty() {
                    println!("  via: {}", f.symbols.join(", "));
                }
            }
            println!();
        }

        if !report.affected_tests.is_empty() {
            println!("### Affected Tests ({} files) \u{26a0}", report.affected_tests.len());
            for f in &report.affected_tests {
                println!("- {} \u{26a0}", f.path);
            }
            println!();
        }

        println!("Total: {} unique files affected.", report.total_files);
    }
    Ok(())
}

/// Print call chain trace paths.
pub fn print_trace_paths(paths: &[TracePath], json: bool) -> Result<()> {
    if json {
        let output: Vec<serde_json::Value> = paths
            .iter()
            .map(|p| {
                serde_json::json!({
                    "depth": p.depth,
                    "hops": p.hops.iter().map(|h| serde_json::json!({
                        "name": h.name, "file": h.file, "line": h.line, "kind": h.kind,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        for (i, path) in paths.iter().enumerate() {
            if i > 0 {
                println!("\n---\n");
            }
            for (j, hop) in path.hops.iter().enumerate() {
                let prefix = if j == 0 { "" } else { "  \u{2192} " };
                let indent = "  ".repeat(j);
                println!("{}{}{} ({})", indent, prefix, hop.name, format!("{}:{}", hop.file, hop.line));
            }
            println!("\n{} hops, {} intermediate nodes", path.hops.len(), path.depth);
        }
    }
    Ok(())
}

/// Print task context.
pub fn print_context(ctx: &TaskContext, format: &str) -> Result<()> {
    match format {
        "json" => {
            let output = serde_json::json!({
                "task": ctx.task,
                "entry_points": ctx.entry_points.iter().map(|ep| serde_json::json!({
                    "name": ep.name, "kind": ep.kind, "file": ep.file, "line": ep.line,
                })).collect::<Vec<_>>(),
                "related_symbols": ctx.related_symbols.iter().map(|rs| serde_json::json!({
                    "name": rs.name, "kind": rs.kind, "file": rs.file, "line": rs.line,
                })).collect::<Vec<_>>(),
                "call_edges": ctx.call_edges.iter().map(|(a, b)| serde_json::json!([a, b])).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        _ => {
            println!("## Context: {}", ctx.task);
            println!();

            if !ctx.entry_points.is_empty() {
                println!("### Entry Points");
                for ep in &ctx.entry_points {
                    println!("- {} {} \u{2192} {}:{}", ep.kind, ep.name, ep.file, ep.line);
                }
                println!();
            }

            if !ctx.related_symbols.is_empty() {
                println!("### Related Symbols");
                for rs in &ctx.related_symbols {
                    println!("- {} {} \u{2192} {}:{}", rs.kind, rs.name, rs.file, rs.line);
                }
                println!();
            }

            if !ctx.call_edges.is_empty() {
                println!("### Key Call Edges");
                for (from, to) in &ctx.call_edges {
                    println!("- {} \u{2192} {}", from, to);
                }
            }
        }
    }
    Ok(())
}

/// Print index status.
pub fn print_status(status: &StatusReport) -> Result<()> {
    println!("## Index Status");
    println!("- State: {}", status.state);
    if status.symbols_count > 0 {
        println!("- Symbols indexed: {}", status.symbols_count);
        println!("- Files indexed: {}", status.files_count);
        println!(
            "- Disk size: {:.1} KB",
            status.disk_size_bytes as f64 / 1024.0
        );
        if !status.last_indexed.is_empty() {
            println!("- Last indexed: {}", status.last_indexed);
        }
    }
    if !status.pending_files.is_empty() {
        println!();
        println!("### Pending Files");
        for pf in &status.pending_files {
            println!("- {}", pf);
        }
    }
    Ok(())
}

/// Print check report.
pub fn print_check_report(report: &CheckReport, json: bool) -> Result<()> {
    if json {
        let output = serde_json::json!({
            "fresh": report.fresh,
            "total_files": report.total_files,
            "stale_files": report.stale_files.iter().map(|sf| serde_json::json!({
                "path": sf.path, "age_seconds": sf.age_seconds,
            })).collect::<Vec<_>>(),
            "fixed": report.fixed,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if report.fresh {
            println!("Index is up to date ({} files).", report.total_files);
        } else {
            println!(
                "Index is stale: {} of {} files have changed.",
                report.stale_files.len(),
                report.total_files
            );
            for sf in &report.stale_files {
                println!("  - {} (modified {}s ago)", sf.path, sf.age_seconds);
            }
            if report.fixed {
                println!("Index has been rebuilt.");
            } else {
                println!("Run `codesnap check --fix` to re-index.");
            }
        }
    }
    std::process::exit(if report.fresh { 0 } else { 3 });
}
