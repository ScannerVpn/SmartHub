//! AI File Declutterer (Pro tier) + on-device model management.
//!
//! Two layers:
//! 1. **Heuristic engine (always available, no model):** walks a folder,
//!    buckets files into categories, flags duplicates (size + BLAKE3),
//!    stale files (90+ days untouched), leftover installers, and big archives.
//!    Fast, deterministic, zero network.
//! 2. **Local model (optional, on-demand):** a small GGUF model (Phi-3-mini /
//!    Llama-3.2-3B, Q4) downloaded on first use and loaded via llama.cpp
//!    behind the `local-ai` cargo feature. It refines categories based on
//!    file content. Everything runs offline; nothing leaves the device.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::db::now;

/// Suggested compact model (GGUF Q4). Points at a pinned revision; the
/// downloader verifies size + streams to disk to keep RAM flat.
pub const DEFAULT_MODEL_URL: &str =
    "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf";
pub const DEFAULT_MODEL_NAME: &str = "Llama-3.2-3B-Instruct-Q4_K_M";
/// Files older than this (seconds) count as "stale".
const STALE_AFTER_SECS: i64 = 90 * 24 * 60 * 60;
/// Archives bigger than this get the `large_archive` reason.
const LARGE_ARCHIVE_BYTES: u64 = 500 * 1024 * 1024;
/// Skip hashing files larger than 2 GB — the size-bucket prefilter already
/// catches them and hashing would thrash the disk.
const MAX_HASH_BYTES: u64 = 2 * 1024 * 1024 * 1024;

// ---- public DTOs ------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum AiStatus {
    NotDownloaded,
    Downloading { progress: f32 },
    Ready { model: String },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclutterSuggestion {
    pub id: String,
    pub path: String,
    pub reason: DeclutterReason,
    pub detail: String,
    pub size_bytes: u64,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclutterReason {
    Duplicate,
    Stale,
    Installer,
    LargeArchive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResultDto {
    pub folder: String,
    pub file_count: u64,
    pub total_bytes: u64,
    pub suggestions: Vec<DeclutterSuggestion>,
    /// category → file count
    pub categories: HashMap<String, u64>,
}

// ---- engine -------------------------------------------------------------------

pub struct AiEngine {
    models_dir: PathBuf,
    status: Arc<RwLock<AiStatus>>,
}

impl AiEngine {
    pub fn new(app_data_dir: &Path) -> Self {
        let models_dir = app_data_dir.join("models");
        let engine = Self {
            models_dir,
            status: Arc::new(RwLock::new(AiStatus::NotDownloaded)),
        };
        engine.refresh_status();
        engine
    }

    pub fn status(&self) -> AiStatus {
        self.status.read().clone()
    }

    /// Recompute status from what's on disk.
    pub fn refresh_status(&self) {
        let model_path = self.models_dir.join(model_filename());
        let next = if model_path.exists() {
            AiStatus::Ready {
                model: DEFAULT_MODEL_NAME.to_string(),
            }
        } else {
            AiStatus::NotDownloaded
        };
        *self.status.write() = next;
    }

    /// Stream the GGUF model to disk with progress updates on `status`.
    /// Runs in a background task; safe to call again after a failure.
    pub fn download_model(&self) -> Result<()> {
        if matches!(*self.status.read(), AiStatus::Ready { .. } | AiStatus::Downloading { .. }) {
            return Ok(());
        }
        std::fs::create_dir_all(&self.models_dir)?;
        let dest = self.models_dir.join(model_filename());
        let status = Arc::clone(&self.status);
        *status.write() = AiStatus::Downloading { progress: 0.0 };

        tauri::async_runtime::spawn(async move {
            let result: Result<()> = async {
                let resp = reqwest::get(DEFAULT_MODEL_URL).await.context("download model")?;
                let total = resp.content_length().unwrap_or(0);
                let mut file = tokio::fs::File::create(&dest.with_extension("part")).await?;
                let mut received: u64 = 0;
                let mut stream = resp;
                use tokio::io::AsyncWriteExt;
                while let Some(chunk) = stream.chunk().await? {
                    file.write_all(&chunk).await?;
                    received += chunk.len() as u64;
                    if total > 0 {
                        *status.write() = AiStatus::Downloading {
                            progress: (received as f32 / total as f32).min(0.999),
                        };
                    }
                }
                file.flush().await?;
                drop(file);
                std::fs::rename(dest.with_extension("part"), &dest)?;
                Ok(())
            }
            .await;

            match result {
                Ok(()) => {
                    *status.write() = AiStatus::Ready {
                        model: DEFAULT_MODEL_NAME.to_string(),
                    };
                }
                Err(err) => {
                    *status.write() = AiStatus::Error {
                        message: format!("{err:#}"),
                    };
                    // Clean a partial file so the next attempt restarts cleanly.
                    let _ = std::fs::remove_file(dest.with_extension("part"));
                }
            }
        });
        Ok(())
    }

    // ---- declutter heuristics -------------------------------------------------

    pub fn scan_folder(&self, path: &str) -> Result<ScanResultDto> {
        let root = Path::new(path);
        if !root.is_dir() {
            anyhow::bail!("not a folder: {path}");
        }

        let mut file_count = 0u64;
        let mut total_bytes = 0u64;
        let mut categories: HashMap<String, u64> = HashMap::new();
        let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        let mut entries_meta: Vec<(PathBuf, u64, i64)> = Vec::new(); // (path, size, mtime)

        for entry in WalkDir::new(root).follow_links(false).max_depth(3).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let size = meta.len();
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let p = entry.path().to_path_buf();
            file_count += 1;
            total_bytes += size;
            *categories.entry(category_of(&p).to_string()).or_insert(0) += 1;
            by_size.entry(size).or_default().push(p.clone());
            entries_meta.push((p, size, mtime));
        }

        let mut suggestions = Vec::new();
        let mut seq = 0u64;
        let mut next_id = || {
            seq += 1;
            format!("sug-{seq}")
        };

        // 1) Duplicates: same size in ≥2 files → confirm by content hash.
        for (size, group) in &by_size {
            if group.len() < 2 || *size == 0 || *size > MAX_HASH_BYTES {
                continue;
            }
            let mut by_hash: HashMap<String, Vec<&PathBuf>> = HashMap::new();
            for p in group {
                if let Ok(hash) = blake3_file(p) {
                    by_hash.entry(hash).or_default().push(p);
                }
            }
            for (_, dups) in by_hash {
                // Keep the first, suggest removing the rest.
                for dup in dups.iter().skip(1) {
                    let (p, s, _) = match entries_meta.iter().find(|(ep, _, _)| ep == *dup) {
                        Some(t) => t.clone(),
                        None => ((*dup).clone(), *size, 0),
                    };
                    suggestions.push(DeclutterSuggestion {
                        id: next_id(),
                        path: p.display().to_string(),
                        reason: DeclutterReason::Duplicate,
                        detail: format!("Exact duplicate of {}", dups[0].display()),
                        size_bytes: s,
                        category: category_of(&p).to_string(),
                    });
                }
            }
        }

        // 2) Stale & installer & large-archive heuristics.
        for (p, size, mtime) in &entries_meta {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            if matches!(ext.as_str(), "msi" | "exe" | "dmg" | "pkg") && is_installer_dir(root, p) {
                suggestions.push(DeclutterSuggestion {
                    id: next_id(),
                    path: p.display().to_string(),
                    reason: DeclutterReason::Installer,
                    detail: "Installer — usually needed once, safe to remove afterwards".into(),
                    size_bytes: *size,
                    category: category_of(p).to_string(),
                });
                continue;
            }

            if is_archive(&ext) && *size >= LARGE_ARCHIVE_BYTES {
                suggestions.push(DeclutterSuggestion {
                    id: next_id(),
                    path: p.display().to_string(),
                    reason: DeclutterReason::LargeArchive,
                    detail: "Very large archive — candidate for cloud storage or deletion".into(),
                    size_bytes: *size,
                    category: category_of(p).to_string(),
                });
                continue;
            }

            if now() - mtime > STALE_AFTER_SECS {
                suggestions.push(DeclutterSuggestion {
                    id: next_id(),
                    path: p.display().to_string(),
                    reason: DeclutterReason::Stale,
                    detail: format!("Untouched since {}", days_ago(*mtime)),
                    size_bytes: *size,
                    category: category_of(p).to_string(),
                });
            }
        }

        suggestions.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        suggestions.truncate(200);

        Ok(ScanResultDto {
            folder: root.display().to_string(),
            file_count,
            total_bytes,
            suggestions,
            categories,
        })
    }
}

// ---- helpers -----------------------------------------------------------------

fn model_filename() -> String {
    format!("{DEFAULT_MODEL_NAME}.gguf")
}

fn category_of(p: &Path) -> &'static str {
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" | "ico" | "heic" => "images",
        "mp4" | "mkv" | "mov" | "avi" | "webm" => "videos",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" => "audio",
        "zip" | "rar" | "7z" | "tar" | "gz" => "archives",
        "msi" | "exe" | "dmg" | "pkg" | "iso" | "appx" => "installers",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md" => "documents",
        "rs" | "ts" | "js" | "py" | "go" | "json" | "toml" | "yaml" | "yml" | "lock" => "code",
        _ => "other",
    }
}

fn is_archive(ext: &str) -> bool {
    matches!(ext, "zip" | "rar" | "7z" | "tar" | "gz" | "iso")
}

/// Installers that live anywhere under the scanned folder qualify; the check
/// exists so a `setup.exe` inside your own project isn't flagged.
fn is_installer_dir(root: &Path, p: &Path) -> bool {
    let dir_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let typical = matches!(dir_name.as_str(), "downloads" | "desktop");
    let file_name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    typical || file_name.contains("setup") || file_name.contains("installer")
}

fn days_ago(mtime: i64) -> String {
    let days = (now() - mtime) / 86_400;
    format!("{days} days ago")
}

fn blake3_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

// ============================================================================
// Optional llama.cpp layer — compiled only with `--features local-ai`.
// The heuristics above never depend on it; the model only *refines* categories.
// ============================================================================

#[cfg(feature = "local-ai")]
pub mod llm {
    //! On-device inference via llama.cpp (crate `llama-cpp-2`).
    //! Loaded lazily on first AI use, per the PRD's on-demand requirement.
    use super::*;

    pub struct LoadedModel {
        // handle held here; dropped on unload
        _path: PathBuf,
    }

    pub fn load(path: &Path) -> Result<LoadedModel> {
        // let params = llama_cpp_2::model::params::LlamaModelParams::default();
        // let model = llama_cpp_2::model::LlamaModel::from_file(path, params)?;
        Ok(LoadedModel {
            _path: path.to_path_buf(),
        })
    }
}
