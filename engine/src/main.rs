mod commands;
mod domain;
mod error;
mod sqlite_store;

use crate::{
    error::{err, ApiError},
    sqlite_store::Engine,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    env,
    io::{self, BufRead, Write},
    path::PathBuf,
};

const MAX_FRAME_BYTES: usize = 32 * 1024 * 1024;

#[derive(Deserialize)]
struct Request {
    version: Option<u64>,
    id: Option<String>,
    op: Option<String>,
    #[serde(default)]
    params: Value,
}

fn parse_args() -> Result<(PathBuf, Option<PathBuf>), String> {
    let mut args = env::args().skip(1);
    let mut dir = None;
    let mut legacy = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--data-dir" => dir = args.next().map(PathBuf::from),
            "--legacy" => legacy = args.next().map(PathBuf::from),
            "--help" | "-h" => {
                return Err(
                    "usage: memoivy-engine --data-dir ABS_PATH [--legacy ABS_NOTES_JSON]".into(),
                )
            }
            _ => return Err(format!("unknown argument: {a}")),
        }
    }
    let d = dir.ok_or("--data-dir is required")?;
    if !d.is_absolute() {
        return Err("--data-dir must be absolute".into());
    }
    if legacy.as_ref().is_some_and(|p| !p.is_absolute()) {
        return Err("--legacy must be absolute".into());
    }
    Ok((d, legacy))
}
fn write_error_line(id: &str, e: ApiError) -> io::Result<()> {
    let mut error = json!({"code":e.code,"message":e.message});
    if let Some(snapshot) = e.snapshot {
        error["snapshot"] = json!(snapshot);
    }
    println!("{}", json!({"version":1,"id":id,"error":error}));
    io::stdout().flush()
}
fn read_frame<R: BufRead>(reader: &mut R) -> io::Result<Option<Result<Vec<u8>, ()>>> {
    let mut out = Vec::new();
    let mut oversized = false;
    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            return if out.is_empty() && !oversized {
                Ok(None)
            } else if oversized {
                Ok(Some(Err(())))
            } else {
                Ok(Some(Ok(out)))
            };
        }
        let newline = buf.iter().position(|b| *b == b'\n');
        let len = newline.unwrap_or(buf.len());
        if !oversized {
            if out.len().saturating_add(len) > MAX_FRAME_BYTES {
                oversized = true;
                out.clear();
            } else {
                out.extend_from_slice(&buf[..len]);
            }
        }
        reader.consume(len + usize::from(newline.is_some()));
        if newline.is_some() {
            return Ok(Some(if oversized { Err(()) } else { Ok(out) }));
        }
    }
}
fn main() {
    let (dir, legacy) = match parse_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2)
        }
    };
    let mut engine = match Engine::open(&dir, legacy) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("engine startup failed: {e}");
            std::process::exit(1)
        }
    };
    let stdin = io::stdin();
    let mut input = stdin.lock();
    loop {
        let frame = match read_frame(&mut input) {
            Ok(Some(frame)) => frame,
            Ok(None) => break,
            Err(e) => {
                eprintln!("stdin error: {e}");
                break;
            }
        };
        let bytes = match frame {
            Ok(bytes) => bytes,
            Err(()) => {
                let _ = write_error_line("", err("FRAME_TOO_LARGE", "Request exceeds 32 MiB"));
                continue;
            }
        };
        let req: Request = match serde_json::from_slice(&bytes) {
            Ok(r) => r,
            Err(e) => {
                let _ = write_error_line("", err("INVALID_JSON", e.to_string()));
                continue;
            }
        };
        let id = req.id.unwrap_or_default();
        if req.version != Some(1) {
            let _ = write_error_line(&id, err("VERSION", "Unsupported protocol version"));
            continue;
        }
        let op = req.op.unwrap_or_default();
        match engine.dispatch(&op, &req.params) {
            Ok(result) => {
                let response = json!({"version":1,"id":id,"result":result});
                println!("{response}");
                let _ = io::stdout().flush();
                if !matches!(op.as_str(), "snapshot" | "flush" | "readAsset") {
                    if let Ok(s) = engine.snapshot() {
                        println!("{}", json!({"version":1,"event":"snapshot","snapshot":s}));
                        let _ = io::stdout().flush();
                    }
                }
            }
            Err(e) => {
                let _ = write_error_line(&id, e);
            }
        }
    }
}
