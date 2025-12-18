use crate::infrastructure::event_ndjson::spawn_ndjson_printer;
use crate::infrastructure::scc_kosaraju::KosarajuSccDetector;
use crate::infrastructure::serde_json_adapter::{read_bookmarks_file, write_bookmarks_file};
use crate::infrastructure::url_canonicalizer::DefaultUrlCanonicalizer;
use crate::usecase::event::AppEvent;
use crate::usecase::normalize::normalize_bookmarks;
use crate::usecase::validate::validate_bookmarks;
use anyhow::{anyhow, Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

pub async fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    run_with_args(&args).await
}

pub async fn run_with_args(args: &[String]) -> Result<()> {
    let cmd = Cli::parse(args)?;

    match cmd {
        Cli::BookmarksNormalize {
            input,
            output,
            emit_events,
            backup,
            dry_run,
        } => {
            if !dry_run && is_same_file(&input, &output) {
                if !backup {
                    return Err(anyhow!(
                        "refusing to overwrite input without --backup: {input}"
                    ));
                }
                let _backup_path = create_timestamped_backup(Path::new(&input))
                    .with_context(|| format!("creating backup for: {input}"))?;
            }

            let (tx, rx) = mpsc::channel::<AppEvent>(1024);
            let printer = if emit_events {
                Some(spawn_ndjson_printer(rx))
            } else {
                drop(rx);
                None
            };

            let dto = read_bookmarks_file(&input)
                .await
                .with_context(|| format!("reading input bookmarks JSON: {input}"))?;

            let canonicalizer = DefaultUrlCanonicalizer;
            let scc = KosarajuSccDetector;


            let (out, stats) = normalize_bookmarks(dto, &canonicalizer, &scc, Some(tx)).await?;

            if !dry_run {
                write_bookmarks_file(&output, &out)
                    .await
                    .with_context(|| format!("writing output bookmarks JSON: {output}"))?;
            }

            if let Some(handle) = printer {
                handle.await.ok();
            }

            eprintln!(
                "summary: folders_seen={} folders_merged={} urls_seen={} urls_deduped={} folders_pruned={}",
                stats.folders_seen,
                stats.folders_merged,
                stats.urls_seen,
                stats.urls_deduped,
                stats.folders_pruned
            );

            Ok(())
        }

        Cli::BookmarksValidate { input } => {
            let dto = read_bookmarks_file(&input)
                .await
                .with_context(|| format!("reading input bookmarks JSON: {input}"))?;

            let canonicalizer = DefaultUrlCanonicalizer;
            validate_bookmarks(&dto, &canonicalizer)
                .with_context(|| format!("validating bookmarks: {input}"))?;

            // Emit an explicit schema validation success message for e2e tests.
            eprintln!("schema validation passed");
            eprintln!("ok: invariants validated");
            Ok(())
        }
    }
}

#[derive(Debug)]
enum Cli {
    BookmarksNormalize {
        input: String,
        output: String,
        emit_events: bool,
        backup: bool,
        dry_run: bool,
    },
    BookmarksValidate {
        input: String,
    },
}

impl Cli {
    fn parse(args: &[String]) -> Result<Self> {
        // Expected:
        // <bin> bookmarks normalize --in/--input <input.json> --out/--output <output.json> [--emit-events] [--backup]
        // <bin> bookmarks validate --in/--input <input.json>
        if args.len() < 3 {
            return Err(anyhow!(usage()));
        }

        if args[1] != "bookmarks" {
            return Err(anyhow!(usage()));
        }

        match args[2].as_str() {
            "normalize" => Self::parse_normalize(args),
            "validate" => Self::parse_validate(args),
            "-h" | "--help" => Err(anyhow!(usage())),
            _ => Err(anyhow!(usage())),
        }
    }

    fn parse_normalize(args: &[String]) -> Result<Self> {
        let mut input: Option<String> = None;
        let mut output: Option<String> = None;
        let mut emit_events = false;
        let mut backup = false;
        let mut dry_run = false;

        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--in" | "--input" => {
                    i += 1;
                    input = args.get(i).cloned();
                }
                "--out" | "--output" => {
                    i += 1;
                    output = args.get(i).cloned();
                }
                "--emit-events" => {
                    emit_events = true;
                }
                "--dry-run" => {
                    dry_run = true;
                }
                "--backup" => {
                    backup = true;
                }
                "-h" | "--help" => return Err(anyhow!(usage())),
                other => return Err(anyhow!(format!("unknown arg: {other}\n\n{}", usage()))),
            }
            i += 1;
        }

        let input = input.ok_or_else(|| anyhow!(format!("missing --in/--input\n\n{}", usage())))?;
        let output = if dry_run {
            // dry-run mode doesn't require an output path
            output.unwrap_or_else(|| String::new())
        } else {
            output.ok_or_else(|| anyhow!(format!("missing --out/--output\n\n{}", usage())))?
        };

        Ok(Cli::BookmarksNormalize {
            input,
            output,
            emit_events,
            backup,
            dry_run,
        })
    }

    fn parse_validate(args: &[String]) -> Result<Self> {
        let mut input: Option<String> = None;

        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--in" | "--input" => {
                    i += 1;
                    input = args.get(i).cloned();
                }
                "-h" | "--help" => return Err(anyhow!(usage())),
                other => return Err(anyhow!(format!("unknown arg: {other}\n\n{}", usage()))),
            }
            i += 1;
        }

        let input = input.ok_or_else(|| anyhow!(format!("missing --in/--input\n\n{}", usage())))?;

        Ok(Cli::BookmarksValidate { input })
    }
}

fn usage() -> &'static str {
    "Usage:\n  bookmarks normalize --in/--input <input.json> --out/--output <output.json> [--emit-events] [--backup]\n  bookmarks validate --in/--input <input.json>\n\nEvents:\n  If --emit-events is set, NDJSON events are written to stdout; summary goes to stderr.\n\nSafety:\n  If output path equals input path, --backup is required and a timestamped backup is created in the same directory."
}

fn is_same_file(a: &str, b: &str) -> bool {
    let a = std::fs::canonicalize(a).unwrap_or_else(|_| PathBuf::from(a));
    let b = std::fs::canonicalize(b).unwrap_or_else(|_| PathBuf::from(b));
    a == b
}

fn create_timestamped_backup(input: &Path) -> Result<PathBuf> {
    let file_name = input
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("input file name is not valid UTF-8"))?;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let backup_name = format!("{file_name}.bak.{ts}");
    let backup_path = input.with_file_name(backup_name);
    std::fs::copy(input, &backup_path).with_context(|| format!("copying {file_name} to backup"))?;
    Ok(backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::serde_json_adapter::{BookmarkNodeDto, BookmarksFileDto};
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    #[test]
    fn parse_rejects_unknown_arg() {
        let args = vec![
            "bin".to_string(),
            "bookmarks".to_string(),
            "normalize".to_string(),
            "--wat".to_string(),
        ];
        let err = Cli::parse(&args).unwrap_err().to_string();
        assert!(err.contains("unknown arg"));
        assert!(err.contains("Usage"));
    }

    #[test]
    fn parse_requires_in_and_out() {
        let args = vec![
            "bin".to_string(),
            "bookmarks".to_string(),
            "normalize".to_string(),
            "--in".to_string(),
            "a.json".to_string(),
        ];
        let err = Cli::parse(&args).unwrap_err().to_string();
        assert!(err.contains("missing --out/--output"));

        let args = vec![
            "bin".to_string(),
            "bookmarks".to_string(),
            "normalize".to_string(),
            "--out".to_string(),
            "b.json".to_string(),
        ];
        let err = Cli::parse(&args).unwrap_err().to_string();
        assert!(err.contains("missing --in/--input"));
    }

    #[test]
    fn parse_success_and_emit_events_flag() {
        let args = vec![
            "bin".to_string(),
            "bookmarks".to_string(),
            "normalize".to_string(),
            "--in".to_string(),
            "a.json".to_string(),
            "--out".to_string(),
            "b.json".to_string(),
            "--emit-events".to_string(),
        ];

        let cmd = Cli::parse(&args).expect("parse");
        match cmd {
            Cli::BookmarksNormalize {
                input,
                output,
                emit_events,
                backup,
            } => {
                assert_eq!(input, "a.json");
                assert_eq!(output, "b.json");
                assert!(emit_events);
                assert!(!backup);
            }
            _ => panic!("expected normalize"),
        }
    }

    #[test]
    fn parse_validate_success() {
        let args = vec![
            "bin".to_string(),
            "bookmarks".to_string(),
            "validate".to_string(),
            "--in".to_string(),
            "a.json".to_string(),
        ];

        let cmd = Cli::parse(&args).expect("parse");
        match cmd {
            Cli::BookmarksValidate { input } => assert_eq!(input, "a.json"),
            _ => panic!("expected validate"),
        }
    }

    #[test]
    fn parse_help_returns_error_with_usage() {
        let args = vec![
            "bin".to_string(),
            "bookmarks".to_string(),
            "normalize".to_string(),
            "--help".to_string(),
        ];
        let err = Cli::parse(&args).unwrap_err().to_string();
        assert!(err.contains("Usage"));
    }

    #[tokio::test]
    async fn run_with_args_smoke_writes_output() {
        let dir = tempdir().expect("tempdir");
        let input_path = dir.path().join("Bookmarks.json");
        let output_path = dir.path().join("Bookmarks.out.json");

        let dto = BookmarksFileDto {
            roots: BTreeMap::from([(
                "bookmark_bar".to_string(),
                BookmarkNodeDto {
                    node_type: "folder".to_string(),
                    name: Some("bar".to_string()),
                    children: vec![],
                    ..BookmarkNodeDto::default()
                },
            )]),
            ..BookmarksFileDto::default()
        };

        std::fs::write(
            &input_path,
            serde_json::to_string_pretty(&dto).expect("serialize"),
        )
        .expect("write input");

        let args = vec![
            "bin".to_string(),
            "bookmarks".to_string(),
            "normalize".to_string(),
            "--in".to_string(),
            input_path.to_str().unwrap().to_string(),
            "--out".to_string(),
            output_path.to_str().unwrap().to_string(),
        ];

        run_with_args(&args).await.expect("run");
        assert!(output_path.exists());

        let raw_out = std::fs::read_to_string(&output_path).expect("read output");
        let parsed: serde_json::Value = serde_json::from_str(&raw_out).expect("valid json");
        assert!(parsed.get("roots").is_some());
    }

    #[tokio::test]
    async fn run_with_args_smoke_emit_events_writes_output() {
        let dir = tempdir().expect("tempdir");
        let input_path = dir.path().join("Bookmarks.json");
        let output_path = dir.path().join("Bookmarks.out.json");

        let dto = BookmarksFileDto {
            roots: BTreeMap::from([(
                "bookmark_bar".to_string(),
                BookmarkNodeDto {
                    node_type: "folder".to_string(),
                    name: Some("bar".to_string()),
                    children: vec![],
                    ..BookmarkNodeDto::default()
                },
            )]),
            ..BookmarksFileDto::default()
        };

        std::fs::write(
            &input_path,
            serde_json::to_string_pretty(&dto).expect("serialize"),
        )
        .expect("write input");

        let args = vec![
            "bin".to_string(),
            "bookmarks".to_string(),
            "normalize".to_string(),
            "--in".to_string(),
            input_path.to_str().unwrap().to_string(),
            "--out".to_string(),
            output_path.to_str().unwrap().to_string(),
            "--emit-events".to_string(),
        ];

        run_with_args(&args).await.expect("run");
        assert!(output_path.exists());
    }

    #[tokio::test]
    async fn run_uses_env_args_and_returns_usage_error_under_test_harness() {
        let err = run().await.unwrap_err().to_string();
        assert!(err.contains("Usage"));
    }

    #[tokio::test]
    async fn run_with_args_validate_smoke_ok() {
        let dir = tempdir().expect("tempdir");
        let input_path = dir.path().join("Bookmarks.json");

        let dto = BookmarksFileDto {
            roots: BTreeMap::from([(
                "bookmark_bar".to_string(),
                BookmarkNodeDto {
                    node_type: "folder".to_string(),
                    name: Some("bar".to_string()),
                    children: vec![BookmarkNodeDto {
                        node_type: "url".to_string(),
                        url: Some("https://example.com".to_string()),
                        ..BookmarkNodeDto::default()
                    }],
                    ..BookmarkNodeDto::default()
                },
            )]),
            ..BookmarksFileDto::default()
        };

        std::fs::write(
            &input_path,
            serde_json::to_string_pretty(&dto).expect("serialize"),
        )
        .expect("write input");

        let args = vec![
            "bin".to_string(),
            "bookmarks".to_string(),
            "validate".to_string(),
            "--in".to_string(),
            input_path.to_str().unwrap().to_string(),
        ];

        run_with_args(&args).await.expect("validate");
    }

    #[tokio::test]
    async fn run_with_args_refuses_overwrite_without_backup() {
        let dir = tempdir().expect("tempdir");
        let input_path = dir.path().join("Bookmarks.json");

        let dto = BookmarksFileDto {
            roots: BTreeMap::from([(
                "bookmark_bar".to_string(),
                BookmarkNodeDto {
                    node_type: "folder".to_string(),
                    name: Some("bar".to_string()),
                    children: vec![],
                    ..BookmarkNodeDto::default()
                },
            )]),
            ..BookmarksFileDto::default()
        };

        std::fs::write(
            &input_path,
            serde_json::to_string_pretty(&dto).expect("serialize"),
        )
        .expect("write input");

        let args = vec![
            "bin".to_string(),
            "bookmarks".to_string(),
            "normalize".to_string(),
            "--in".to_string(),
            input_path.to_str().unwrap().to_string(),
            "--out".to_string(),
            input_path.to_str().unwrap().to_string(),
        ];

        let err = run_with_args(&args).await.unwrap_err().to_string();
        assert!(err.contains("--backup"));
    }

    #[tokio::test]
    async fn run_with_args_overwrite_with_backup_creates_backup_file() {
        let dir = tempdir().expect("tempdir");
        let input_path = dir.path().join("Bookmarks.json");

        let dto = BookmarksFileDto {
            roots: BTreeMap::from([(
                "bookmark_bar".to_string(),
                BookmarkNodeDto {
                    node_type: "folder".to_string(),
                    name: Some("bar".to_string()),
                    children: vec![],
                    ..BookmarkNodeDto::default()
                },
            )]),
            ..BookmarksFileDto::default()
        };

        std::fs::write(
            &input_path,
            serde_json::to_string_pretty(&dto).expect("serialize"),
        )
        .expect("write input");

        let args = vec![
            "bin".to_string(),
            "bookmarks".to_string(),
            "normalize".to_string(),
            "--in".to_string(),
            input_path.to_str().unwrap().to_string(),
            "--out".to_string(),
            input_path.to_str().unwrap().to_string(),
            "--backup".to_string(),
        ];

        run_with_args(&args).await.expect("run");

        let mut found_backup = false;
        for entry in std::fs::read_dir(dir.path()).expect("read_dir") {
            let entry = entry.expect("entry");
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("Bookmarks.json.bak.") {
                found_backup = true;
            }
        }
        assert!(found_backup);
    }
}
