
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tools: Vec<SkillTool>,
    #[serde(default)]
    pub prompts: Vec<String>,
    #[serde(skip)]
    pub location: Option<PathBuf>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    pub name: String,
    pub description: String,
    pub kind: String,
    pub command: String,
    #[serde(default)]
    pub args: HashMap<String, String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

/// Load all skills from the `skills/` directory relative to the current working directory.
pub fn load_skills() -> Vec<Skill> {
    let mut skills = Vec::new();
    let dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("skills");

    let Ok(entries) = fs::read_dir(&dir) else {
        log::debug!("No skills directory found at {:?}", dir);
        return skills;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let md_path = path.join("SKILL.md");

        if md_path.exists() {
            match load_skill_md(&md_path, &path) {
                Ok(skill) => skills.push(skill),
                Err(e) => log::warn!("Failed to load skill from {:?}: {}", md_path, e),
            }
        }
    }

    skills
}

/// Load a skill from a SKILL.md file (simpler format)
fn load_skill_md(path: &Path, dir: &Path) -> Result<Skill> {
    let content = fs::read_to_string(path)?;
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(Skill {
        name,
        description: extract_description(&content),
        version: "0.1.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: Vec::new(),
        prompts: vec![content],
        location: Some(path.to_path_buf()),
    })
}


fn extract_description(content: &str) -> String {
    // Skip optional YAML frontmatter
    let content = if content.starts_with("---") {
        if let Some((_, rest)) = content.split_once("---\n").and_then(|(_, after)| after.split_once("---\n")) {
            rest
        } else {
            content
        }
    } else {
        content
    };

    // Find first non-empty line that is not a heading
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            return trimmed.to_string();
        }
    }

    // Fallback: first non-empty line
    content
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("No description")
        .trim()
        .to_string()
}
