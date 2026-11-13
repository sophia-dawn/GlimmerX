#!/usr/bin/env python3
"""Guard against documentation drift for facts that are cheap to verify.

Checks:
1. Every package named in the 01-overview stack table exists in package.json.
2. The accounts column count in 08-constraints matches schema_v1.sql, and the
   section does not describe columns that do not exist.

Run from the repository root: `python tools/check_docs.py`
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACKAGE_JSON = ROOT / "package.json"
OVERVIEW = ROOT / "docs" / "design" / "01-overview.md"
CONSTRAINTS = ROOT / "docs" / "design" / "08-constraints.md"
SCHEMA = ROOT / "src-tauri" / "src" / "db" / "schema_v1.sql"

errors: list[str] = []

# Names that appear in the stack table but are not npm packages.
NON_NPM_NAMES = {"shadcn/ui"}


def check_stack_packages() -> None:
    manifest = json.loads(PACKAGE_JSON.read_text(encoding="utf-8"))
    known = set(manifest.get("dependencies", {})) | set(
        manifest.get("devDependencies", {})
    )
    text = OVERVIEW.read_text(encoding="utf-8")
    in_table = False
    for line in text.splitlines():
        if line.startswith("## "):
            in_table = line.strip() == "## 技术栈"
            continue
        if not in_table or not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) < 2:
            continue
        bold = re.findall(r"\*\*(.+?)\*\*", cells[1])
        for value in bold:
            if "自研" in value or value.startswith("`"):
                continue
            name = value.strip()
            if name in NON_NPM_NAMES:
                continue
            if not re.fullmatch(r"@?[a-z0-9][a-z0-9._/-]*", name):
                continue
            if name not in known:
                errors.append(
                    f"01-overview.md 技术栈引用了 package.json 中不存在的依赖: {name}"
                )


def schema_account_columns() -> list[str]:
    sql = SCHEMA.read_text(encoding="utf-8")
    match = re.search(
        r"CREATE TABLE IF NOT EXISTS accounts\s*\((.*?)\n\);", sql, re.DOTALL
    )
    if not match:
        raise SystemExit("schema_v1.sql 中找不到 accounts 建表语句")
    columns: list[str] = []
    for raw in match.group(1).splitlines():
        line = raw.strip().rstrip(",")
        if not line or line.startswith("--"):
            continue
        word = line.split()[0]
        if word.upper() in {"PRIMARY", "UNIQUE", "FOREIGN", "CHECK", "CONSTRAINT"}:
            continue
        columns.append(word)
    return columns


def check_accounts_section() -> None:
    columns = schema_account_columns()
    text = CONSTRAINTS.read_text(encoding="utf-8")
    section = text.split("## accounts 表设计", 1)
    if len(section) != 2:
        errors.append("08-constraints.md 缺少「## accounts 表设计」小节")
        return
    body = section[1].split("\n## ", 1)[0]

    claimed = re.search(r"当前\s*(\d+)\s*列", body)
    if not claimed:
        errors.append("08-constraints.md 未声明 accounts 列数（应写「当前 N 列」）")
    elif int(claimed.group(1)) != len(columns):
        errors.append(
            f"08-constraints.md 写 accounts 为 {claimed.group(1)} 列，"
            f"schema_v1.sql 实际为 {len(columns)} 列"
        )

    for field in ("subtype", "parent_id"):
        for line in body.splitlines():
            if not re.search(rf"`{field}`", line):
                continue
            # A sentence that explicitly says the column does not exist is fine.
            if "没有" in line or "不存在" in line:
                continue
            errors.append(
                f"08-constraints.md 描述了 accounts.{field}，但 schema_v1.sql 中不存在该列"
            )


def main() -> int:
    check_stack_packages()
    check_accounts_section()
    if errors:
        print("文档校验失败：")
        for err in errors:
            print(f"  - {err}")
        return 1
    print("文档校验通过：技术栈依赖与 accounts 表结构均与代码一致")
    return 0


if __name__ == "__main__":
    sys.exit(main())
