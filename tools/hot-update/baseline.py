#!/usr/bin/env python3
"""迁移登记基线回填工具（admin 自助热更新 P1，我方新增）。

设计报告 §4.1：生产此前靠人工顺序执行迁移、无可信登记。P1 上线前必须把
「已被实证执行过」的迁移以 applied_by='baseline' 补登记进
`sys_schema_migration`（迁移 027 建表），此后热更新才分得清待执行/已执行。

本工具**只生成脚本、绝不连库**：
- 输入：`baseline_applied.txt`（已实证文件清单，一行一名）+ 仓库迁移文件；
- 输出到 --out-dir：
  - `baseline_probes.sql`：只读探针（每文件抽取 CREATE TABLE / ADD COLUMN
    目标，查 information_schema 是否存在），部署时先在生产库跑它人工核对；
  - `baseline_apply.sql`：逐文件一个 DO 块——探针全过才 INSERT 登记
    （ON CONFLICT DO NOTHING 幂等），探针不过 RAISE WARNING 跳过该文件，
    psql 输出即核验记录。执行前置：迁移 027 已执行（脚本内有守卫）。

文件定位顺序：rust/migrations/ 优先，其次 rust/deploy/pg/（同名以先到为准，
与清单注释口径一致：登记的是实际执行过的那份，sha 按定位文件计算）。
"""
import argparse
import hashlib
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SEARCH_DIRS = ["rust/migrations", "rust/deploy/pg"]


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def locate(repo_root: Path, name: str) -> Path | None:
    for d in SEARCH_DIRS:
        p = repo_root / d / name
        if p.exists():
            return p
    return None


def extract_probes(sql: str) -> list[tuple[str, str]]:
    """抽取探针目标：(kind, target)；kind = table | column（target=table.column）。"""
    probes: list[tuple[str, str]] = []
    clean = re.sub(r"--[^\n]*", " ", sql)
    for m in re.finditer(r"CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?([A-Za-z_][\w$]*)", clean, re.I):
        probes.append(("table", m.group(1)))
    for m in re.finditer(
        r"ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?([A-Za-z_][\w$]*)\s+ADD\s+COLUMN\s+(?:IF\s+NOT\s+EXISTS\s+)?([A-Za-z_][\w$]*)",
        clean,
        re.I,
    ):
        probes.append(("column", f"{m.group(1)}.{m.group(2)}"))
    # 去重保序
    seen: set[tuple[str, str]] = set()
    out = []
    for p in probes:
        if p not in seen:
            seen.add(p)
            out.append(p)
    return out


def probe_sql_expr(kind: str, target: str) -> str:
    if kind == "table":
        return (
            "EXISTS (SELECT 1 FROM information_schema.tables "
            f"WHERE table_schema = 'public' AND table_name = '{target}')"
        )
    table, column = target.split(".", 1)
    return (
        "EXISTS (SELECT 1 FROM information_schema.columns "
        f"WHERE table_schema = 'public' AND table_name = '{table}' AND column_name = '{column}')"
    )


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo-root", default=str(REPO_ROOT))
    ap.add_argument("--list", default=str(Path(__file__).with_name("baseline_applied.txt")))
    ap.add_argument("--out-dir", required=True)
    ap.add_argument("--version", default="0.0.2", help="登记的 product_version（基线对应产品版本）")
    args = ap.parse_args(argv[1:])

    repo_root = Path(args.repo_root)
    names = []
    for line in Path(args.list).read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            names.append(line)
    if not names:
        print("清单为空，拒绝生成（基线不能为空）", file=sys.stderr)
        return 1

    entries = []
    missing = []
    for name in names:
        path = locate(repo_root, name)
        if path is None:
            missing.append(name)
            continue
        data = path.read_bytes()
        entries.append({
            "file": name,
            "sha256": sha256_hex(data),
            "probes": extract_probes(data.decode("utf-8", "replace")),
        })
    if missing:
        print(f"以下文件在 {SEARCH_DIRS} 均未找到，已中止：{missing}", file=sys.stderr)
        return 1

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    # —— baseline_probes.sql（只读） ——
    probe_lines = [
        "-- 基线探针（只读）：逐文件核验其 CREATE TABLE / ADD COLUMN 目标是否已在库中。",
        "-- 全部 ok=true 才可执行 baseline_apply.sql；有 false 的文件先查清再改清单。",
    ]
    for e in entries:
        if not e["probes"]:
            probe_lines.append(
                f"SELECT '{e['file']}' AS file, '(无可抽取探针，须人工核验)' AS probe, NULL AS ok;"
            )
        for kind, target in e["probes"]:
            probe_lines.append(
                f"SELECT '{e['file']}' AS file, '{kind}:{target}' AS probe, "
                f"({probe_sql_expr(kind, target)}) AS ok;"
            )
    (out_dir / "baseline_probes.sql").write_text("\n".join(probe_lines) + "\n", encoding="utf-8")

    # —— baseline_apply.sql（探针守卫 + 幂等登记） ——
    apply_lines = [
        "-- 基线回填：探针全过的文件才登记（applied_by='baseline'），不过的跳过并 WARNING。",
        "-- 前置：迁移 027 已执行（sys_schema_migration 存在）。幂等可重复执行。",
        "DO $$ BEGIN",
        "  IF to_regclass('public.sys_schema_migration') IS NULL THEN",
        "    RAISE EXCEPTION 'sys_schema_migration 不存在：请先执行迁移 027';",
        "  END IF;",
        "END $$;",
    ]
    for e in entries:
        conds = [probe_sql_expr(k, t) for k, t in e["probes"]]
        cond = " AND ".join(conds) if conds else "FALSE /* 无探针：默认不登记，须人工确认后改脚本 */"
        apply_lines += [
            "DO $$ BEGIN",
            f"  IF {cond} THEN",
            "    INSERT INTO sys_schema_migration (filename, sha256, product_version, applied_at, applied_by, job_id)",
            f"    VALUES ('{e['file']}', '{e['sha256']}', '{args.version}', EXTRACT(EPOCH FROM now())::bigint, 'baseline', '')",
            "    ON CONFLICT (filename) DO NOTHING;",
            f"    RAISE NOTICE 'baseline 已登记（或已存在）: {e['file']}';",
            "  ELSE",
            f"    RAISE WARNING '探针未通过，跳过登记: {e['file']}';",
            "  END IF;",
            "END $$;",
        ]
    (out_dir / "baseline_apply.sql").write_text("\n".join(apply_lines) + "\n", encoding="utf-8")

    no_probe = [e["file"] for e in entries if not e["probes"]]
    print(f"已生成 baseline_probes.sql / baseline_apply.sql：{len(entries)} 个文件")
    if no_probe:
        print(f"注意：以下文件无可抽取探针（apply 脚本默认不登记，须人工核验）：{no_probe}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
