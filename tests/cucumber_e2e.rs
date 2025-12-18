use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use cucumber::{World as _, given, then, when};
use tempfile::TempDir;

#[derive(Debug, Default, cucumber::World)]
struct TestWorld {
    dir: Option<TempDir>,
    input_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    output_path_2: Option<PathBuf>,
    last_cmd: Option<Output>,
}

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_microsoft-edge-bookmark-sorter-flattener")
}

fn run_cmd(args: Vec<String>) -> Output {
    Command::new(exe())
        .args(args)
        .output()
        .expect("failed to run bookmarks binary")
}

fn stderr_string(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn write_fixture(path: &Path) {
    // Minimal Chrome/Edge-ish bookmark JSON matching our DTOs.
    // Intentionally includes:
    // - duplicate folder name "Z" (nested under another root) for merge testing
    // - duplicate URL differing by fragment/case for URL canonicalization + dedup
    let json = r#"{
  "checksum": "x",
  "roots": {
    "bookmark_bar": {
      "type": "folder",
      "children": [
        {
          "type": "folder",
          "name": "Z",
          "id": "1",
          "guid": "guid-z-winner",
          "date_added": "100",
          "children": [
            {
              "type": "url",
              "name": "keep-me",
              "id": "10",
              "url": "http://EXAMPLE.com/page#frag",
              "visit_count": 1,
              "date_last_used": "100",
              "date_added": "10"
            },
            {
              "type": "url",
              "name": "also-same",
              "id": "11",
              "url": "http://example.com/page",
              "visit_count": 10,
              "date_last_used": "200",
              "date_added": "20"
            }
          ]
        }
      ]
    },
    "other": {
      "type": "folder",
      "children": [
        {
          "type": "folder",
          "name": "J",
          "id": "20",
          "date_added": "150",
          "children": [
            {
              "type": "folder",
              "name": "Z",
              "id": "2",
              "guid": "guid-z-loser",
              "date_added": "200",
              "children": [
                {
                  "type": "url",
                  "name": "other",
                  "id": "12",
                  "url": "http://example.com/other#x",
                  "date_added": "30"
                }
              ]
            }
          ]
        }
      ]
    }
  },
  "version": 1
}"#;

    fs::write(path, json).expect("write fixture")
}

fn find_backup_file(dir: &Path, input_file_name: &str) -> Option<PathBuf> {
    let prefix = format!("{input_file_name}.bak.");
    let entries = fs::read_dir(dir).ok()?;
    for ent in entries.flatten() {
        let file_name = ent.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.starts_with(&prefix) {
            return Some(ent.path());
        }
    }
    None
}

#[given("a temp bookmarks workspace")]
fn a_temp_bookmarks_workspace(world: &mut TestWorld) {
    world.dir = Some(tempfile::tempdir().expect("tempdir"));
}

#[given("an input bookmarks file with duplicates")]
fn an_input_bookmarks_file_with_duplicates(world: &mut TestWorld) {
    let dir = world.dir.as_ref().expect("temp dir");
    let input_path = dir.path().join("Bookmarks.json");
    write_fixture(&input_path);
    world.input_path = Some(input_path);
}

#[when("I run bookmarks normalize to an output file")]
fn i_run_bookmarks_normalize_to_an_output_file(world: &mut TestWorld) {
    let dir = world.dir.as_ref().expect("temp dir");
    let input_path = world.input_path.as_ref().expect("input");
    let output_path = dir.path().join("Out.json");

    let out = run_cmd(vec![
        "bookmarks".to_string(),
        "normalize".to_string(),
        "--in".to_string(),
        input_path.to_string_lossy().into_owned(),
        "--out".to_string(),
        output_path.to_string_lossy().into_owned(),
    ]);

    world.output_path = Some(output_path);
    world.last_cmd = Some(out);
}

#[when("I run bookmarks validate on the output file")]
fn i_run_bookmarks_validate_on_the_output_file(world: &mut TestWorld) {
    let output_path = world.output_path.as_ref().expect("output");

    let out = run_cmd(vec![
        "bookmarks".to_string(),
        "validate".to_string(),
        "--in".to_string(),
        output_path.to_string_lossy().into_owned(),
    ]);
    world.last_cmd = Some(out);
}

#[when("I run bookmarks normalize twice to two output files")]
fn i_run_bookmarks_normalize_twice_to_two_output_files(world: &mut TestWorld) {
    let dir = world.dir.as_ref().expect("temp dir");
    let input_path = world.input_path.as_ref().expect("input");

    let out1 = dir.path().join("Out1.json");
    let out2 = dir.path().join("Out2.json");

    let r1 = run_cmd(vec![
        "bookmarks".to_string(),
        "normalize".to_string(),
        "--in".to_string(),
        input_path.to_string_lossy().into_owned(),
        "--out".to_string(),
        out1.to_string_lossy().into_owned(),
    ]);
    assert!(r1.status.success(), "first normalize failed: {}", stderr_string(&r1));

    let r2 = run_cmd(vec![
        "bookmarks".to_string(),
        "normalize".to_string(),
        "--in".to_string(),
        input_path.to_string_lossy().into_owned(),
        "--out".to_string(),
        out2.to_string_lossy().into_owned(),
    ]);
    world.output_path = Some(out1);
    world.output_path_2 = Some(out2);
    world.last_cmd = Some(r2);
}

#[when("I run bookmarks normalize in place without backup")]
fn i_run_bookmarks_normalize_in_place_without_backup(world: &mut TestWorld) {
    let input_path = world.input_path.as_ref().expect("input");

    let out = run_cmd(vec![
        "bookmarks".to_string(),
        "normalize".to_string(),
        "--in".to_string(),
        input_path.to_string_lossy().into_owned(),
        "--out".to_string(),
        input_path.to_string_lossy().into_owned(),
    ]);

    world.last_cmd = Some(out);
}

#[when("I run bookmarks normalize in place with backup")]
fn i_run_bookmarks_normalize_in_place_with_backup(world: &mut TestWorld) {
    let input_path = world.input_path.as_ref().expect("input");

    let out = run_cmd(vec![
        "bookmarks".to_string(),
        "normalize".to_string(),
        "--in".to_string(),
        input_path.to_string_lossy().into_owned(),
        "--out".to_string(),
        input_path.to_string_lossy().into_owned(),
        "--backup".to_string(),
    ]);

    world.last_cmd = Some(out);
}

#[then("the command succeeds")]
fn the_command_succeeds(world: &mut TestWorld) {
    let out = world.last_cmd.as_ref().expect("last cmd");
    assert!(
        out.status.success(),
        "command failed (status={:?})\nstderr:\n{}\nstdout:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );

    // Spot-check output shape for the normalize command.
    // We keep this light so it stays resilient to internal refactors.
    if let Some(output_path) = world.output_path.as_ref() {
        if output_path.exists() {
            let raw = fs::read_to_string(output_path).expect("read output");
            let v: serde_json::Value = serde_json::from_str(&raw).expect("parse output json");

            // Expect a single merged Z folder somewhere in roots.
            let mut z_count = 0usize;
            let mut canonical_page_count = 0usize;

            let mut stack = Vec::new();
            if let Some(roots) = v.get("roots").and_then(|r| r.as_object()) {
                for root in roots.values() {
                    stack.push(root);
                }
            }

            while let Some(node) = stack.pop() {
                if let Some(obj) = node.as_object() {
                    if obj.get("type").and_then(|t| t.as_str()) == Some("folder")
                        && obj.get("name").and_then(|n| n.as_str()) == Some("Z")
                    {
                        z_count += 1;
                    }
                    if obj.get("type").and_then(|t| t.as_str()) == Some("url")
                        && obj.get("url").and_then(|u| u.as_str()) == Some("http://example.com/page")
                    {
                        canonical_page_count += 1;
                    }
                    if let Some(children) = obj.get("children").and_then(|c| c.as_array()) {
                        for ch in children {
                            stack.push(ch);
                        }
                    }
                }
            }

            assert_eq!(z_count, 1, "expected exactly one merged Z folder");
            assert_eq!(
                canonical_page_count, 1,
                "expected url dedup to leave one canonical page url"
            );
        }
    }
}

#[then("the command fails")]
fn the_command_fails(world: &mut TestWorld) {
    let out = world.last_cmd.as_ref().expect("last cmd");
    assert!(
        !out.status.success(),
        "expected failure but succeeded; stderr: {}",
        stderr_string(out)
    );
}

#[then(expr = "stderr mentions {string}")]
fn stderr_mentions(world: &mut TestWorld, needle: String) {
    let out = world.last_cmd.as_ref().expect("last cmd");
    let stderr = stderr_string(out);
    assert!(
        stderr.contains(&needle),
        "stderr did not contain {needle:?}. stderr was:\n{stderr}"
    );
}

#[then("the two outputs are identical")]
fn the_two_outputs_are_identical(world: &mut TestWorld) {
    let a = world.output_path.as_ref().expect("out1");
    let b = world.output_path_2.as_ref().expect("out2");

    let a_raw = fs::read_to_string(a).expect("read out1");
    let b_raw = fs::read_to_string(b).expect("read out2");

    assert_eq!(a_raw, b_raw, "normalize outputs differed");
}

#[then("a timestamped backup file exists")]
fn a_timestamped_backup_file_exists(world: &mut TestWorld) {
    let dir = world.dir.as_ref().expect("temp dir");
    let input_path = world.input_path.as_ref().expect("input");
    let file_name = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("utf-8 file name");

    let found = find_backup_file(dir.path(), file_name);
    assert!(
        found.is_some(),
        "expected a timestamped backup file like {file_name}.bak.<ts>"
    );
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    TestWorld::cucumber()
        .max_concurrent_scenarios(Some(1))
        .fail_on_skipped()
        .run_and_exit("tests/features")
        .await;
}
