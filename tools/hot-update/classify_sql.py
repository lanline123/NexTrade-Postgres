#!/usr/bin/env python3
"""迁移 SQL 语句级分类器（admin 自助热更新，我方新增）。

把一个迁移文件的顶层语句解析成人类可读的变更标注（如「加字段
product.weight」「新增表 foo」「删字段 bar.baz」），并判定是否含破坏性
语句（DROP / TRUNCATE / 无 WHERE 的 DELETE/UPDATE）。

口径声明（与设计报告 §4.2 一致）：这是对 SQL **文本**的解析标注，不是与
活库 information_schema 的结构比对；admin 页面展示时必须如实标注
「按待执行迁移解析」。

切分器感知：单引号字符串（'' 转义）、双引号标识符、-- 行注释、
/* */ 块注释、$tag$ 美元引用字符串——其中的分号不切分。

用法：
  classify_sql.py <file.sql>          # 输出 JSON {changes, destructive, statements}
  classify_sql.py --selftest          # 跑内置断言
"""
import json
import re
import sys


def split_statements(sql: str) -> list[str]:
    """按顶层分号切分语句（感知字符串/注释/美元引用）。"""
    out: list[str] = []
    buf: list[str] = []
    i, n = 0, len(sql)
    while i < n:
        ch = sql[i]
        two = sql[i : i + 2]
        if two == "--":
            j = sql.find("\n", i)
            j = n if j == -1 else j
            buf.append(sql[i:j])
            i = j
            continue
        if two == "/*":
            j = sql.find("*/", i + 2)
            j = n - 2 if j == -1 else j
            buf.append(sql[i : j + 2])
            i = j + 2
            continue
        if ch == "'":
            j = i + 1
            while j < n:
                if sql[j] == "'":
                    if j + 1 < n and sql[j + 1] == "'":
                        j += 2
                        continue
                    break
                j += 1
            buf.append(sql[i : j + 1])
            i = j + 1
            continue
        if ch == '"':
            j = sql.find('"', i + 1)
            j = n - 1 if j == -1 else j
            buf.append(sql[i : j + 1])
            i = j + 1
            continue
        if ch == "$":
            m = re.match(r"\$[A-Za-z_][A-Za-z0-9_]*\$|\$\$", sql[i:])
            if m:
                tag = m.group(0)
                j = sql.find(tag, i + len(tag))
                if j != -1:
                    buf.append(sql[i : j + len(tag)])
                    i = j + len(tag)
                    continue
            buf.append(ch)
            i += 1
            continue
        if ch == ";":
            stmt = "".join(buf).strip()
            if stmt:
                out.append(stmt)
            buf = []
            i += 1
            continue
        buf.append(ch)
        i += 1
    tail = "".join(buf).strip()
    if tail:
        out.append(tail)
    return out


def strip_comments(stmt: str) -> str:
    stmt = re.sub(r"/\*.*?\*/", " ", stmt, flags=re.S)
    stmt = re.sub(r"--[^\n]*", " ", stmt)
    return re.sub(r"\s+", " ", stmt).strip()


IDENT = r'(?:"[^"]+"|[A-Za-z_][A-Za-z0-9_$]*)(?:\s*\.\s*(?:"[^"]+"|[A-Za-z_][A-Za-z0-9_$]*))?'


def last_part(ident: str) -> str:
    part = ident.split(".")[-1].strip().strip('"')
    return part


def classify_statement(stmt: str) -> tuple[list[str], bool]:
    """返回 (标注列表, 是否破坏性)。"""
    s = strip_comments(stmt)
    up = s.upper()
    if not up or up in ("BEGIN", "COMMIT", "ROLLBACK"):
        return [], False

    m = re.match(rf"CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?({IDENT})", s, re.I)
    if m:
        return [f"新增表 {last_part(m.group(1))}"], False
    m = re.match(rf"DROP\s+TABLE\s+(?:IF\s+EXISTS\s+)?({IDENT})", s, re.I)
    if m:
        return [f"删除表 {last_part(m.group(1))}"], True
    m = re.match(rf"ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?({IDENT})\s+(.*)$", s, re.I | re.S)
    if m:
        table = last_part(m.group(1))
        rest = m.group(2)
        changes: list[str] = []
        destructive = False
        for sub in rest.split(","):
            sub = sub.strip()
            a = re.match(r'ADD\s+COLUMN\s+(?:IF\s+NOT\s+EXISTS\s+)?("[^"]+"|[A-Za-z_][\w$]*)', sub, re.I)
            if a:
                changes.append(f"加字段 {table}.{a.group(1).strip(chr(34))}")
                continue
            d = re.match(r'DROP\s+COLUMN\s+(?:IF\s+EXISTS\s+)?("[^"]+"|[A-Za-z_][\w$]*)', sub, re.I)
            if d:
                changes.append(f"删字段 {table}.{d.group(1).strip(chr(34))}")
                destructive = True
                continue
            al = re.match(r'ALTER\s+COLUMN\s+("[^"]+"|[A-Za-z_][\w$]*)', sub, re.I)
            if al:
                changes.append(f"改字段 {table}.{al.group(1).strip(chr(34))}")
                continue
            rn = re.match(r'RENAME\s+COLUMN\s+("[^"]+"|[A-Za-z_][\w$]*)\s+TO\s+("[^"]+"|[A-Za-z_][\w$]*)', sub, re.I)
            if rn:
                changes.append(f"改字段 {table}.{rn.group(1).strip(chr(34))}")
                continue
            if re.match(r"ADD\s+(?:CONSTRAINT|PRIMARY|UNIQUE|CHECK|FOREIGN)", sub, re.I):
                changes.append(f"加约束 {table}")
                continue
            if re.match(r"DROP\s+CONSTRAINT", sub, re.I):
                changes.append(f"删约束 {table}")
                destructive = True
                continue
            changes.append(f"改表 {table}")
        return changes, destructive
    m = re.match(rf"CREATE\s+(UNIQUE\s+)?INDEX\s+(?:IF\s+NOT\s+EXISTS\s+)?({IDENT})", s, re.I)
    if m:
        return [f"加索引 {last_part(m.group(2))}"], False
    m = re.match(rf"DROP\s+INDEX\s+(?:IF\s+EXISTS\s+)?({IDENT})", s, re.I)
    if m:
        return [f"删索引 {last_part(m.group(1))}"], True
    if re.match(r"TRUNCATE\b", up):
        return ["清空表数据（TRUNCATE）"], True
    m = re.match(rf"DELETE\s+FROM\s+({IDENT})", s, re.I)
    if m:
        table = last_part(m.group(1))
        if re.search(r"\bWHERE\b", up):
            return [f"删除数据 {table}"], False
        return [f"删除数据 {table}（无 WHERE）"], True
    m = re.match(rf"UPDATE\s+({IDENT})", s, re.I)
    if m:
        table = last_part(m.group(1))
        if re.search(r"\bWHERE\b", up):
            return [f"数据订正 {table}"], False
        return [f"数据订正 {table}（无 WHERE）"], True
    m = re.match(rf"INSERT\s+INTO\s+({IDENT})", s, re.I)
    if m:
        return [f"写入数据 {last_part(m.group(1))}"], False
    m = re.match(r"CREATE\s+(OR\s+REPLACE\s+)?(FUNCTION|VIEW|TYPE|EXTENSION|SEQUENCE|TRIGGER)\b", up)
    if m:
        kind = {"FUNCTION": "函数", "VIEW": "视图", "TYPE": "类型",
                "EXTENSION": "扩展", "SEQUENCE": "序列", "TRIGGER": "触发器"}[m.group(2)]
        return [f"新增/更新{kind}"], False
    if re.match(r"DROP\s+(FUNCTION|VIEW|TYPE|SEQUENCE|TRIGGER)\b", up):
        return ["删除数据库对象"], True
    head = " ".join(s.split()[:3])
    return [f"其他语句：{head}"], False


def classify_sql(sql: str) -> dict:
    changes: list[str] = []
    destructive = False
    statements = split_statements(sql)
    for stmt in statements:
        c, d = classify_statement(stmt)
        destructive = destructive or d
        for item in c:
            if item not in changes:
                changes.append(item)
    return {"changes": changes, "destructive": destructive, "statements": len(statements)}


def selftest() -> int:
    cases = [
        ("ALTER TABLE product ADD COLUMN IF NOT EXISTS weight int;", ["加字段 product.weight"], False),
        ("ALTER TABLE product DROP COLUMN weight;", ["删字段 product.weight"], True),
        ("CREATE TABLE IF NOT EXISTS foo (id int);", ["新增表 foo"], False),
        ("UPDATE config SET x = 1;", ["数据订正 config（无 WHERE）"], True),
        ("UPDATE config SET x = 1 WHERE id = 2;", ["数据订正 config"], False),
        ("INSERT INTO t (a) VALUES ('a;b');", ["写入数据 t"], False),
        ("CREATE UNIQUE INDEX IF NOT EXISTS idx_a ON t (a);", ["加索引 idx_a"], False),
        ("ALTER TABLE t ADD COLUMN a int, ADD COLUMN b text;", ["加字段 t.a", "加字段 t.b"], False),
        ("DO $$ BEGIN RAISE NOTICE 'x;y'; END $$;", None, False),  # 只验不误判破坏性
    ]
    for sql, expect, destr in cases:
        got = classify_sql(sql)
        assert got["destructive"] == destr, (sql, got)
        if expect is not None:
            assert got["changes"] == expect, (sql, got)
    print("selftest OK")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) >= 2 and argv[1] == "--selftest":
        return selftest()
    if len(argv) != 2:
        print(__doc__)
        return 2
    with open(argv[1], encoding="utf-8") as fh:
        result = classify_sql(fh.read())
    print(json.dumps(result, ensure_ascii=False, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
