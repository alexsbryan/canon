// SPDX-License-Identifier: AGPL-3.0-or-later
//! An opt-in admission round; source history and canon-core remain unchanged.
use std::{env, fs, process};

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 6
        || !matches!(args[0].as_str(), "--basis" | "--arrangements")
        || args[2] != "--log"
        || args[4] != "--at"
    {
        return Err(
            "usage: canon-institution --basis <acts.jsonl>|--arrangements <manifest.json> --log <acts.jsonl> --at <unix-seconds>"
                .into(),
        );
    }
    let basis = fs::read_to_string(&args[1]).map_err(|e| e.to_string())?;
    let source = fs::read_to_string(&args[3]).map_err(|e| e.to_string())?;
    let at = args[5].parse().map_err(|_| "--at needs Unix seconds")?;
    let projected = if args[0] == "--basis" {
        canon_institution::project(&basis, &source, at)?
    } else {
        canon_institution::interpret(
            &canon_core::Log::parse(&source).map_err(|e| e.to_string())?,
            &serde_json::from_str(&basis).map_err(|e| e.to_string())?,
            at,
        )?
        .to_json()
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&projected).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("canon-institution: could not judge: {e}");
        process::exit(2);
    }
}
