//! Downloads Intelligence Scanner
//!
//! FORGE Session: 20260203-step3-downloads
//! Problem: Scan ~/Downloads, classify by nexcore relevance, report actions.
//!
//! T1 decomposition:
//! - **Sequence (σ)**: Directory walk → Classify each → Collect result.
//! - **Mapping (μ)**: Transformation of `Path` attributes to `ContentType`.
//! - **Recursion (ρ)**: Recursive directory size calculation (`dir_size`).
//! - **State (ς)**: `ScanReport` accumulating classified findings.
//! - **Void (∅)**: `Unknown` variant representing unclassifiable path absence.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

// ============================================================================
// T3: Content Type Classification (Mapping primitive)
// ============================================================================

/// What kind of content this entry represents.
///
/// Tier: T3 (domain-specific to NexCore ecosystem)
/// Grounds to: T1 Mapping (path attributes → category)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentType {
    /// NexCore skill directory (has SKILL.md)
    Skill { name: String },
    /// NexCore configuration (CLAUDE.md, .mcp.json)
    NexConfig,
    /// Rust project (has Cargo.toml)
    RustProject { name: String },
    /// FDA FAERS adverse event data
    FaersData { quarter: String },
    /// Primitive Codex or framework document
    PrimitiveDocument,
    /// General documentation (markdown, pdf, txt)
    Documentation,
    /// Archive file (zip, tar)
    Archive,
    /// Python code (migration target)
    PythonLegacy,
    /// Unclassifiable (Void primitive)
    Unknown,
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Skill { name } => write!(f, "Skill({name})"),
            Self::NexConfig => write!(f, "NexConfig"),
            Self::RustProject { name } => write!(f, "RustProject({name})"),
            Self::FaersData { quarter } => write!(f, "FAERS({quarter})"),
            Self::PrimitiveDocument => write!(f, "PrimitiveDoc"),
            Self::Documentation => write!(f, "Documentation"),
            Self::Archive => write!(f, "Archive"),
            Self::PythonLegacy => write!(f, "PythonLegacy⚠"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

// ============================================================================
// T2-C: Classified Entry (State + Mapping composite)
// ============================================================================

/// A single classified entry from the scan.
///
/// Tier: T2-C (composes State + Mapping)
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ClassifiedEntry {
    /// Path to the entry
    pub path: PathBuf,
    /// Classified content type
    pub content_type: ContentType,
    /// Size in bytes (recursive for directories)
    pub size_bytes: u64,
    /// Whether this entry is a directory
    pub is_directory: bool,
    /// Suggested action for this entry
    pub action: SuggestedAction,
}

/// What should be done with this entry.
///
/// Tier: T2-P (cross-domain: any triage system has actions)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SuggestedAction {
    /// Move into NexCore ecosystem (~/.claude/skills/, ~/nexcore/, etc.)
    Migrate { destination: String },
    /// Archive to long-term storage
    Archive,
    /// Already processed or duplicate — safe to remove
    Cleanup,
    /// Keep in place, no action needed
    Keep,
    /// Needs manual review
    Review,
}

impl fmt::Display for SuggestedAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Migrate { destination } => write!(f, "MIGRATE → {destination}"),
            Self::Archive => write!(f, "ARCHIVE"),
            Self::Cleanup => write!(f, "CLEANUP"),
            Self::Keep => write!(f, "KEEP"),
            Self::Review => write!(f, "REVIEW"),
        }
    }
}

// ============================================================================
// T1: Classification Logic (Mapping primitive)
// ============================================================================

/// Classify a path into a ContentType.
///
/// Pure Mapping: Path → ContentType
pub fn classify(path: &Path) -> ContentType {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let lower = name.to_lowercase();

    // Directory classification
    if path.is_dir() {
        if path.join("SKILL.md").exists() {
            return ContentType::Skill { name };
        }
        if path.join("Cargo.toml").exists() {
            return ContentType::RustProject { name };
        }
        if lower.contains("faers") {
            let quarter = lower
                .split('_')
                .next_back()
                .unwrap_or("unknown")
                .to_string();
            return ContentType::FaersData { quarter };
        }
        if path.join("CLAUDE.md").exists() {
            return ContentType::NexConfig;
        }
        if lower.contains("primitive") || lower.contains("codex") {
            return ContentType::PrimitiveDocument;
        }
    }

    // File classification
    if path.is_file()
        && let Some(ext) = path.extension().and_then(|e| e.to_str())
    {
        match ext {
            "py" => return ContentType::PythonLegacy,
            "zip" | "tar" | "gz" | "bz2" => return ContentType::Archive,
            "pdf" | "txt" => return ContentType::Documentation,
            "md" => {
                if lower.contains("primitive") || lower.contains("codex") {
                    return ContentType::PrimitiveDocument;
                }
                return ContentType::Documentation;
            }
            "rs" | "toml" => {
                return ContentType::RustProject { name };
            }
            "skill" => {
                return ContentType::Skill { name };
            }
            _ => {}
        }
    }

    ContentType::Unknown
}

/// Suggest an action for a classified entry.
///
/// Pure Mapping: ContentType → SuggestedAction
pub fn suggest_action(content_type: &ContentType, path: &Path) -> SuggestedAction {
    match content_type {
        ContentType::Skill { .. } => SuggestedAction::Migrate {
            destination: "~/.claude/skills/".into(),
        },
        ContentType::NexConfig => SuggestedAction::Review,
        ContentType::RustProject { .. } => SuggestedAction::Migrate {
            destination: "~/nexcore/crates/".into(),
        },
        ContentType::FaersData { .. } => {
            if path.is_file() {
                SuggestedAction::Archive
            } else {
                SuggestedAction::Migrate {
                    destination: "~/nexcore/data/faers/".into(),
                }
            }
        }
        ContentType::PrimitiveDocument => SuggestedAction::Migrate {
            destination: "~/.claude/knowledge/primitive-codex/".into(),
        },
        ContentType::Documentation => SuggestedAction::Review,
        ContentType::Archive => SuggestedAction::Archive,
        ContentType::PythonLegacy => SuggestedAction::Migrate {
            destination: "Flag for Rust rewrite".into(),
        },
        ContentType::Unknown => SuggestedAction::Review,
    }
}

// ============================================================================
// T1: Scan Logic (Sequence + Recursion primitives)
// ============================================================================

/// Scan report accumulator.
///
/// Tier: T2-C (State + Aggregation)
#[derive(Debug)]
#[non_exhaustive]
pub struct ScanReport {
    /// Classified entries
    pub entries: Vec<ClassifiedEntry>,
    /// Root path that was scanned
    pub scan_path: PathBuf,
}

impl ScanReport {
    /// Scan a directory and classify all entries.
    ///
    /// Sequence: read_dir → classify each → collect
    /// Recursion: top-level only (depth 1)
    pub fn scan(dir: &Path) -> std::io::Result<Self> {
        let mut entries = Vec::new();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip hidden files
            if path
                .file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }

            let metadata = entry.metadata()?;
            let content_type = classify(&path);
            let action = suggest_action(&content_type, &path);

            let size_bytes = if metadata.is_dir() {
                dir_size(&path).unwrap_or(0)
            } else {
                metadata.len()
            };

            entries.push(ClassifiedEntry {
                path,
                content_type,
                size_bytes,
                is_directory: metadata.is_dir(),
                action,
            });
        }

        // Sort by content type significance
        entries.sort_by(|a, b| a.content_type.to_string().cmp(&b.content_type.to_string()));

        Ok(Self {
            entries,
            scan_path: dir.to_path_buf(),
        })
    }

    /// Count entries by content type.
    ///
    /// Aggregation: Sequence + Mapping → counts
    #[allow(clippy::arithmetic_side_effects, reason = "incrementing counts")]
    pub fn summary(&self) -> Vec<(String, usize)> {
        let mut counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for entry in &self.entries {
            *counts.entry(entry.content_type.to_string()).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    }

    /// Get entries that need action (not Keep).
    pub fn actionable(&self) -> Vec<&ClassifiedEntry> {
        self.entries
            .iter()
            .filter(|e| e.action != SuggestedAction::Keep)
            .collect()
    }

    /// Format as human-readable report.
    pub fn display(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "# Downloads Intelligence Report\n## Scanned: {}\n\n",
            self.scan_path.display()
        ));

        // Summary
        out.push_str("## Summary\n\n| Type | Count |\n|------|-------|\n");
        for (type_name, count) in self.summary() {
            out.push_str(&format!("| {type_name} | {count} |\n"));
        }
        out.push_str(&format!("\n**Total entries:** {}\n\n", self.entries.len()));

        // Actionable items
        let actionable = self.actionable();
        out.push_str(&format!(
            "## Actionable Items ({} of {})\n\n",
            actionable.len(),
            self.entries.len()
        ));
        out.push_str("| Entry | Type | Size | Action |\n|-------|------|------|--------|\n");
        for entry in &actionable {
            let name = entry
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let size = format_bytes(entry.size_bytes);
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                name, entry.content_type, size, entry.action
            ));
        }

        out
    }
}

/// Calculate total size of a directory (Recursion primitive).
#[allow(clippy::arithmetic_side_effects, reason = "size accumulations")]
fn dir_size(path: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                total += dir_size(&entry.path())?;
            } else {
                total += metadata.len();
            }
        }
    }
    Ok(total)
}

/// Format bytes as human-readable string.
#[allow(
    clippy::as_conversions,
    reason = "safe conversion from u64 to f64 for formatting"
)]
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

// ============================================================================
// Tests (CTVP Phase 0: Preclinical)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn classify_skill_directory() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("my-skill");
        fs::create_dir(&skill_dir).unwrap();
        File::create(skill_dir.join("SKILL.md"))
            .unwrap()
            .write_all(b"---\nname: test\n---")
            .unwrap();

        let result = classify(&skill_dir);
        assert!(matches!(result, ContentType::Skill { .. }));
    }

    #[test]
    fn classify_rust_project() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("my-crate");
        fs::create_dir(&proj).unwrap();
        File::create(proj.join("Cargo.toml"))
            .unwrap()
            .write_all(b"[package]\nname = \"test\"")
            .unwrap();

        let result = classify(&proj);
        assert!(matches!(result, ContentType::RustProject { .. }));
    }

    #[test]
    fn classify_faers_directory() {
        let tmp = TempDir::new().unwrap();
        let faers = tmp.path().join("faers_ascii_2025Q4");
        fs::create_dir(&faers).unwrap();

        let result = classify(&faers);
        assert!(matches!(result, ContentType::FaersData { .. }));
    }

    #[test]
    fn classify_python_file() {
        let tmp = TempDir::new().unwrap();
        let py = tmp.path().join("legacy.py");
        File::create(&py).unwrap();

        let result = classify(&py);
        assert_eq!(result, ContentType::PythonLegacy);
    }

    #[test]
    fn classify_codex_md() {
        let tmp = TempDir::new().unwrap();
        let codex = tmp.path().join("THE_PRIMITIVE_CODEX.md");
        File::create(&codex).unwrap();

        let result = classify(&codex);
        assert_eq!(result, ContentType::PrimitiveDocument);
    }

    #[test]
    fn classify_unknown_file() {
        let tmp = TempDir::new().unwrap();
        let unknown = tmp.path().join(".com.google.Chrome.abc");
        File::create(&unknown).unwrap();

        let result = classify(&unknown);
        assert_eq!(result, ContentType::Unknown);
    }

    #[test]
    fn scan_produces_report() {
        let tmp = TempDir::new().unwrap();

        let skill = tmp.path().join("my-skill");
        fs::create_dir(&skill).unwrap();
        File::create(skill.join("SKILL.md")).unwrap();

        File::create(tmp.path().join("readme.md")).unwrap();
        File::create(tmp.path().join("data.zip")).unwrap();

        let report = ScanReport::scan(tmp.path()).unwrap();
        assert_eq!(report.entries.len(), 3);

        let summary = report.summary();
        assert!(!summary.is_empty());
    }

    #[test]
    fn format_bytes_human_readable() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(2_097_152), "2.0 MB");
        assert_eq!(format_bytes(1_610_612_736), "1.5 GB");
    }

    #[test]
    fn actionable_filters_keep() {
        let entries = vec![
            ClassifiedEntry {
                path: PathBuf::from("/tmp/a"),
                content_type: ContentType::Documentation,
                size_bytes: 100,
                is_directory: false,
                action: SuggestedAction::Keep,
            },
            ClassifiedEntry {
                path: PathBuf::from("/tmp/b"),
                content_type: ContentType::Skill {
                    name: "test".into(),
                },
                size_bytes: 200,
                is_directory: true,
                action: SuggestedAction::Migrate {
                    destination: "~/.claude/skills/".into(),
                },
            },
        ];
        let report = ScanReport {
            entries,
            scan_path: PathBuf::from("/tmp"),
        };
        assert_eq!(report.actionable().len(), 1);
    }
}
