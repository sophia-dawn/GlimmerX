#!/usr/bin/env python3
"""Regenerate the Rust user-facing message table from the frontend i18n files.

Run from anywhere:  python tools/gen_error_messages.py

The zh/en `errors` sections in src/i18n/locales/*.json are the single source
of truth for backend error messages. This keeps the frontend and the Rust
backend in sync (AGENTS.md: backend produces friendly messages, frontend
displays them verbatim).
"""

import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "src-tauri/src/errors/messages.rs"


def flatten(obj, prefix=""):
    out = {}
    for key, value in obj.items():
        path = f"{prefix}.{key}" if prefix else key
        if isinstance(value, dict):
            out.update(flatten(value, path))
        else:
            out[path] = value
    return out


def main():
    tables = {}
    for lang in ("zh", "en"):
        path = ROOT / f"src/i18n/locales/{lang}.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        tables[lang] = flatten(data["errors"])

    if set(tables["zh"]) != set(tables["en"]):
        missing = set(tables["zh"]) ^ set(tables["en"])
        raise SystemExit(f"zh/en error keys differ: {sorted(missing)}")

    lines = [
        "// Auto-generated from src/i18n/locales/{zh,en}.json (errors section).",
        "// Do not edit by hand; regenerate with tools/gen_error_messages.py.",
        "",
        "#[derive(Clone, Copy)]",
        "pub enum Lang {",
        "    Zh,",
        "    En,",
        "}",
        "",
        "pub fn lookup(lang: Lang, key: &str) -> Option<&'static str> {",
        "    match (lang, key) {",
    ]
    for key in sorted(tables["zh"]):
        for variant, lang in (("Zh", "zh"), ("En", "en")):
            value = json.dumps(tables[lang][key], ensure_ascii=False)
            lines.append(
                f"        (Lang::{variant}, {json.dumps(key, ensure_ascii=False)}) => Some({value}),"
            )
    lines += ["        _ => None,", "    }", "}", ""]
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text("\n".join(lines), encoding="utf-8", newline="\n")
    # Long English messages need rustfmt's line wrapping; keep the generated
    # file identical to what `cargo fmt --check` expects.
    if shutil.which("rustfmt"):
        subprocess.run(
            ["rustfmt", "--edition", "2021", str(OUTPUT)],
            check=False,
        )
    else:
        print("rustfmt not found; run `cargo fmt` after regenerating")
    print(f"wrote {OUTPUT.relative_to(ROOT)} with {len(tables['zh'])} keys")


if __name__ == "__main__":
    main()
