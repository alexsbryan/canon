#!/usr/bin/env bash
# Check the core in isolation: workspace feature unification can otherwise
# enable serde/std through the CLI and hide a broken no_std dependency setup.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

python3 - <<'PY'
import json
import re
import subprocess
from pathlib import Path

# Runtime closure, not build-host derive macros. Changes to this list require
# explaining why the new dependency belongs inside the mechanical core.
APPROVED = {
    "canon-core", "serde", "serde_core", "serde_json", "sha2", "digest",
    "block-buffer", "crypto-common", "generic-array", "typenum", "cpufeatures",
    "libc", "cfg-if", "itoa", "memchr", "zmij",
}
DIRECT = {"serde", "serde_json", "sha2"}

def violations(tree):
    errors = []
    for line in tree.splitlines():
        package, features = line.split("|", 1)
        name = package.split()[0]
        enabled = set(features.replace(" (*)", "").split(",")) - {""}
        if name not in APPROVED:
            errors.append(f"unapproved runtime dependency: {name}")
        if "std" in enabled:
            errors.append(f"{name} enables std in the isolated core")
        if name == "serde_json" and "preserve_order" in enabled:
            errors.append("serde_json/preserve_order changes canonical annotation rendering")
    return errors

# A gate that only ever sees the passing graph can accidentally become inert.
# These negative controls run before inspecting the actual graph.
assert not violations("canon-core v0.2.0|\nserde v1|alloc,derive")
assert violations("serde v1|alloc,std")
assert violations("ureq v2|tls")
assert violations("serde_json v1|alloc,preserve_order")

metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version", "1", "--no-deps"], text=True,
))
core = next(p for p in metadata["packages"] if p["name"] == "canon-core")
deps = [d for d in core["dependencies"] if d["kind"] is None]
errors = []
source_dir = Path(core["manifest_path"]).parent / "src"
root = (source_dir / "lib.rs").read_text()
for attribute in ("#![no_std]", "#![forbid(unsafe_code)]"):
    if not re.search(r"^" + re.escape(attribute) + r"$", root, re.MULTILINE):
        errors.append(f"core must retain {attribute}")
for path in source_dir.rglob("*.rs"):
    source = path.read_text()
    if path.name == "lib.rs":
        source = source.replace("#[cfg(test)]\nextern crate std;", "")
    if re.search(r"\bextern\s+crate\s+std\b", source):
        errors.append(f"{path.name} imports std outside the test-harness exception")
if {d["name"] for d in deps} != DIRECT:
    errors.append("core normal dependencies must remain serde, serde_json, and sha2")
for dep in deps:
    if dep["uses_default_features"]:
        errors.append(f"{dep['name']} must explicitly disable default features")
tree = subprocess.check_output([
    "cargo", "tree", "--package", "canon-core", "--edges", "normal,no-proc-macro",
    "--prefix", "none", "--format", "{p}|{f}",
], text=True)
errors.extend(violations(tree))
if errors:
    raise SystemExit("core-boundary:\n  " + "\n  ".join(sorted(set(errors))))
print("core-boundary: approved isolated runtime graph; no std or preserve_order")
PY

# This is additional to the full-workspace lint/test gates. It compiles the
# normal library without the CLI's features or a test harness importing std.
cargo clippy --package canon-core --lib -- -D warnings
