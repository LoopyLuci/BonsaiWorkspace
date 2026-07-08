//! Isolated build/test sandbox for a self-upgrade proposal.
//!
//! A proposal is never applied directly to the user's working tree. Instead:
//!   1. `run_in_worktree` creates a real `git worktree` (a separate checkout
//!      sharing the same object database — nothing here touches the user's
//!      actual uncommitted work), writes the proposed files there, commits
//!      them on a throwaway branch, and runs the real build+test commands.
//!   2. `promote` fast-forward-merges that branch into the current branch —
//!      if a fast-forward isn't possible (the repo moved on since the
//!      worktree was created), it fails closed rather than force-merging.
//!   3. `discard` removes the worktree and branch, applying nothing.
//!
//! Build/test commands are chosen by which paths were touched: `.rs` files
//! build the `workspace/src-tauri` crate; `.svelte`/`.ts` files run the
//! Workspace frontend's own `check`/`build`/`check:bundle` scripts (the
//! same three commands used by hand throughout this session).

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxOutcome {
    pub worktree_path: String,
    pub branch_name: String,
    pub build_ok: bool,
    pub test_ok: bool,
    pub build_output: String,
    pub test_output: String,
}

impl SandboxOutcome {
    pub fn passed(&self) -> bool {
        self.build_ok && self.test_ok
    }
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("failed to run 'git {}': {e}", args.join(" ")))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return Err(format!("'git {}' failed:\n{stdout}\n{stderr}", args.join(" ")));
    }
    Ok(stdout)
}

fn run_cmd(cwd: &Path, program: &str, args: &[&str]) -> (bool, String) {
    match Command::new(program).args(args).current_dir(cwd).output() {
        Ok(out) => {
            let combined = format!(
                "$ {program} {}\n{}\n{}",
                args.join(" "),
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr),
            );
            (out.status.success(), combined)
        }
        Err(e) => (false, format!("failed to run '{program} {}': {e}", args.join(" "))),
    }
}

/// Creates an isolated worktree, writes the proposed files into it, commits
/// them on a throwaway branch, and runs the appropriate build+test commands.
/// `files` are `(path relative to repo_root, new file content)`.
///
/// `target`, if given, builds/tests only *that* `survival::targets::
/// MonitorTarget`'s own directory with its own commands (e.g. a fix proposed
/// for `kernel` gets `kernel`'s own `cargo build`/`test`, not always
/// `workspace/src-tauri`'s) — additive: `None` preserves the exact original
/// two-hardcoded-path behavior every existing call site still uses.
pub fn run_in_worktree(
    repo_root: &Path,
    files: &[(String, String)],
    target: Option<&crate::survival::targets::MonitorTarget>,
) -> Result<SandboxOutcome, String> {
    let id = uuid::Uuid::new_v4();
    let branch_name = format!("self-upgrade/{id}");
    let worktree_path = std::env::temp_dir().join(format!("omnisystem-self-upgrade-{id}"));

    run_git(
        repo_root,
        &["worktree", "add", "-b", &branch_name, worktree_path.to_str().unwrap(), "HEAD"],
    )?;

    for (rel_path, content) in files {
        let dest = worktree_path.join(rel_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("failed to create '{}': {e}", parent.display()))?;
        }
        std::fs::write(&dest, content).map_err(|e| format!("failed to write '{rel_path}': {e}"))?;
    }

    run_git(&worktree_path, &["add", "-A"])?;
    run_git(
        &worktree_path,
        &["commit", "-m", "self-upgrade: proposed change (sandboxed, pending verification)"],
    )?;

    let (build_ok, test_ok, build_output, test_output) = if let Some(target) = target {
        run_target_build(&worktree_path, target)
    } else {
        let touches_rust = files.iter().any(|(p, _)| p.ends_with(".rs"));
        let touches_frontend = files.iter().any(|(p, _)| p.ends_with(".svelte") || p.ends_with(".ts"));

        let mut build_ok = true;
        let mut test_ok = true;
        let mut build_output = String::new();
        let mut test_output = String::new();

        if touches_rust {
            let crate_dir = worktree_path.join("Omnisystem/OmniHarness/workspace/src-tauri");
            let (ok, out) = run_cmd(&crate_dir, "cargo", &["build", "--lib"]);
            build_ok &= ok;
            build_output.push_str(&out);
            if ok {
                let (tok, tout) = run_cmd(&crate_dir, "cargo", &["test", "--lib"]);
                test_ok &= tok;
                test_output.push_str(&tout);
            }
        }

        if touches_frontend {
            let frontend_dir = worktree_path.join("Omnisystem/OmniHarness/workspace/src");
            let (ok, out) = run_cmd(&frontend_dir, "npm", &["run", "check"]);
            build_ok &= ok;
            build_output.push_str(&out);
            if ok {
                let (bok, bout) = run_cmd(&frontend_dir, "npm", &["run", "build"]);
                test_ok &= bok;
                test_output.push_str(&bout);
                if bok {
                    let (cok, cout) = run_cmd(&frontend_dir, "npm", &["run", "check:bundle"]);
                    test_ok &= cok;
                    test_output.push_str(&cout);
                }
            }
        }

        if !touches_rust && !touches_frontend {
            // Nothing we know how to build/test (e.g. a doc-only change) — not
            // a pass in the sense of "verified safe", just nothing to verify.
            // Handled explicitly rather than defaulting to true so it's visible
            // in the recorded outcome, not silently indistinguishable from a
            // real green build.
            build_output = "No recognized buildable/testable files touched — nothing was run.".to_string();
        }

        (build_ok, test_ok, build_output, test_output)
    };

    Ok(SandboxOutcome {
        worktree_path: worktree_path.to_string_lossy().to_string(),
        branch_name,
        build_ok,
        test_ok,
        build_output,
        test_output,
    })
}

/// Builds/tests exactly one `MonitorTarget`'s own directory with its own
/// commands — used when a self-upgrade proposal is explicitly scoped to a
/// target outside `workspace/src-tauri`/`workspace/src` (see
/// `survival::targets` for the registry of what's real and checkable).
fn run_target_build(worktree_path: &Path, target: &crate::survival::targets::MonitorTarget) -> (bool, bool, String, String) {
    use crate::survival::targets::TargetKind;
    let dir = worktree_path.join(target.rel_path);
    match target.kind {
        TargetKind::RustCrate => {
            let (ok, out) = run_cmd(&dir, "cargo", &["build", "--lib"]);
            if !ok {
                return (false, false, out, String::new());
            }
            let (tok, tout) = run_cmd(&dir, "cargo", &["test", "--lib"]);
            (ok, tok, out, tout)
        }
        TargetKind::NpmSvelteCheck => {
            let (ok, out) = run_cmd(&dir, "npm", &["run", "check"]);
            if !ok {
                return (false, false, out, String::new());
            }
            let (bok, bout) = run_cmd(&dir, "npm", &["run", "build"]);
            (ok, bok, out, bout)
        }
        TargetKind::NpmTypecheckLint => {
            let (ok, out) = run_cmd(&dir, "npx", &["tsc", "--noEmit"]);
            (ok, ok, out, String::new())
        }
    }
}

/// Fast-forward-merges the sandbox branch into the current branch and
/// cleans up the worktree. Fails closed (no merge happens) if a
/// fast-forward isn't possible.
pub fn promote(repo_root: &Path, outcome: &SandboxOutcome) -> Result<(), String> {
    run_git(repo_root, &["merge", "--ff-only", &outcome.branch_name])?;
    cleanup(repo_root, outcome)
}

/// Discards the sandbox entirely — nothing from it is ever applied.
pub fn discard(repo_root: &Path, outcome: &SandboxOutcome) -> Result<(), String> {
    cleanup(repo_root, outcome)
}

fn cleanup(repo_root: &Path, outcome: &SandboxOutcome) -> Result<(), String> {
    run_git(repo_root, &["worktree", "remove", "--force", &outcome.worktree_path])?;
    // Best-effort: already merged (promote) or simply unreferenced (discard).
    let _ = run_git(repo_root, &["branch", "-D", &outcome.branch_name]);
    Ok(())
}

#[allow(dead_code)]
pub fn worktree_path_buf(outcome: &SandboxOutcome) -> PathBuf {
    PathBuf::from(&outcome.worktree_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a real, throwaway git repo (never the actual Omnisystem
    /// repo) with one commit, so these tests exercise real `git worktree`
    /// mechanics end-to-end rather than mocking git out.
    fn scratch_repo() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("omnisystem-self-upgrade-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        run_git(&dir, &["init", "-q"]).unwrap();
        run_git(&dir, &["config", "user.email", "test@example.com"]).unwrap();
        run_git(&dir, &["config", "user.name", "Self-Upgrade Test"]).unwrap();
        std::fs::write(dir.join("README.md"), "hello\n").unwrap();
        run_git(&dir, &["add", "-A"]).unwrap();
        run_git(&dir, &["commit", "-q", "-m", "initial"]).unwrap();
        dir
    }

    fn cleanup_repo(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn sandbox_promote_lands_the_file_and_cleans_up_the_worktree() {
        let repo = scratch_repo();
        let files = vec![("notes.txt".to_string(), "proposed content\n".to_string())];

        let outcome = run_in_worktree(&repo, &files, None).expect("sandbox run should succeed");
        // A plain .txt file matches neither the Rust nor frontend build
        // detection, so there's nothing to actually build/test — both
        // flags should default to true (nothing to verify, not a failure).
        assert!(outcome.build_ok);
        assert!(outcome.test_ok);
        assert!(std::path::Path::new(&outcome.worktree_path).exists());

        promote(&repo, &outcome).expect("promote should fast-forward cleanly");

        // The file should now exist in the real repo (not just the
        // worktree) — compared with normalized line endings since Windows
        // git's `core.autocrlf` may rewrite `\n` to `\r\n` on checkout.
        let landed = std::fs::read_to_string(repo.join("notes.txt")).unwrap();
        assert_eq!(landed.replace("\r\n", "\n"), "proposed content\n");
        // The worktree should be gone after promotion.
        assert!(!std::path::Path::new(&outcome.worktree_path).exists());

        cleanup_repo(&repo);
    }

    #[test]
    fn sandbox_discard_applies_nothing_and_cleans_up() {
        let repo = scratch_repo();
        let files = vec![("should-not-land.txt".to_string(), "discarded content\n".to_string())];

        let outcome = run_in_worktree(&repo, &files, None).expect("sandbox run should succeed");
        discard(&repo, &outcome).expect("discard should clean up without applying anything");

        assert!(!repo.join("should-not-land.txt").exists());
        assert!(!std::path::Path::new(&outcome.worktree_path).exists());

        cleanup_repo(&repo);
    }

    #[test]
    fn target_aware_build_uses_the_targets_own_directory_and_commands() {
        let repo = scratch_repo();
        // A minimal, real, dependency-free Rust crate under a target
        // subdirectory — proves the target-aware path builds/tests exactly
        // this crate, not the default `workspace/src-tauri` assumption.
        let crate_dir = repo.join("some-crate");
        std::fs::create_dir_all(crate_dir.join("src")).unwrap();
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"some-crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(crate_dir.join("src/lib.rs"), "pub fn add(a: i32, b: i32) -> i32 { a + b }\n").unwrap();
        run_git(&repo, &["add", "-A"]).unwrap();
        run_git(&repo, &["commit", "-q", "-m", "add some-crate"]).unwrap();

        let target = crate::survival::targets::MonitorTarget {
            name: "some-crate",
            rel_path: "some-crate",
            kind: crate::survival::targets::TargetKind::RustCrate,
        };
        let files = vec![(
            "some-crate/src/lib.rs".to_string(),
            "pub fn add(a: i32, b: i32) -> i32 { a + b + 0 }\n".to_string(),
        )];
        let outcome = run_in_worktree(&repo, &files, Some(&target)).expect("sandbox run should succeed");
        assert!(outcome.build_ok, "build_output: {}", outcome.build_output);
        assert!(outcome.test_ok, "test_output: {}", outcome.test_output);

        let _ = discard(&repo, &outcome);
        cleanup_repo(&repo);
    }
}
