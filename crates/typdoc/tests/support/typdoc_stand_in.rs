//! Stand-in for `typdoc` on the shell examples harness's `PATH`. It is never the real CLI:
//! it does nothing but record what it was called with, so a test can compare that with what
//! an example expects a shell to pass through. The harness places it on `PATH` under the name
//! `typdoc`, and points it at an output file with the environment variable `STAND_IN_OUT_FILE`.
//! It records its arguments and every environment variable whose name starts with `TYPDOC_`,
//! as one JSON object: `{ "args": [...], "env": { ... } }`.

use std::collections::BTreeMap;
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let env: BTreeMap<String, String> = std::env::vars()
        .filter(|(name, _)| name.starts_with("TYPDOC_"))
        .collect();
    let out_path = std::env::var("STAND_IN_OUT_FILE")
        .expect("the harness sets STAND_IN_OUT_FILE before calling the stand-in");
    let recorded = serde_json::json!({ "args": args, "env": env });
    let mut file = std::fs::File::create(&out_path)
        .unwrap_or_else(|e| panic!("cannot create {out_path}: {e}"));
    file.write_all(recorded.to_string().as_bytes())
        .unwrap_or_else(|e| panic!("cannot write {out_path}: {e}"));
}
