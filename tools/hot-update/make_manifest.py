#!/usr/bin/env python3
"""生成 release-manifest.json（admin 自助热更新，我方新增）。

在发版收尾时运行：汇聚四面产物（沿用各构建线既有包，不改变其产物口径），
产出统一产品 Release 的清单资产 `release-manifest.json`，结构与后端
`nextrade_core::models::system::system_update::ReleaseManifest` 逐字段对应。

契约要点：
- tag 形如 `v<产品版本>`，资产 = 四面产物 + 本清单；
- `migrations` = 仓库 `rust/migrations/` 全部**编号**迁移文件的有序全序
  （待执行 = 清单 − 生产登记表，故必须给全序而非仅本版增量；日期命名的
  历史文件不入序列，口径见设计报告 §4.1「新迁移只进编号序列」）；
- 每条迁移带 sha256、首行注释摘要、语句级分类（classify_sql.py 解析标注）、
  破坏性标志与 SQL 预览（>8000 字符截断并置 sqlTruncated）；
- 产品版本与后端内嵌戳 `common::version::PRODUCT_VERSION` 交叉校验，
  不一致默认拒绝（发版时两处同改）。

用法示例：
  python3 make_manifest.py --version 0.0.3 --commit <sha> \
      --admin /tmp/nextrade-admin.tgz --pc /tmp/nextrade-pc.tgz \
      --h5 /tmp/nextrade-h5.tgz --backend /tmp/nextrade \
      --build 123 --out /tmp/release-manifest.json
然后把 --out 与四面产物一并上传到 tag v0.0.3 的 Release。
"""
import argparse
import hashlib
import io
import json
import re
import sys
import tarfile
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from classify_sql import classify_sql  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]
SQL_PREVIEW_LIMIT = 8000


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def extract_entry(html: str, prefix: str, suffix: str) -> str | None:
    i = html.find(prefix)
    if i == -1:
        return None
    tail = html[i:]
    j = tail.find(suffix)
    if j == -1:
        return None
    cand = tail[: j + len(suffix)]
    if all(c.isalnum() or c in "-_." for c in cand):
        return cand
    return None


def stamp_from_tgz(path: Path, surface: str) -> str:
    """从面包内读版本戳：version.json 优先，其次面专属指纹。读不到返回空串。"""
    try:
        with tarfile.open(path, "r:gz") as tf:
            names = {n.lstrip("./"): n for n in tf.getnames()}

            def read(name: str) -> str | None:
                key = names.get(name)
                if key is None:
                    return None
                f = tf.extractfile(key)
                return f.read().decode("utf-8", "replace") if f else None

            vj = read("version.json")
            if vj:
                try:
                    v = json.loads(vj)
                    if v.get("stamp"):
                        return str(v["stamp"])
                    if v.get("gitSha"):
                        return str(v["gitSha"])
                except json.JSONDecodeError:
                    pass
            if surface == "pc":
                info = read("DEPLOY_INFO.txt")
                if info:
                    for line in info.splitlines():
                        if line.startswith("buildId "):
                            return line.split(None, 1)[1].strip()
                bid = read(".next/BUILD_ID")
                if bid:
                    return bid.strip()
            if surface in ("admin", "h5"):
                html = read("index.html")
                if html:
                    prefix = "index-" if surface == "admin" else "app."
                    entry = extract_entry(html, prefix, ".js")
                    if entry:
                        return entry
    except (tarfile.TarError, OSError):
        return ""
    return ""


def surface_asset(path: str | None, surface: str, build: str, commit: str) -> dict | None:
    if not path:
        return None
    p = Path(path)
    data = p.read_bytes()
    if surface == "backend":
        stamp = commit
    else:
        stamp = stamp_from_tgz(p, surface)
        if not stamp:
            print(f"警告：{surface} 包内读不到版本戳，stamp 置空（页面将无法判定该面新旧）", file=sys.stderr)
    return {
        "file": p.name,
        "sha256": sha256_hex(data),
        "size": len(data),
        "build": build,
        "stamp": stamp,
    }


def migration_entries(repo_root: Path) -> list[dict]:
    mig_dir = repo_root / "rust" / "migrations"
    files = sorted(f for f in mig_dir.glob("[0-9]*.sql"))
    out = []
    for f in files:
        text = f.read_text(encoding="utf-8")
        cls = classify_sql(text)
        summary = ""
        for line in text.splitlines():
            line = line.strip()
            if line.startswith("--"):
                candidate = line.lstrip("-").strip()
                if candidate:
                    summary = candidate[:120]
                    break
        out.append({
            "file": f.name,
            "sha256": sha256_hex(text.encode("utf-8")),
            "summary": summary,
            "changes": cls["changes"],
            "destructive": cls["destructive"],
            "requiresBackendRestart": True,
            "sqlPreview": text[:SQL_PREVIEW_LIMIT],
            "sqlTruncated": len(text) > SQL_PREVIEW_LIMIT,
        })
    return out


def changelog_notes(repo_root: Path, version: str) -> str:
    changelog = repo_root / "CHANGELOG.md"
    if not changelog.exists():
        return ""
    text = changelog.read_text(encoding="utf-8")
    sections: list[str] = []
    current: list[str] = []
    capturing = False
    for line in text.splitlines():
        if line.startswith("## "):
            if capturing and current:
                sections.append("\n".join(current).strip())
            capturing = line[3:].strip().startswith(version)
            current = [line] if capturing else []
        elif capturing:
            current.append(line)
    if capturing and current:
        sections.append("\n".join(current).strip())
    return "\n\n".join(sections)


def product_version_const(repo_root: Path) -> str | None:
    vf = repo_root / "rust" / "crates" / "common" / "src" / "version.rs"
    if not vf.exists():
        return None
    m = re.search(r'PRODUCT_VERSION:\s*&str\s*=\s*"([^"]+)"', vf.read_text(encoding="utf-8"))
    return m.group(1) if m else None


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--version", required=True, help="产品版本 x.y.z")
    ap.add_argument("--commit", required=True, help="源码 commit（完整 sha）")
    ap.add_argument("--build", default="", help="构建号（写入各面 build 字段）")
    ap.add_argument("--backend")
    ap.add_argument("--admin")
    ap.add_argument("--pc")
    ap.add_argument("--h5")
    ap.add_argument("--min-schema", default=None, help="静态面更新前必须已执行的迁移文件名")
    ap.add_argument("--requires-miniprogram", action="store_true")
    ap.add_argument("--requires-app", action="store_true")
    ap.add_argument("--allow-version-mismatch", action="store_true",
                    help="跳过与后端内嵌 PRODUCT_VERSION 的交叉校验")
    ap.add_argument("--repo-root", default=str(REPO_ROOT))
    ap.add_argument("--out", required=True)
    args = ap.parse_args(argv[1:])

    repo_root = Path(args.repo_root)
    const_version = product_version_const(repo_root)
    if const_version != args.version and not args.allow_version_mismatch:
        print(
            f"拒绝：--version {args.version} 与后端内嵌 PRODUCT_VERSION {const_version} 不一致；"
            "发版时须同改（或确属特殊情况时加 --allow-version-mismatch）",
            file=sys.stderr,
        )
        return 1

    manifest = {
        "productVersion": args.version,
        "gitCommit": args.commit,
        "releasedAt": datetime.now(timezone.utc).isoformat(),
        "notes": changelog_notes(repo_root, args.version),
        "surfaces": {
            "backend": surface_asset(args.backend, "backend", args.build, args.commit),
            "admin": surface_asset(args.admin, "admin", args.build, args.commit),
            "pc": surface_asset(args.pc, "pc", args.build, args.commit),
            "h5": surface_asset(args.h5, "h5", args.build, args.commit),
        },
        "migrations": migration_entries(repo_root),
        "compat": {
            "minSchema": args.min_schema,
            "requiresMiniProgramUpdate": args.requires_miniprogram,
            "requiresAppUpdate": args.requires_app,
        },
    }
    out = Path(args.out)
    out.write_text(json.dumps(manifest, ensure_ascii=False, indent=1), encoding="utf-8")
    n_mig = len(manifest["migrations"])
    destr = sum(1 for m in manifest["migrations"] if m["destructive"])
    print(f"已生成 {out}：版本 {args.version}，迁移 {n_mig} 条（破坏性 {destr} 条）")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
