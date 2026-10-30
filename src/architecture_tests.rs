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
    for (path, code) in sources("src/app/windows/canvas") {
        let allowed = path.ends_with("canvas/port.rs") || path.ends_with("canvas/tests.rs");
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
    for (path, code) in sources("src/app/windows/canvas/model") {
        for forbidden in ["gtk::", "use gtk", "persistence", "cairo"] {
            assert!(
                !code.contains(forbidden),
                "{path} must stay pure: {forbidden}"
            );
        }
    }
}
