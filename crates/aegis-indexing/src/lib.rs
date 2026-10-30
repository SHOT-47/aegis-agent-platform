//! Incremental SHA-256 Codebase Intelligence & Citation-Preserving Search Engine (Section 4).
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEvidenceHit {
    pub file_path: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub symbol_name: Option<String>,
    pub score: f64,
    pub snippet: String,
    pub citation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalIndexReport {
    pub files_scanned: usize,
    pub files_reindexed: usize,
    pub files_unchanged: usize,
    pub total_symbols: usize,
}

pub struct ProjectIntelligenceIndex {
    workspace_root: PathBuf,
    file_hashes: BTreeMap<PathBuf, String>,
    symbols: Vec<(PathBuf, usize, String, String)>,
}

impl ProjectIntelligenceIndex {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self { workspace_root: workspace_root.into(), file_hashes: BTreeMap::new(), symbols: Vec::new() }
    }
    pub fn sync_incremental(&mut self) -> IncrementalIndexReport {
        let mut files = Vec::new();
        self.walk(&self.workspace_root.clone(), &mut files);
        let mut reindexed = 0;
        let mut unchanged = 0;
        let fn_re = Regex::new(r"^\s*(?:pub\s+)?(?:async\s+)?(?:fn|struct|enum|trait|def|class)\s+([A-Za-z0-9_]+)").unwrap();
        for fpath in &files {
            let Ok(rel) = fpath.strip_prefix(&self.workspace_root) else { continue };
            let Ok(bytes) = fs::read(fpath) else { continue };
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let digest = hex::encode(hasher.finalize());
            if self.file_hashes.get(rel) == Some(&digest) { unchanged += 1; continue; }
            self.file_hashes.insert(rel.to_path_buf(), digest);
            reindexed += 1;
            let text = String::from_utf8_lossy(&bytes);
            for (idx, line) in text.lines().enumerate() {
                if let Some(cap) = fn_re.captures(line) {
                    self.symbols.push((rel.to_path_buf(), idx + 1, cap[1].to_string(), line.trim().to_string()));
                }
            }
        }
        IncrementalIndexReport { files_scanned: files.len(), files_reindexed: reindexed, files_unchanged: unchanged, total_symbols: self.symbols.len() }
    }
    fn walk(&self, dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(".") || name == "target" || name == "node_modules" { continue; }
            if p.is_dir() { self.walk(&p, out); } else if p.is_file() { out.push(p); }
        }
    }
    pub fn search(&self, query: &str, top_k: usize) -> Vec<SearchEvidenceHit> {
        let q = query.to_lowercase();
        let mut hits: Vec<SearchEvidenceHit> = self.symbols.iter().filter_map(|(path, line, name, sig)| {
            if name.to_lowercase().contains(&q) || sig.to_lowercase().contains(&q) {
                Some(SearchEvidenceHit {
                    file_path: path.clone(), line_start: *line, line_end: line + 15,
                    symbol_name: Some(name.clone()), score: 10.0, snippet: sig.clone(),
                    citation: format!("{}:{}-{}", path.display(), line, line + 15),
                })
            } else { None }
        }).collect();
        hits.truncate(top_k);
        hits
    }
}
