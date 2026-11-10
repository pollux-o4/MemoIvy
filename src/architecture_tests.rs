//! ADR-0010의 모듈 경계 검증. 담당이 다른 프런트엔드·백엔드가 서로의 내부에 의존하지 않게 한다.
use std::{fs, path::Path};

fn rust_sources(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push((
                path.display().to_string(),
                fs::read_to_string(&path).unwrap(),
            ));
        }
    }
}

fn sources(relative: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    rust_sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join(relative),
        &mut out,
    );
    assert!(!out.is_empty(), "{relative} must exist");
    out
}

#[test]
fn adr_0010_backend_does_not_know_the_frontend() {
    for (path, code) in sources("src/persistence") {
        for forbidden in ["gtk::", "use gtk", "crate::app", "crate::presentation"] {
            assert!(!code.contains(forbidden), "{path} depends on {forbidden}");
        }
    }
}

#[test]
fn adr_0010_canvas_reaches_storage_only_through_its_port() {
    for (path, code) in sources("src/app/canvas") {
        let allowed = path.ends_with("canvas/port.rs") || path.contains("canvas/tests/");
        if !allowed {
            assert!(
                !code.contains("persistence"),
                "{path} bypasses the canvas port"
            );
        }
    }
}

#[test]
fn adr_0010_canvas_model_stays_pure() {
    for (path, code) in sources("src/app/canvas/model") {
        for forbidden in ["gtk::", "use gtk", "persistence", "cairo"] {
            assert!(
                !code.contains(forbidden),
                "{path} must stay pure: {forbidden}"
            );
        }
    }
}

#[test]
fn adr_0010_frontend_uses_only_the_notebook_boundary() {
    for (path, code) in sources("src/app") {
        for forbidden in [
            "SaveQueue",
            "Store::",
            "save_queue",
            ".revision",
            ".transaction(",
        ] {
            assert!(
                !code.contains(forbidden),
                "{path} bypasses Notebook: {forbidden}"
            );
        }
    }
    let exports =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/persistence/mod.rs"))
            .unwrap();
    assert!(
        exports
            .lines()
            .filter(|line| line.contains("mod "))
            .all(|line| !line.trim_start().starts_with("pub")),
        "backend implementation modules must remain private"
    );
}

#[test]
fn adr_0010_features_coordinate_through_the_shell() {
    for feature in ["library", "editor", "canvas", "files", "shared"] {
        for (path, code) in sources(&format!("src/app/{feature}")) {
            for sibling in ["library", "editor", "canvas", "files"] {
                if sibling == feature {
                    continue;
                }
                for forbidden in [
                    format!("{sibling}::"),
                    format!("app::{sibling}"),
                    format!("super::{sibling}"),
                ] {
                    assert!(
                        !code.contains(&forbidden),
                        "{path} reaches sibling {sibling}; use shell commands/views"
                    );
                }
            }
        }
    }
}

#[test]
fn adr_0010_source_folders_keep_a_small_file_surface() {
    fn inspect(dir: &Path) {
        let entries: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        let files = entries.iter().filter(|p| p.is_file()).count();
        assert!(
            files <= 5,
            "{} has {files} files; group by responsibility",
            dir.display()
        );
        for child in entries.iter().filter(|p| p.is_dir()) {
            inspect(child);
        }
    }
    inspect(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
}

#[test]
fn adr_0010_shared_creation_keeps_policy_in_its_callers() {
    for (path, code) in sources("src/app/shared/creation") {
        for forbidden in ["shell::", "persistence::", "Notebook", "AppState"] {
            assert!(
                !code.contains(forbidden),
                "{path} owns application policy: {forbidden}"
            );
        }
    }
}

#[test]
fn ops_003_primary_gtk_suite_stays_complete_and_isolated() {
    let config =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(".mise.toml")).unwrap();
    let task = |name: &str| {
        config
            .split(&format!("[tasks.{name}]"))
            .nth(1)
            .unwrap_or_else(|| panic!("missing task {name}"))
            .split("\n[")
            .next()
            .unwrap()
    };
    assert!(
        task("test")
            .lines()
            .any(|line| line == "run = \"dbus-run-session -- xvfb-run -a cargo test --locked\""),
        "primary suite must run all tests on a virtual display"
    );
    assert!(
        task("check")
            .lines()
            .any(|line| line.trim() == "\"mise run test\","),
        "integration check must include the primary suite"
    );
}
