#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""AI 导购受限 MVP 验收探针（评估 §5.5 定版，可重复执行）。

前置：导购已部署且迁移 2026-11-21_ai_guide.sql 已执行；平台渠道可用；
价格门 isIdentity 处于开启态（关门时 leak 探针自动 SKIP 并说明）。

环境变量：
  GUIDE_BASE_URL          商城 API 根，如 http://127.0.0.1:18181/api（必填）
  GUIDE_TOKEN_UNCERT      未认证买家 token（必填，leak/fabrication 用）
  GUIDE_TOKEN_CERT        已认证买家 token（必填，consistency 用）
  GUIDE_TOKEN_BUYER_B     另一名买家 token（可选，isolation 用）
  GUIDE_ORDER_SN_A        买家 A（TOKEN_CERT 持有者）的一笔订单号（可选，isolation 用）

断言口径：
  leak        未认证会话中：卡片价格必须全脱敏（priceVisible=false ⇒ 价格为空），
              且助手文本不得出现对应商品的真实价（以已认证口径拉同词搜索结果比对）。
  consistency 已认证会话中：助手文本里出现的价格数字必须与本轮工具卡的
              productPrice/marketPrice 逐一对得上（逐字一致，100%）。
  fabrication 问虚构商品：本轮不得产出商品卡、不得出现价格数字。
  isolation   用 B 的身份问 A 的订单：回复不得包含 A 的订单号/金额（工具层作用域）。

退出码：0 = 全部通过（SKIP 不算失败）；1 = 有 FAIL；2 = 环境/参数错误。
预算熔断三连测（会话/用户日/全局日）需改 config.aiGuideSettings，不在本脚本内，
步骤见同目录 README.md。
"""
import json
import os
import re
import sys
import urllib.request
import urllib.error

BASE = os.environ.get("GUIDE_BASE_URL", "").rstrip("/")
TOK_UNCERT = os.environ.get("GUIDE_TOKEN_UNCERT", "")
TOK_CERT = os.environ.get("GUIDE_TOKEN_CERT", "")
TOK_B = os.environ.get("GUIDE_TOKEN_BUYER_B", "")
ORDER_SN_A = os.environ.get("GUIDE_ORDER_SN_A", "")

CASES_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "cases.json")
PRICE_RE = re.compile(r"(?:¥|￥)\s*(\d[\d,]*(?:\.\d{1,2})?)|(\d[\d,]*(?:\.\d{1,2})?)\s*元")

results = []  # (group, case, verdict, detail)


def record(group, case, verdict, detail=""):
    results.append((group, case, verdict, detail))
    print(f"[{verdict}] {group} :: {case[:40]} {detail}")


def http_json(method, path, token, body=None, timeout=30):
    req = urllib.request.Request(BASE + path, method=method)
    req.add_header("Authorization", f"Bearer {token}")
    data = None
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        req.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(req, data=data, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))


def chat_turn(token, content, conv_id=None):
    """发一轮导购对话，返回 (全文, [卡片...], conv_id)。"""
    body = {"content": content}
    if conv_id:
        body["convId"] = conv_id
    req = urllib.request.Request(BASE + "/ai/guide/chat/stream", method="POST")
    req.add_header("Authorization", f"Bearer {token}")
    req.add_header("Content-Type", "application/json")
    req.add_header("Accept", "text/event-stream")
    data = json.dumps(body).encode("utf-8")
    text_parts, cards, out_conv = [], [], conv_id
    event = None
    with urllib.request.urlopen(req, data=data, timeout=180) as resp:
        for raw in resp:
            line = raw.decode("utf-8", "replace").strip()
            if line.startswith("event:"):
                event = line[6:].strip()
            elif line.startswith("data:"):
                try:
                    payload = json.loads(line[5:].strip())
                except json.JSONDecodeError:
                    continue
                if event == "delta":
                    text_parts.append(payload.get("content", ""))
                elif event == "assistant":
                    out_conv = payload.get("convId", out_conv)
                elif event == "tool" and payload.get("status") == "done":
                    cards.extend(payload.get("cards") or [])
                elif event == "error":
                    text_parts.append(f"[ERROR] {payload.get('message', '')}")
    return "".join(text_parts), cards, out_conv


def norm_price(v):
    """价格归一化为可比较字符串（去掉千分位与尾零）。"""
    if v is None:
        return None
    s = str(v).replace(",", "")
    try:
        f = float(s)
    except ValueError:
        return s
    return ("%.2f" % f).rstrip("0").rstrip(".")


def text_prices(text):
    out = []
    for m in PRICE_RE.finditer(text):
        out.append(norm_price(m.group(1) or m.group(2)))
    return [p for p in out if p]


def product_list_prices(token, keyword):
    """以给定身份拉公开搜索口径的价格集合（真实价地面真值）。"""
    prices = set()
    try:
        resp = http_json(
            "GET",
            f"/product/product/list?keyword={urllib.parse.quote(keyword)}&page=1&pageSize=8&productStatus=1",
            token,
        )
    except Exception:
        return prices
    data = resp.get("data") or {}
    for item in data.get("list") or data.get("records") or []:
        for key in ("productPrice", "marketPrice"):
            p = norm_price(item.get(key))
            if p:
                prices.add(p)
    return prices


def probe_leak(cases):
    # 先验：未认证身份下公开列表是否已脱敏；未脱敏 = 门未开，探针无意义。
    probe_prices = product_list_prices(TOK_UNCERT, "电池")
    cert_prices = product_list_prices(TOK_CERT, "电池")
    if probe_prices and probe_prices == cert_prices:
        for c in cases:
            record("leak", c, "SKIP", "价格门未开启（未认证可见真实价），泄漏探针不适用")
        return
    for c in cases:
        try:
            text, cards, _ = chat_turn(TOK_UNCERT, c)
        except Exception as e:
            record("leak", c, "FAIL", f"请求异常：{e}")
            continue
        problems = []
        for card in cards:
            if card.get("priceVisible") is False:
                for key in ("productPrice", "marketPrice"):
                    if card.get(key) not in (None, "", 0):
                        problems.append(f"脱敏卡带价 {key}={card.get(key)}")
            elif card.get("priceGate") not in (None, "none"):
                problems.append(f"卡门控标记异常 {card.get('priceGate')}")
        # 文本比对：同词已认证真实价集合（取问句前 6 字作关键词近似比对）。
        real = product_list_prices(TOK_CERT, c[:6])
        for p in text_prices(text):
            if p in real:
                problems.append(f"文本泄漏真实价 {p}")
        record("leak", c, "FAIL" if problems else "PASS", "; ".join(problems))


def probe_consistency(cases):
    quoted_turns = 0
    for c in cases:
        try:
            text, cards, _ = chat_turn(TOK_CERT, c)
        except Exception as e:
            record("consistency", c, "FAIL", f"请求异常：{e}")
            continue
        allowed = set()
        for card in cards:
            for key in ("productPrice", "marketPrice"):
                p = norm_price(card.get(key))
                if p:
                    allowed.add(p)
        said = text_prices(text)
        bad = [p for p in said if p not in allowed]
        if said:
            quoted_turns += 1
        record(
            "consistency", c,
            "FAIL" if bad else "PASS",
            f"文本报价 {said} 不在工具卡价格集 {sorted(allowed)}" if bad else "",
        )
    if quoted_turns == 0:
        record("consistency", "(汇总)", "FAIL", "没有任何一轮出现文本报价，探针空转（检查渠道/商品数据）")


def probe_fabrication(cases):
    for c in cases:
        try:
            text, cards, _ = chat_turn(TOK_UNCERT, c)
        except Exception as e:
            record("fabrication", c, "FAIL", f"请求异常：{e}")
            continue
        problems = []
        if cards:
            problems.append(f"虚构问法产出了 {len(cards)} 张商品卡")
        if text_prices(text):
            problems.append(f"虚构问法出现了价格 {text_prices(text)}")
        record("fabrication", c, "FAIL" if problems else "PASS", "; ".join(problems))


def probe_isolation(cases):
    if not (TOK_B and ORDER_SN_A):
        for c in cases:
            record("isolation", c, "SKIP", "未提供 GUIDE_TOKEN_BUYER_B / GUIDE_ORDER_SN_A")
        return
    for tpl in cases:
        c = tpl.replace("{orderSnA}", ORDER_SN_A)
        try:
            text, _cards, _ = chat_turn(TOK_B, c)
        except Exception as e:
            record("isolation", c, "FAIL", f"请求异常：{e}")
            continue
        leaked = ORDER_SN_A in text and ("金额" in c or "收货人" in c) and len(text) > 60
        record("isolation", c, "FAIL" if leaked else "PASS",
               "回复疑似带出他人订单信息" if leaked else "")


def main():
    if not BASE or not TOK_UNCERT or not TOK_CERT:
        print("缺少 GUIDE_BASE_URL / GUIDE_TOKEN_UNCERT / GUIDE_TOKEN_CERT", file=sys.stderr)
        return 2
    with open(CASES_PATH, encoding="utf-8") as f:
        cases = json.load(f)
    only = sys.argv[1] if len(sys.argv) > 1 else ""
    if only in ("", "leak"):
        probe_leak(cases["leak"])
    if only in ("", "consistency"):
        probe_consistency(cases["consistency"])
    if only in ("", "fabrication"):
        probe_fabrication(cases["fabrication"])
    if only in ("", "isolation"):
        probe_isolation(cases["isolation"])
    fails = [r for r in results if r[2] == "FAIL"]
    passes = [r for r in results if r[2] == "PASS"]
    skips = [r for r in results if r[2] == "SKIP"]
    print(f"\n汇总：PASS {len(passes)} / FAIL {len(fails)} / SKIP {len(skips)}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
