use anyhow::Result;
use std::fs;
use std::path::Path;

/// Skill files embedded at compile time. This eliminates the need for users
/// to manually copy skill files — `codesnap skill` writes them automatically.
const SKILL_MD: &str = include_str!("../.claude/skills/lhr-codesnap/SKILL.md");
const COMMANDS_MD: &str = include_str!("../.claude/skills/lhr-codesnap/references/commands.md");
const ARCHITECTURE_MD: &str =
    include_str!("../.claude/skills/lhr-codesnap/references/architecture.md");

/// Install skill files to a target project directory.
///
/// Creates `.claude/skills/lhr-codesnap/` with `SKILL.md` and
/// `references/commands.md`, `references/architecture.md`.
pub fn install(project_path: &str) -> Result<()> {
    let base = Path::new(project_path);
    let skill_dir = base.join(".claude").join("skills").join("lhr-codesnap");
    let refs_dir = skill_dir.join("references");

    fs::create_dir_all(&refs_dir)?;

    fs::write(skill_dir.join("SKILL.md"), SKILL_MD)?;
    fs::write(refs_dir.join("commands.md"), COMMANDS_MD)?;
    fs::write(refs_dir.join("architecture.md"), ARCHITECTURE_MD)?;

    eprintln!("Skill files installed to {}", skill_dir.display());
    eprintln!("Run 'codesnap init' to build the project index.");
    Ok(())
}
