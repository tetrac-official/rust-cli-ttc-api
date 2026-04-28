//! Cross-platform launcher tests for `.claude/skills/skill-trading/scripts/skill-trading`.
//!
//! The launcher picks the right prebuilt binary based on `uname -s`/`-m`.
//! These tests stub `uname` via PATH and drop stub binaries in the launcher's
//! directory to verify the dispatch logic on every (OS, ARCH) combination
//! the launcher supports — without needing the real binaries on disk.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

/// Path to the real launcher script in this repo.
fn launcher_source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(".claude/skills/skill-trading/scripts/skill-trading")
}

/// Build a unique per-test sandbox containing:
/// - A copy of the launcher script (so its `$0` resolves to the temp dir).
/// - A `bin/` subdir with a fake `uname` honoring FAKE_OS / FAKE_ARCH env vars.
struct Sandbox {
    root: PathBuf,
    launcher: PathBuf,
    fake_path_dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let id = format!(
            "launcher-test-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        );
        let root = std::env::temp_dir().join(id);
        fs::create_dir_all(&root).unwrap();

        // Copy the launcher to the sandbox so `dirname "$0"` points here —
        // that's how the launcher locates its sibling binaries.
        let launcher = root.join("skill-trading");
        fs::copy(launcher_source(), &launcher).expect("copy launcher");
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();

        // Fake `uname` lives in a separate dir we prepend to PATH.
        let fake_path_dir = root.join("bin");
        fs::create_dir_all(&fake_path_dir).unwrap();
        let fake_uname = fake_path_dir.join("uname");
        fs::write(
            &fake_uname,
            r#"#!/bin/sh
case "$1" in
  -s) printf "%s\n" "${FAKE_OS:-Linux}" ;;
  -m) printf "%s\n" "${FAKE_ARCH:-x86_64}" ;;
  *)  printf "fake-uname-unsupported\n" ;;
esac
"#,
        )
        .unwrap();
        fs::set_permissions(&fake_uname, fs::Permissions::from_mode(0o755)).unwrap();

        Self {
            root,
            launcher,
            fake_path_dir,
        }
    }

    /// Drop an executable stub at `<root>/<name>` that prints a distinctive
    /// line so tests can assert which binary was exec'd.
    fn add_stub_binary(&self, name: &str) {
        let path = self.root.join(name);
        fs::write(
            &path,
            format!(
                "#!/bin/sh\nprintf \"stub:%s args=%s\\n\" \"{}\" \"$*\"\n",
                name
            ),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// Run the launcher with stubbed `uname` and given OS/ARCH.
    fn run(&self, os: &str, arch: &str, args: &[&str]) -> std::process::Output {
        let path = format!(
            "{}:{}",
            self.fake_path_dir.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        Command::new(&self.launcher)
            .args(args)
            .env("PATH", path)
            .env("FAKE_OS", os)
            .env("FAKE_ARCH", arch)
            .output()
            .expect("launcher must spawn")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn stdout(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}
fn stderr(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

// ============================================================================
// Supported platforms — happy-path dispatch
// ============================================================================

#[test]
fn darwin_arm64_dispatches_to_darwin_arm64_binary() {
    let s = Sandbox::new("darwin-arm64");
    s.add_stub_binary("skill-trading-darwin-arm64");
    let out = s.run("Darwin", "arm64", &["info"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("stub:skill-trading-darwin-arm64"),
        "got stdout: {}",
        stdout(&out)
    );
    assert!(stdout(&out).contains("args=info"), "args must be forwarded");
}

#[test]
fn darwin_x86_64_dispatches_to_darwin_x64_binary() {
    let s = Sandbox::new("darwin-x64");
    s.add_stub_binary("skill-trading-darwin-x64");
    let out = s.run("Darwin", "x86_64", &[]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("stub:skill-trading-darwin-x64"));
}

#[test]
fn linux_x86_64_dispatches_to_linux_x64_binary() {
    let s = Sandbox::new("linux-x86_64");
    s.add_stub_binary("skill-trading-linux-x64");
    let out = s.run("Linux", "x86_64", &[]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("stub:skill-trading-linux-x64"));
}

#[test]
fn linux_amd64_alias_dispatches_to_same_linux_x64_binary() {
    // Some distros report amd64 instead of x86_64 — the launcher case statement
    // accepts both.
    let s = Sandbox::new("linux-amd64");
    s.add_stub_binary("skill-trading-linux-x64");
    let out = s.run("Linux", "amd64", &[]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("stub:skill-trading-linux-x64"));
}

#[test]
fn linux_aarch64_dispatches_to_linux_arm64_binary() {
    // CLAUDE.md notes: macOS reports arm64, Linux reports aarch64 — the
    // launcher must accept both for the same physical CPU family.
    let s = Sandbox::new("linux-aarch64");
    s.add_stub_binary("skill-trading-linux-arm64");
    let out = s.run("Linux", "aarch64", &[]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("stub:skill-trading-linux-arm64"));
}

#[test]
fn linux_arm64_alias_dispatches_to_same_linux_arm64_binary() {
    let s = Sandbox::new("linux-arm64");
    s.add_stub_binary("skill-trading-linux-arm64");
    let out = s.run("Linux", "arm64", &[]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("stub:skill-trading-linux-arm64"));
}

// ============================================================================
// CLI argument forwarding
// ============================================================================

#[test]
fn forwards_multiple_args_to_the_real_binary() {
    let s = Sandbox::new("forward-args");
    s.add_stub_binary("skill-trading-linux-x64");
    let out = s.run("Linux", "x86_64", &["account", "balance", "-e", "orderly"]);
    assert!(out.status.success());
    let stdout_str = stdout(&out);
    assert!(stdout_str.contains("account"));
    assert!(stdout_str.contains("balance"));
    assert!(stdout_str.contains("-e"));
    assert!(stdout_str.contains("orderly"));
}

// ============================================================================
// Failure paths — useful errors for agents and humans
// ============================================================================

#[test]
fn unsupported_platform_exits_non_zero_with_clear_message() {
    let s = Sandbox::new("unsupported");
    // No stubs added — but the OS/ARCH wouldn't match anyway.
    let out = s.run("FreeBSD", "amd64", &[]);
    assert!(!out.status.success(), "must fail for unsupported platform");
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("unsupported platform FreeBSD-amd64"),
        "stderr should name the offending combo, got: {err}"
    );
}

#[test]
fn unsupported_platform_lists_available_binaries_when_present() {
    // Helpful for an agent debugging "why won't it run on this machine":
    // the launcher should list what IS bundled.
    let s = Sandbox::new("unsupported-with-stubs");
    s.add_stub_binary("skill-trading-darwin-arm64");
    s.add_stub_binary("skill-trading-linux-x64");
    let out = s.run("Plan9", "ppc64", &[]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("available binaries"));
    assert!(err.contains("skill-trading-darwin-arm64"));
    assert!(err.contains("skill-trading-linux-x64"));
}

#[test]
fn missing_bundled_binary_exits_non_zero_with_clear_message() {
    // OS+ARCH is supported but the matching binary file isn't shipped.
    // Common scenario: `make release-all` was skipped, so linux-x64 ran
    // on a darwin-arm64 dev machine but no linux-x64 binary is present.
    let s = Sandbox::new("missing-bundle");
    // Deliberately do NOT add the darwin-arm64 stub.
    s.add_stub_binary("skill-trading-linux-x64"); // present but not the matching one
    let out = s.run("Darwin", "arm64", &[]);
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("not bundled"),
        "stderr should mention 'not bundled', got: {err}"
    );
    assert!(
        err.contains("Darwin-arm64"),
        "stderr should name the missing combo, got: {err}"
    );
    // And it should still list what IS bundled — agent recovery hint.
    assert!(err.contains("available binaries"));
    assert!(err.contains("skill-trading-linux-x64"));
}

#[test]
fn missing_bundle_lists_no_binaries_section_when_dir_is_empty() {
    // Edge case: launcher invoked from a stripped-down install with NO
    // binaries at all. The `ls | grep ... || true` should keep the launcher
    // from crashing on the listing step.
    let s = Sandbox::new("empty-dir");
    let out = s.run("Darwin", "arm64", &[]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("not bundled"), "got: {err}");
    // No grep match means the listing is empty — but the launcher must still
    // exit cleanly with the "not bundled" message, not 141 (SIGPIPE) or
    // similar from a failed pipeline.
    assert_eq!(out.status.code(), Some(1));
}

// ============================================================================
// Exit code propagation — agents rely on these
// ============================================================================

#[test]
fn exit_code_from_real_binary_is_propagated() {
    // Replace the stub with one that exits with a specific non-zero code so
    // we can verify the launcher exec's it (rather than wrapping/swallowing
    // the exit code).
    let s = Sandbox::new("exit-code");
    let stub = s.root.join("skill-trading-linux-x64");
    fs::write(&stub, "#!/bin/sh\nexit 42\n").unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    let out = s.run("Linux", "x86_64", &[]);
    assert_eq!(out.status.code(), Some(42), "exec must propagate exit code");
}
