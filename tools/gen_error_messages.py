#!/usr/bin/env python3
"""Regenerate the Rust user-facing message table from the frontend i18n files.

Run from anywhere:
    python tools/gen_error_messages.py          # regenerate
    python tools/gen_error_messages.py --check  # verify it is up to date

The zh/en `errors` sections in src/i18n/locales/*.json are the single source
of truth for backend error messages. This keeps the frontend and the Rust
backend in sync (AGENTS.md: backend produces friendly messages, frontend
displays them verbatim).

The script also fails when the Rust code references a key that no longer exists
in those JSON files — otherwise the backend would silently fall back to a
generic message at runtime.
"""

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "src-tauri/src/errors/messages.rs"
RS_SOURCE = ROOT / "src-tauri/src"

KEY_PREFIX = "errors."

# `errors::message("errors.x")` and `AppError::Foo("errors.x")` reference a key
# literally; `AppError::AiError("code")` is rendered as `errors.ai.code`.
LITERAL_KEY_PATTERNS = (
    re.compile(r'errors::message\("([^"]+)"\)'),
    re.compile(r'AppError::\w+\("(errors\.[\w.]+)"'),
)
AI_CODE_PATTERN = re.compile(r'AppError::AiError\("([^"]+)"')


def flatten(obj, prefix=""):
    out = {}
    for key, value in obj.items():
        path = f"{prefix}.{key}" if prefix else key
        if isinstance(value, dict):
            out.update(flatten(value, path))
        else:
            out[path] = value
    return out


def load_tables():
    tables = {}
    for lang in ("zh", "en"):
        path = ROOT / f"src/i18n/locales/{lang}.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        tables[lang] = flatten(data["errors"])

    if set(tables["zh"]) != set(tables["en"]):
        missing = set(tables["zh"]) ^ set(tables["en"])
        raise SystemExit(f"zh/en error keys differ: {sorted(missing)}")
    return tables


def referenced_keys():
    """Every error key the Rust backend can ask for."""
    keys = set()
    for path in sorted(RS_SOURCE.rglob("*.rs")):
        text = path.read_text(encoding="utf-8")
        for pattern in LITERAL_KEY_PATTERNS:
            keys.update(m.group(1) for m in pattern.finditer(text))
        for match in AI_CODE_PATTERN.finditer(text):
            keys.add(f"{KEY_PREFIX}ai.{match.group(1)}")
    return keys


def validate_usage(tables):
    known = set(tables["zh"])
    missing = sorted(
        key for key in referenced_keys() if key[len(KEY_PREFIX) :] not in known
    )
    if missing:
        raise SystemExit(
            "Rust references error keys that are not in src/i18n/locales/*.json:\n  "
            + "\n  ".join(missing)
        )


def build_source(tables):
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
    return "\n".join(lines)


def rustfmt_source(source):
    """Format like `cargo fmt --check` expects (long English messages wrap)."""
    if not shutil.which("rustfmt"):
        print("rustfmt not found; run `cargo fmt` after regenerating", file=sys.stderr)
        return source
    with tempfile.NamedTemporaryFile(
        "w", suffix=".rs", encoding="utf-8", delete=False, newline="\n"
    ) as handle:
        handle.write(source)
        tmp_path = Path(handle.name)
    try:
        subprocess.run(
            ["rustfmt", "--edition", "2021", str(tmp_path)],
            check=False,
            capture_output=True,
        )
        return tmp_path.read_text(encoding="utf-8")
    finally:
        tmp_path.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail instead of writing when generated output is stale",
    )
    args = parser.parse_args()

    tables = load_tables()
    validate_usage(tables)
    formatted = rustfmt_source(build_source(tables)).replace("\r\n", "\n")

    if args.check:
        current = (
            OUTPUT.read_text(encoding="utf-8").replace("\r\n", "\n")
            if OUTPUT.exists()
            else ""
        )
        if current != formatted:
            raise SystemExit(
                f"{OUTPUT.relative_to(ROOT)} is out of date; "
                "run `python tools/gen_error_messages.py`"
            )
        print(f"error messages up to date ({len(tables['zh'])} keys)")
        return

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(formatted, encoding="utf-8", newline="\n")
    print(f"wrote {OUTPUT.relative_to(ROOT)} with {len(tables['zh'])} keys")


if __name__ == "__main__":
    main()
