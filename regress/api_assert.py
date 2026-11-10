#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""关键接口业务字段回归断言（NexTrade-Postgres）。

为什么需要它
------------
迁移期的缺陷有两类，HTTP 状态码只能抓住第一类：

1. **硬失败**：SQL 语法/类型错 → 500，状态码可见。
2. **静默失败**：服务层用 `.unwrap_or_default()` 兜住错误 → 200 + 空数组/全 0。
   真实案例：`/adminapi/panel/panel/statisticalData` 的访问量曲线在
   `TO_CHAR(text, ...)` 报错时被吞掉，页面显示「访问量 0」，与「当天没人访问」
   完全无法区分（见 MIGRATION-AUDIT-2026-11-09.md §30.3）。

因此本脚本对每个用例做四层断言：

```text
L0 HTTP 状态码 200
L1 业务码 code == 0（200 里也可能是 code:500 的业务错误）
L2 结构断言：data 不是 null；分页 total/records 自洽；坐标轴长度对齐、日期升序
L3 业务取值断言：关键字段存在且类型正确、主键 > 0、金额可解析、
   「total > 0 却 records 为空」判失败（分页恒空）
L4 基线守卫：与 regress/baseline.json 对比，基线 > 0 而本次为 0 → 判失败
   （专门抓「静默全 0」这类回归）
```

另有**跨接口一致性**断言，不依赖数据量、不受造数影响：

```text
statisticalData(days) 的访问量合计 == accessStatistics 同区间的点击量合计
panel/index.awaitShip == 订单列表 orderStatus=1 的 total
panel/index.productOnSaleNum == 商品列表 productStatus=1 的 total
panel/index.productTotalNum == 商品列表 total
cart/getCount == cart/list 的 total.totalCount
```

用法
----
```bash
# 默认打生产域名
python regress/api_assert.py

# 打本机后端（会员 8181 / 管理 8080）
python regress/api_assert.py --member-base http://127.0.0.1:8181 \
                             --admin-base  http://127.0.0.1:8080

# 只看一侧、看细节、导 JSON
python regress/api_assert.py --only admin -v --json regress/last.json

# 数据整体变化后重建基线（例如换库/重置演示数据）
python regress/api_assert.py --update-baseline
```

退出码：`0` 全通过 / `1` 有断言失败 / `2` 登录或网络不可用。

只依赖 Python 标准库（与仓库里既有验证脚本一致），不引入 requests。
"""

from __future__ import annotations

import argparse
import datetime as _dt
import http.cookiejar
import json
import os
import re
import ssl
import sys
import time
import urllib.error
import urllib.request

DEFAULT_BASE = "https://battery.recy.cc"
BASELINE_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "baseline.json")

MEMBER_USER = ("123", "123")
ADMIN_USER = ("admin", "admin123")

DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


# --------------------------------------------------------------------------- #
# 断言原语
# --------------------------------------------------------------------------- #
class AssertFail(Exception):
    """单条断言失败。"""


class Skip(Exception):
    """前提用例未通过，跳过该项校验（不算失败）。"""


def need(cond, msg):
    if not cond:
        raise AssertFail(msg)


def _is_int(v):
    return isinstance(v, int) and not isinstance(v, bool)


def _num(v):
    """把数字或数字字符串转 float；失败返回 None（金额常以字符串下发）。"""
    if isinstance(v, bool) or v is None:
        return None
    if isinstance(v, (int, float)):
        return float(v)
    if isinstance(v, str):
        try:
            return float(v.strip())
        except ValueError:
            return None
    return None


KIND_HELP = {
    "int": "整数（非 bool）",
    "int>0": "正整数",
    "int>=0": "非负整数",
    "num": "数字或数字字符串",
    "num>0": "正数",
    "str": "字符串",
    "str!": "非空字符串",
    "list": "数组",
    "list!": "非空数组",
    "dict": "对象",
    "dict!": "非空对象",
    "bool01": "0/1 或 bool",
    "any": "任意非 None",
}


def kind_ok(value, kind):  # noqa: C901 - 逐类型直读比字典分派更好懂
    if kind == "any":
        return value is not None
    if kind == "int":
        return _is_int(value)
    if kind == "int>0":
        return _is_int(value) and value > 0
    if kind == "int>=0":
        return _is_int(value) and value >= 0
    if kind == "num":
        return _num(value) is not None
    if kind == "num>0":
        n = _num(value)
        return n is not None and n > 0
    if kind == "str":
        return isinstance(value, str)
    if kind == "str!":
        return isinstance(value, str) and value.strip() != ""
    if kind == "list":
        return isinstance(value, list)
    if kind == "list!":
        return isinstance(value, list) and len(value) > 0
    if kind == "dict":
        return isinstance(value, dict)
    if kind == "dict!":
        return isinstance(value, dict) and len(value) > 0
    if kind == "bool01":
        return isinstance(value, bool) or (_is_int(value) and value in (0, 1))
    raise AssertFail("未知断言类型 %r（可用：%s）" % (kind, "/".join(KIND_HELP)))


def check_fields(obj, spec, where=""):
    """按 {字段: kind} 校验对象；缺字段与类型不符都算失败。"""
    need(isinstance(obj, dict), "%s 期望对象，实际 %s" % (where or "data", type(obj).__name__))
    for key, kind in spec.items():
        path = "%s.%s" % (where, key) if where else key
        need(key in obj, "缺字段 %s" % path)
        need(kind_ok(obj[key], kind),
             "字段 %s 不满足 %s（实际 %r）" % (path, KIND_HELP[kind], obj[key]))


# --------------------------------------------------------------------------- #
# 结构断言工厂
# --------------------------------------------------------------------------- #
def paged(total_min=None, fields=None, sample=5, keys=("total", "records")):
    """分页信封断言：total 与 records 必须自洽，total>0 时 records 不许为空。"""
    total_key, rec_key = keys

    def _check(data):
        need(isinstance(data, dict), "期望分页对象，实际 %s" % type(data).__name__)
        need(total_key in data, "缺 %s" % total_key)
        need(_is_int(data[total_key]), "%s 应为整数（实际 %r）" % (total_key, data[total_key]))
        need(rec_key in data, "缺 %s" % rec_key)
        need(isinstance(data[rec_key], list), "%s 应为数组" % rec_key)
        total, recs = data[total_key], data[rec_key]
        # 这一条就是「分页恒空」的哨兵：total 说有数据，records 却给了空
        need(total <= 0 or len(recs) > 0,
             "total=%d 但 %s 为空 —— 查询/分页被吞" % (total, rec_key))
        need(total >= 0, "total 为负（%d）" % total)
        if total_min is not None:
            need(total >= total_min, "total=%d 低于期望下限 %d" % (total, total_min))
        # 分页回显字段（两套命名都出现过）
        for k in ("size", "pageSize", "current", "page", "pages", "totalPage"):
            if k in data and data[k] is not None:
                need(_is_int(data[k]), "%s 应为整数（实际 %r）" % (k, data[k]))
        need(len(recs) <= total or total == 0,
             "records(%d) 多于 total(%d)" % (len(recs), total))
        if fields and recs:
            for row in recs[:sample]:
                check_fields(row, fields, where=rec_key + "[0]")
        return total

    return _check


def listing(min_len=1, fields=None, sample=5):
    """裸数组断言（类目、币种字典、模板列表等）。"""

    def _check(data):
        need(isinstance(data, list), "期望数组，实际 %s" % type(data).__name__)
        need(len(data) >= min_len, "数组长度 %d < %d" % (len(data), min_len))
        if fields and data:
            for row in data[:sample]:
                check_fields(row, fields, where="data[0]")
        return len(data)

    return _check


def axis_of(axis_keys, min_points=1, labels="date"):
    """坐标轴断言：横轴与每条纵轴等长；日期标签格式与升序。"""

    def _check(data):
        need(isinstance(data, dict), "期望坐标轴对象，实际 %s" % type(data).__name__)
        need("horizontalAxis" in data, "缺 horizontalAxis")
        hx = data["horizontalAxis"]
        need(isinstance(hx, list) and len(hx) >= min_points,
             "horizontalAxis 长度 %s < %d" % (len(hx) if isinstance(hx, list) else hx, min_points))
        if labels == "date":
            for lab in hx:
                need(isinstance(lab, str) and DATE_RE.match(lab),
                     "横轴标签不是 YYYY-MM-DD：%r" % (lab,))
            need(hx == sorted(hx), "横轴日期未升序：%s … %s" % (hx[0], hx[-1]))
        for key in axis_keys:
            need(key in data, "缺纵轴 %s" % key)
            ys = data[key]
            need(isinstance(ys, list), "%s 应为数组" % key)
            need(len(ys) == len(hx),
                 "%s 长度 %d 与横轴 %d 不对齐" % (key, len(ys), len(hx)))
            for v in ys:
                need(_num(v) is not None, "%s 含非数字值 %r" % (key, v))
                need(_num(v) >= 0, "%s 含负值 %r" % (key, v))
        return data

    return _check


def obj(fields):
    def _check(data):
        check_fields(data, fields)
        return data

    return _check


# --------------------------------------------------------------------------- #
# HTTP 客户端
# --------------------------------------------------------------------------- #
class Http(object):
    def __init__(self, base, timeout=40):
        self.base = base.rstrip("/")
        self.timeout = timeout
        ctx = ssl.create_default_context()
        jar = http.cookiejar.CookieJar()
        self.opener = urllib.request.build_opener(
            urllib.request.HTTPCookieProcessor(jar),
            urllib.request.HTTPSHandler(context=ctx),
        )

    def call(self, path, body=None, headers=None, method=None, retries=1):
        url = self.base + path
        payload = json.dumps(body).encode("utf-8") if body is not None else None
        attempt = 0
        while True:
            req = urllib.request.Request(
                url, data=payload, method=method or ("POST" if payload else "GET"))
            req.add_header("Content-Type", "application/json")
            for k, v in (headers or {}).items():
                req.add_header(k, v)
            try:
                with self.opener.open(req, timeout=self.timeout) as r:
                    return r.status, r.read().decode("utf-8", "replace")
            except urllib.error.HTTPError as e:
                return e.code, e.read().decode("utf-8", "replace")
            except (urllib.error.URLError, OSError) as e:
                # 只对网络层抖动重试；断言失败绝不重试（会把问题掩盖成偶发）
                if attempt >= retries:
                    raise AssertFail("网络错误：%s" % e)
                attempt += 1
                time.sleep(1.5)


class Session(object):
    """登录态 + 解析后的最后一次响应缓存。"""

    def __init__(self, http, side):
        self.http = http
        self.side = side
        self.token = None

    def login(self):
        if self.side == "member":
            st, txt = self.http.call(
                "/api/user/login/signin",
                {"username": MEMBER_USER[0], "password": MEMBER_USER[1]})
            need(st == 200, "会员登录 HTTP %s：%s" % (st, txt[:160]))
            tok = (json.loads(txt).get("data") or {}).get("token")
            need(tok, "会员登录未返回 token：%s" % txt[:160])
            self.token = tok
            return
        st, txt = self.http.call("/adminapi/common/csrf/create")
        need(st == 200, "CSRF 获取失败 HTTP %s" % st)
        csrf = (json.loads(txt).get("data")) or ""
        st, txt = self.http.call(
            "/adminapi/login/signin",
            {"loginType": "password", "username": ADMIN_USER[0],
             "password": ADMIN_USER[1], "captchaUid": ""},
            {"X-CSRF-Token": csrf})
        need(st == 200, "管理端登录 HTTP %s：%s" % (st, txt[:160]))
        tok = (json.loads(txt).get("data") or {}).get("token")
        need(tok, "管理端登录未返回 token：%s" % txt[:160])
        self.token = tok

    def call(self, path, body=None, method=None):
        h = {"Authorization": "Bearer " + self.token} if self.token else {}
        return self.http.call(path, body, h, method=method)


# --------------------------------------------------------------------------- #
# 用例定义
# --------------------------------------------------------------------------- #
class Case(object):
    def __init__(self, cid, side, path, method="GET", body=None,
                 check=None, metrics=None):
        self.id = cid
        self.side = side
        self.path = path
        self.method = method
        self.body = body
        self.check = check
        self.metrics = metrics            # fn(data) -> dict[str, number]


def days_ago(n, fmt="%Y-%m-%d"):
    return (_dt.date.today() - _dt.timedelta(days=n)).strftime(fmt)


def _sum(seq):
    return int(sum(_num(v) or 0 for v in seq))


# —— 会员端 ——

MEMBER_CASES = [
    Case("member.home.index", "member", "/api/home/home/index",
         check=lambda d: need(isinstance(d.get("moduleList"), list) and d["moduleList"],
                              "moduleList 为空 —— 首页装修数据没取到"),
         metrics=lambda d: {"moduleList": len(d.get("moduleList") or [])}),
    Case("member.product.list", "member", "/api/product/product/list?page=1&size=3",
         check=paged(total_min=1, fields={
             "productId": "int>0", "productName": "str!",
             "productPrice": "num", "productStock": "int>=0", "categoryId": "int"}),
         metrics=lambda d: {"total": d["total"]}),
    Case("member.category.list", "member", "/api/category/category/list",
         check=listing(min_len=1, fields={
             "categoryId": "int>0", "categoryName": "str!", "children": "list"}),
         metrics=lambda d: {"count": len(d)}),
    Case("member.article.list", "member", "/api/article/article/list?page=1&size=3",
         check=paged(total_min=1, fields={"articleId": "int>0", "articleTitle": "str!"})),
    Case("member.shop.list", "member", "/api/shop/shop/list?page=1&size=3",
         check=paged(total_min=1, fields={"shopId": "int>0", "shopTitle": "str!"})),
    Case("member.config.base", "member", "/api/common/config/base",
         check=obj({"adminDomain": "str!", "categoryDecorateType": "int"}),
         metrics=lambda d: {"keys": len(d)}),
    Case("member.pc.getNav", "member", "/api/common/pc/getNav",
         check=lambda d: _nav_check(d),
         metrics=lambda d: {"navGroups": sum(1 for k, v in d.items()
                                             if k.endswith("Nav") and v)}),
    Case("member.pc.getCatFloor", "member", "/api/common/pc/getCatFloor",
         check=lambda d: (
             need(isinstance(d.get("catFloor"), list) and d["catFloor"],
                  "catFloor 为空 —— 分类楼层没取到"),
             check_fields(d["catFloor"][0], {"catFloorId": "int>0",
                                             "catFloorName": "str!", "catList": "list"},
                          where="catFloor[0]")),
         metrics=lambda d: {"floors": len(d.get("catFloor") or [])}),
    Case("member.region.province", "member", "/api/sys/region/getProvinceList",
         check=listing(min_len=30, fields={"regionId": "int>0", "regionName": "str!"}),
         metrics=lambda d: {"count": len(d)}),
    Case("member.i18n.locales", "member", "/api/common/i18n/getLocales",
         check=lambda d: _locales_check(d),
         metrics=lambda d: {"count": len(d)}),
    Case("member.wechatLive.list", "member", "/api/promotion/wechatLive/list?page=1&size=3",
         check=paged(total_min=0, fields={"wechatLiveId": "int>0",
                                          "wechatLiveTitle": "str!"})),
    Case("member.sign.index", "member", "/api/user/sign/index",
         check=lambda d: _sign_check(d),
         metrics=lambda d: {"days": len(d.get("days") or []),
                            "recommendGoods": len(d.get("recommendGoods") or [])}),
    Case("member.cart.getCount", "member", "/api/cart/cart/getCount",
         check=lambda d: need(_is_int(d) and d >= 0, "期望非负整数，实际 %r" % (d,)),
         metrics=lambda d: {"count": d}),
    Case("member.cart.list", "member", "/api/cart/cart/list",
         check=lambda d: (
             check_fields(d, {"total": "dict!", "cartList": "list"}),
             check_fields(d["total"], {"totalCount": "int>=0"}, where="total")),
         metrics=lambda d: {"totalCount": d["total"]["totalCount"]}),
    Case("member.product.history", "member", "/api/product/product/getHistoryProduct",
         check=paged(total_min=0, fields={"productId": "int>0", "productName": "str!"})),
    Case("member.order.list", "member", "/api/user/order/list?page=1&size=3",
         check=paged(total_min=1, fields={"orderId": "int>0", "orderSn": "str!",
                                          "orderStatus": "int"}),
         metrics=lambda d: {"total": d["total"]}),
    Case("member.user.detail", "member", "/api/user/user/detail",
         check=obj({"memberName": "str!", "balance": "num"})),
    Case("member.message.list", "member", "/api/user/message/list?page=1&size=3",
         check=paged(total_min=0, fields={"messageId": "int>0", "title": "str!"})),
]


def _nav_check(d):
    check_fields(d, {"mainNav": "list", "topBarNav": "list",
                     "bottomNav": "list", "sidebarNav": "list"})
    filled = [k for k in ("mainNav", "topBarNav", "bottomNav", "sidebarNav") if d[k]]
    need(filled, "四组导航全为空 —— 导航配置丢失")
    first = d[filled[0]][0]
    check_fields(first, {"id": "int>0", "title": "str!"}, where=filled[0] + "[0]")


def _locales_check(d):
    need(isinstance(d, list) and len(d) >= 2, "语言列表不足 2 项")
    for row in d:
        check_fields(row, {"localeCode": "str!", "isDefault": "bool01",
                           "currency": "dict"}, where="locales[0]")
    need(sum(1 for r in d if r.get("isDefault") in (1, True)) == 1,
         "默认语言应恰好 1 个，实际 %d" % sum(1 for r in d if r.get("isDefault") in (1, True)))


def _sign_check(d):
    need(isinstance(d.get("days"), list) and len(d["days"]) == 7,
         "签到规则应为 7 天，实际 %s 天" % (len(d.get("days") or [])))
    for row in d["days"]:
        check_fields(row, {"id": "int>0", "name": "str!", "points": "int>=0"},
                     where="days[0]")
    check_fields(d, {"isSign": "bool01", "recommendGoods": "list"})


# —— 管理端 ——

ADMIN_CASES = [
    Case("admin.panel.index", "admin", "/adminapi/panel/panel/index",
         check=lambda d: _panel_index_check(d),
         metrics=lambda d: {
             "productTotalNum": d["consoleData"]["productTotalNum"],
             "productOnSaleNum": d["consoleData"]["productOnSaleNum"],
             "awaitShip": d["consoleData"]["awaitShip"]}),
    Case("admin.panel.statisticalData", "admin", "/adminapi/panel/panel/statisticalData?days=30",
         check=axis_of(["longitudinalAxisAccess", "longitudinalAxisOrderNum",
                        "longitudinalAxisOrderAmount"], min_points=25),
         # 访问量合计是「静默全 0」的重灾区，进基线守卫
         metrics=lambda d: {"access_sum": _sum(d["longitudinalAxisAccess"]),
                            "order_num_sum": _sum(d["longitudinalAxisOrderNum"]),
                            "points": len(d["horizontalAxis"])}),
    Case("admin.panel.vendorStatisticalData", "admin",
         "/adminapi/panel/panel/vendorStatisticalData?days=30",
         check=axis_of(["longitudinalAxisIncome", "longitudinalAxisOrderNum"], min_points=25)),
    Case("admin.statistics.accessStatistics", "admin",
         "/adminapi/panel/statisticsAccess/accessStatistics"
         "?startTime=%s&endTime=%s&isHits=1" % (days_ago(29), days_ago(0)),
         check=axis_of(["longitudinalAxis"], min_points=25),
         metrics=lambda d: {"click_sum": _sum(d["longitudinalAxis"])}),
    Case("admin.statistics.addUserTrends", "admin",
         # dateType=1（年）横轴恒为 12 个「01..12」，是这套接口里唯一长度确定的刻度，
         # 适合做结构断言；dateType=3（日）横轴长度随区间漂移（24/31 都出现过）。
         "/adminapi/panel/statisticsUser/addUserTrends"
         "?startEndTime=%d-01-01,%s&dateType=1" % (_dt.date.today().year, days_ago(0)),
         check=lambda d: (axis_of(["longitudinalAxis"], min_points=12, labels="num")(d),
                          need(len(d["horizontalAxis"]) == 12,
                               "年刻度应为 12 个月，实际 %d" % len(d["horizontalAxis"])))),
    Case("admin.stockWarning.list", "admin", "/adminapi/product/stockWarning/list?page=1&size=3",
         check=paged(total_min=1, fields={"productId": "int>0", "productName": "str!",
                                          "skuId": "int>=0", "productSn": "str!"}),
         metrics=lambda d: {"total": d["total"]}),
    Case("admin.stockWarning.setting", "admin", "/adminapi/product/stockWarning/setting",
         check=obj({"notifyEnabled": "bool01", "defaultLowerLimit": "int>=0",
                    "shopId": "int>=0"})),
    Case("admin.lang.currency.dictionary", "admin", "/adminapi/lang/currency/dictionary",
         check=lambda d: _currency_check(d),
         metrics=lambda d: {"count": len(d)}),
    Case("admin.order.list", "admin", "/adminapi/order/order/list?page=1&size=3",
         check=paged(total_min=1, fields={"orderId": "int>0", "orderSn": "str!",
                                          "orderStatus": "int", "userId": "int>0"}),
         metrics=lambda d: {"total": d["total"]}),
    Case("admin.order.log.list", "admin", "/adminapi/order/orderLog/list?page=1&size=3",
         check=paged(total_min=1, fields={"logId": "int>0", "logTime": "str!",
                                          "description": "str"}),
         metrics=lambda d: {"total": d["total"]}),
    Case("admin.coupon.list", "admin", "/adminapi/promotion/coupon/list?page=1&size=3",
         check=paged(total_min=1, fields={"couponId": "int>0", "couponName": "str!",
                                          "couponMoney": "num"}),
         metrics=lambda d: {"total": d["total"]}),
    Case("admin.groupon.list", "admin", "/adminapi/promotion/groupon/list?page=1&size=3",
         check=paged(total_min=0)),
    Case("admin.coupon.issue.users", "admin",
         "/adminapi/promotion/coupon/issue/users?page=1&size=2",
         check=paged(total_min=0)),
    Case("admin.finance.userBalanceLog", "admin",
         "/adminapi/finance/userBalanceLog/list?page=1&size=3",
         check=paged(total_min=1, fields={"logId": "int>0", "username": "str!",
                                          "balance": "num", "afterBalance": "num"}),
         metrics=lambda d: {"total": d["total"]}),
    Case("admin.finance.accountPanel", "admin", "/adminapi/finance/accountPanel/list?page=1&size=3",
         check=obj({"balance": "num", "frozenMoney": "num", "surplus": "num",
                    "toCashAmount": "num", "voucherAmount": "num", "usePoints": "num"})),
    Case("admin.finance.staffFundWallet", "admin",
         "/adminapi/finance/staffFundWallet/overview",
         check=obj({"availableBalance": "num", "frozenBalance": "num",
                    "totalIncome": "num", "totalWithdrawn": "num"})),
    Case("admin.finance.categoryFeeRule", "admin",
         "/adminapi/finance/categoryFeeRule/list",
         method="POST", body={"targetType": 1, "page": 1, "size": 2},
         check=paged(total_min=0)),
    Case("admin.finance.merchantApplication", "admin",
         "/adminapi/finance/merchantApplication/list?page=1&size=2",
         check=paged(total_min=0)),
    Case("admin.vendor.account", "admin", "/adminapi/vendor/vendorAccount/list?page=1&size=2",
         check=paged(total_min=0)),
    Case("admin.vendor.account.log", "admin",
         "/adminapi/vendor/vendorAccount/logList?page=1&size=2", check=paged(total_min=0)),
    Case("admin.vendor.user", "admin",
         "/adminapi/vendor/adminVendorUser/list?page=1&size=2", check=paged(total_min=0)),
    Case("admin.vendor.withdraw", "admin",
         "/adminapi/vendor/vendorWithdraw/list?page=1&size=2", check=paged(total_min=0)),
    Case("admin.vendor.withdraw.summary", "admin",
         "/adminapi/vendor/vendorWithdraw/summaryList", check=paged(total_min=0)),
    Case("admin.vendor.shopBind", "admin",
         "/adminapi/vendor/vendorShopBind/list?page=1&size=2", check=paged(total_min=0)),
    Case("admin.vendor.merchant", "admin",
         "/adminapi/vendor/vendorShopBind/merchantList?page=1&size=2",
         check=paged(total_min=1, fields={"merchant_id": "int>0"})),
    Case("admin.vendor.stockAdd", "admin",
         "/adminapi/vendor/stock/getAddByList?page=1&size=2", check=paged(total_min=0)),
    Case("admin.product.config", "admin", "/adminapi/product/product/config",
         check=lambda d: (
             check_fields(d, {"serviceList": "list!", "shippingTplList": "list!",
                              "userRankList": "list!", "eCardList": "list"}),
             check_fields(d["shippingTplList"][0], {"shipping_tpl_id": "int>0",
                                                    "shipping_tpl_name": "str!"},
                          where="shippingTplList[0]")),
         metrics=lambda d: {"shippingTpl": len(d["shippingTplList"]),
                            "service": len(d["serviceList"])}),
    Case("admin.product.shippingTplList", "admin", "/adminapi/product/product/shippingTplList",
         check=listing(min_len=1, fields={"shipping_tpl_id": "int>0",
                                          "shipping_tpl_name": "str!",
                                          "pricing_type": "int"}),
         metrics=lambda d: {"count": len(d)}),
    Case("admin.setting.shippingTpl", "admin",
         "/adminapi/setting/shippingTpl/list?page=1&size=2",
         check=paged(total_min=1, fields={"shippingTplId": "int>0",
                                          "shippingTplName": "str!",
                                          "pricingType": "int"}),
         metrics=lambda d: {"total": d["total"]}),
    Case("admin.authority.adminLog", "admin",
         "/adminapi/authority/adminLog/list?page=1&size=2",
         check=paged(total_min=1, fields={"logId": "int>0", "username": "str",
                                          "logTime": "any"}),
         metrics=lambda d: {"total": d["total"]}),
    Case("admin.content.wechatLive", "admin",
         "/adminapi/content/wechatLive/list?page=1&size=2",
         check=paged(total_min=0, fields={"wechatLiveId": "int>0",
                                          "wechatLiveTitle": "str!"})),
    Case("admin.wechat.menu", "admin", "/adminapi/wechat/menu",
         check=lambda d: check_fields(d.get("tree") or {}, {"buttons": "list"},
                                      where="tree")),
    Case("admin.o2o.staffWallet", "admin", "/adminapi/o2o/serviceStaffWallet/detail",
         check=obj({"walletId": "int>=0", "staffId": "int>=0", "balance": "num"})),
    Case("admin.proxy.pickupList", "admin",
         "/adminapi/proxyShopping/order/check/pickupList?page=1&size=2",
         check=paged(total_min=0)),
    Case("admin.panel.searchMenu", "admin", "/adminapi/panel/panel/searchMenu?keyword=order",
         check=lambda d: need(isinstance(d, list), "期望数组，实际 %s" % type(d).__name__)),
]


def _panel_index_check(d):
    need(isinstance(d.get("consoleData"), dict), "缺 consoleData")
    c = d["consoleData"]
    check_fields(c, {"awaitPay": "int>=0", "awaitShip": "int>=0", "awaitAfterSale": "int>=0",
                     "awaitComment": "int>=0", "productOnSaleNum": "int>0",
                     "productTotalNum": "int>0", "todayOrderNum": "int>=0",
                     "todayOrderAmount": "num"}, where="consoleData")
    need(c["productTotalNum"] >= c["productOnSaleNum"],
         "商品总数(%d) 小于在售数(%d)" % (c["productTotalNum"], c["productOnSaleNum"]))
    check_fields(d.get("realTimeData") or {}, {"todayVisitNum": "int>=0",
                                               "viewGrowthRate": "num"}, where="realTimeData")


def _currency_check(d):
    need(isinstance(d, list) and len(d) >= 100, "币种字典不足 100 条（实际 %s）" %
         (len(d) if isinstance(d, list) else d))
    for row in d[:5]:
        check_fields(row, {"id": "int>0", "code": "str!", "name": "str!"}, where="dictionary[0]")
    codes = [r.get("code") for r in d]
    need(len(set(codes)) == len(codes), "币种 code 有重复")


CASES = MEMBER_CASES + ADMIN_CASES


# --------------------------------------------------------------------------- #
# 跨接口一致性
# --------------------------------------------------------------------------- #
def cross_checks(ctx, cache):
    """返回 [(名称, 失败原因或 None, 说明)]。全部不依赖数据量。"""

    def pick(cid):
        if cid not in cache:
            raise Skip("%s 未通过" % cid)
        return cache[cid]["data"]

    out = []

    # 1) statisticalData 的访问量合计 == accessStatistics 同区间的点击量合计
    name = "xcheck.statisticalData↔accessStatistics"
    try:
        sd = pick("admin.panel.statisticalData")
        start, end = sd["horizontalAxis"][0], sd["horizontalAxis"][-1]
        st, txt = ctx["admin"].call(
            "/adminapi/panel/statisticsAccess/accessStatistics"
            "?startTime=%s&endTime=%s&isHits=1" % (start, end))
        body = json.loads(txt)
        need(st == 200 and body.get("code") == 0, "accessStatistics 调用失败：%s" % txt[:120])
        other = _sum(body["data"]["longitudinalAxis"])
        mine = _sum(sd["longitudinalAxisAccess"])
        # 两次请求之间仍可能有真实访问进来，且平台口径与 shop_id=0 口径理论上可不同，
        # 故用比值判定：相差 <10% 视为一致；被吞成 0 时比值直接崩到 0。
        big, small = max(mine, other, 1), min(mine, other)
        need(small >= big * 0.9,
             "同一 statistics_base 区间（%s~%s）两条曲线差异过大：statisticalData=%d "
             "accessStatistics=%d —— 通常意味着其中一条被 unwrap_or_default 吞成 0"
             % (start, end, mine, other))
        out.append((name, None, "%s~%s 合计 %d vs %d" % (start, end, mine, other)))
    except Skip as e:
        out.append((name, None, "跳过（%s）" % e))
    except AssertFail as e:
        out.append((name, str(e), ""))

    # 2) panel/index 的待发货数 == 订单列表 orderStatus=1 的 total
    name = "xcheck.panelIndex.awaitShip"
    try:
        want = pick("admin.panel.index")["consoleData"]["awaitShip"]
        st, txt = ctx["admin"].call("/adminapi/order/order/list?page=1&size=1&orderStatus=1")
        got = json.loads(txt)["data"]["total"]
        need(want == got, "面板 awaitShip=%d，订单列表 orderStatus=1 total=%d" % (want, got))
        out.append((name, None, "均 %d" % want))
    except Skip as e:
        out.append((name, None, "跳过（%s）" % e))
    except AssertFail as e:
        out.append((name, str(e), ""))

    # 3) panel/index 商品数与商品列表一致（在售 / 总数）
    name = "xcheck.panelIndex.productCounts"
    try:
        c = pick("admin.panel.index")["consoleData"]
        st, txt = ctx["admin"].call("/adminapi/product/product/list?page=1&size=1")
        total = json.loads(txt)["data"]["total"]
        st, txt = ctx["admin"].call("/adminapi/product/product/list?page=1&size=1&productStatus=1")
        on_sale = json.loads(txt)["data"]["total"]
        need(c["productTotalNum"] == total,
             "面板 productTotalNum=%d，商品列表 total=%d" % (c["productTotalNum"], total))
        need(c["productOnSaleNum"] == on_sale,
             "面板 productOnSaleNum=%d，在售 total=%d" % (c["productOnSaleNum"], on_sale))
        out.append((name, None, "总 %d / 在售 %d" % (total, on_sale)))
    except Skip as e:
        out.append((name, None, "跳过（%s）" % e))
    except AssertFail as e:
        out.append((name, str(e), ""))

    # 4) 购物车角标 == 购物车明细合计
    name = "xcheck.cart.count"
    try:
        badge = pick("member.cart.getCount")
        in_list = pick("member.cart.list")["total"]["totalCount"]
        need(badge == in_list, "getCount=%s，cartList.totalCount=%s" % (badge, in_list))
        out.append((name, None, "均 %s" % badge))
    except Skip as e:
        out.append((name, None, "跳过（%s）" % e))
    except AssertFail as e:
        out.append((name, str(e), ""))

    return out


# --------------------------------------------------------------------------- #
# 基线
# --------------------------------------------------------------------------- #
def load_baseline(path):
    if not os.path.exists(path):
        return {}
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except (ValueError, OSError):
        return {}


def baseline_guard(metrics, baseline, strict):
    """基线 > 0 而本次为 0 → 失败（静默丢数据）；strict 时低于一半也失败。"""
    problems = []
    for key, cur in metrics.items():
        if key not in baseline:
            continue
        base = _num(baseline[key])
        if base is None or base <= 0:
            continue
        cur_n = _num(cur)
        if cur_n is None or cur_n <= 0:
            problems.append("基线 %s=%g，本次 %s —— 疑似静默丢数据" % (key, base, cur))
        elif strict and cur_n < base * 0.5:
            problems.append("基线 %s=%g，本次 %g（不足一半）" % (key, base, cur_n))
    return problems


# --------------------------------------------------------------------------- #
# 自检：证明「该挂的时候会挂」（不需要网络）
# --------------------------------------------------------------------------- #
def self_test():
    """逐条验证断言原语的正反两面，防止门禁沦为安慰剂。"""
    cases = []

    def expect_fail(name, fn):
        try:
            fn()
        except AssertFail:
            cases.append((name, True, ""))
        except Exception as e:  # noqa: BLE001
            cases.append((name, False, "抛了别的异常：%s" % e))
        else:
            cases.append((name, False, "本该失败却通过了"))

    def expect_pass(name, fn):
        try:
            fn()
        except Exception as e:  # noqa: BLE001
            cases.append((name, False, str(e)))
        else:
            cases.append((name, True, ""))

    # —— 分页：total 说有、records 却空 = 查询被吞
    expect_fail("paged: total=5 但 records 为空",
                lambda: paged()({"total": 5, "records": []}))
    expect_fail("paged: 字段 productId 为 0",
                lambda: paged(fields={"productId": "int>0"})(
                    {"total": 1, "records": [{"productId": 0}]}))
    expect_fail("paged: 缺 total",
                lambda: paged()({"records": [{"a": 1}]}))
    expect_fail("paged: records 多于 total",
                lambda: paged()({"total": 1, "records": [{"a": 1}, {"a": 2}]}))
    expect_pass("paged: 正常",
                lambda: paged(fields={"productId": "int>0"})(
                    {"total": 2, "records": [{"productId": 7}], "size": 1, "current": 1}))

    # —— 坐标轴：全 0 能过结构断言，但会被基线守卫抓住（见下）
    axis = axis_of(["longitudinalAxis"])
    expect_fail("axis: 纵轴长度与横轴不齐",
                lambda: axis({"horizontalAxis": ["2026-11-01", "2026-11-02"],
                              "longitudinalAxis": [0]}))
    expect_fail("axis: 日期未升序",
                lambda: axis({"horizontalAxis": ["2026-11-02", "2026-11-01"],
                              "longitudinalAxis": [1, 2]}))
    expect_fail("axis: 日期格式非法",
                lambda: axis({"horizontalAxis": ["11-01"], "longitudinalAxis": [1]}))
    expect_fail("axis: 纵轴出现非数字",
                lambda: axis({"horizontalAxis": ["2026-11-01"],
                              "longitudinalAxis": ["-"]}))
    expect_pass("axis: 全 0 结构合法（靠基线对照判业务失败）",
                lambda: axis({"horizontalAxis": ["2026-11-01"], "longitudinalAxis": [0]}))

    # —— 基线守卫：基线 402 → 本次 0，正是 statisticalData 那次静默丢数据
    expect_fail("baseline: 基线 402 本次 0",
                lambda: need(not baseline_guard({"k": 0}, {"k": 402}, False),
                             "基线 402 → 0"))
    expect_pass("baseline: 基线 0 不设限",
                lambda: need(not baseline_guard({"k": 0}, {"k": 0}, True), "x"))
    expect_fail("baseline: strict 跌破一半",
                lambda: need(not baseline_guard({"k": 100}, {"k": 400}, True), "x"))

    # —— 字段类型
    expect_fail("kind: int 拒绝 bool",
                lambda: check_fields({"a": True}, {"a": "int"}))
    expect_fail("kind: str! 拒绝空白串",
                lambda: check_fields({"a": "   "}, {"a": "str!"}))
    expect_pass("kind: num 接受字符串金额",
                lambda: check_fields({"a": "47690.20"}, {"a": "num"}))

    width = max(len(n) for n, _, _ in cases)
    bad = 0
    for name, ok, info in cases:
        print("%s  %-*s  %s" % ("PASS" if ok else "FAIL", width, name, info))
        bad += 0 if ok else 1
    print("\n自检 %d 项 / 失败 %d" % (len(cases), bad))
    return 0 if bad == 0 else 1


# --------------------------------------------------------------------------- #
# 运行器
# --------------------------------------------------------------------------- #
def run_case(ctx, case, baseline, strict, verbose):
    started = time.time()
    res = {"id": case.id, "side": case.side, "method": case.method, "path": case.path,
           "ok": False, "http": None, "code": None, "detail": "", "ms": 0,
           "metrics": {}, "_data": None}
    try:
        st, txt = ctx[case.side].call(case.path, case.body, method=case.method)
        res["http"] = st
        need(st == 200, "HTTP %s：%s" % (st, (txt or "<空响应>")[:160]))
        try:
            body = json.loads(txt)
        except ValueError:
            raise AssertFail("响应不是 JSON：%s" % txt[:160])
        need(isinstance(body, dict), "响应顶层不是对象")
        res["code"] = body.get("code")
        need(body.get("code") == 0,
             "业务码 code=%r msg=%r" % (body.get("code"),
                                        body.get("msg") or body.get("message")))
        data = body.get("data")
        need(data is not None, "data 为 null —— 查询被吞或未取到")
        if case.check:
            case.check(data)
        if case.metrics:
            res["metrics"] = {"%s.%s" % (case.id, k): v
                              for k, v in case.metrics(data).items()}
        problems = baseline_guard(res["metrics"], baseline, strict)
        need(not problems, "；".join(problems))
        res["ok"] = True
        res["_data"] = data
        res["detail"] = _brief(data)
    except AssertFail as e:
        res["detail"] = str(e)
    except Exception as e:  # noqa: BLE001 - 任何未预期异常都要记成失败而非崩溃
        res["detail"] = "未预期异常 %s: %s" % (type(e).__name__, e)
    res["ms"] = int((time.time() - started) * 1000)
    if verbose and not res["ok"]:
        print("  ↳ %s %s\n    %s" % (case.method, case.path, res["detail"]))
    return res


def _brief(data):
    if isinstance(data, dict):
        if "total" in data and "records" in data:
            return "total=%s recs=%d" % (data["total"], len(data["records"]))
        if "horizontalAxis" in data:
            return "axis=%d" % len(data["horizontalAxis"])
        return "keys=%d" % len(data)
    if isinstance(data, list):
        return "list=%d" % len(data)
    return "%s" % (data,)


def main(argv=None):
    ap = argparse.ArgumentParser(description="关键接口业务字段回归断言（抓 200 但静默失败）")
    ap.add_argument("--base", default=DEFAULT_BASE, help="两侧共用域名（默认 %s）" % DEFAULT_BASE)
    ap.add_argument("--member-base", help="会员端根地址（默认同 --base）")
    ap.add_argument("--admin-base", help="管理端根地址（默认同 --base）")
    ap.add_argument("--only", choices=("all", "member", "admin"), default="all")
    ap.add_argument("--baseline", default=BASELINE_PATH, help="基线文件路径")
    ap.add_argument("--update-baseline", action="store_true", help="用本次结果重建基线")
    ap.add_argument("--strict", action="store_true", help="基线跌破一半也判失败")
    ap.add_argument("--timeout", type=float, default=40.0)
    ap.add_argument("--json", help="把结果写成 JSON")
    ap.add_argument("--self-test", action="store_true",
                    help="离线自检断言原语（不访问网络），确认门禁不会永远通过")
    ap.add_argument("-v", "--verbose", action="store_true")
    args = ap.parse_args(argv)

    if args.self_test:
        return self_test()

    mbase = args.member_base or args.base
    abase = args.admin_base or args.base
    ctx = {"member": Session(Http(mbase, args.timeout), "member"),
           "admin": Session(Http(abase, args.timeout), "admin")}

    print("业务字段回归断言  member=%s  admin=%s" % (mbase, abase))
    try:
        if args.only in ("all", "member"):
            ctx["member"].login()
        if args.only in ("all", "admin"):
            ctx["admin"].login()
    except AssertFail as e:
        print("登录失败：%s" % e)
        return 2

    baseline = load_baseline(args.baseline)
    strict = args.strict
    selected = [c for c in CASES if args.only == "all" or c.side == args.only]

    results, cache, metrics_now = [], {}, {}
    for case in selected:
        res = run_case(ctx, case, baseline, strict, args.verbose)
        results.append(res)
        if res["ok"]:
            cache[case.id] = {"data": res["_data"]}
        if res["metrics"]:
            metrics_now.update(res["metrics"])

    # 跨接口一致性（前提用例缺失时该项跳过，不误报）
    cross = []
    if not args.update_baseline:
        try:
            cross = cross_checks(ctx, cache)
        except Exception as e:  # noqa: BLE001
            cross = [("xcheck", "跨接口校验异常 %s: %s" % (type(e).__name__, e), "")]

    failed = [r for r in results if not r["ok"]]
    cross_failed = [c for c in cross if c[1]]

    width = max([len(r["id"]) for r in results] + [10])
    for r in results:
        print("%s  %-*s  %-3s  %-8s  %s" % (
            "PASS" if r["ok"] else "FAIL", width, r["id"],
            r["http"] if r["http"] is not None else "-",
            "%.0fms" % r["ms"], r["detail"]))
    for name, err, info in cross:
        print("%s  %-*s  %s" % ("PASS" if not err else "FAIL", width, name, err or info))

    if args.update_baseline:
        merged = dict(baseline)
        merged.update(metrics_now)
        with open(args.baseline, "w", encoding="utf-8") as f:
            json.dump(merged, f, ensure_ascii=False, indent=2, sort_keys=True)
            f.write("\n")
        print("\n基线已更新：%s（%d 项指标）" % (args.baseline, len(merged)))

    print("\n合计 %d 用例 / 通过 %d / 失败 %d；跨接口 %d 项 / 失败 %d" % (
        len(results), len(results) - len(failed), len(failed), len(cross), len(cross_failed)))

    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump({"base": {"member": mbase, "admin": abase},
                       "cases": [{k: v for k, v in r.items() if not k.startswith("_")}
                                 for r in results],
                       "cross": [{"id": n, "error": e, "info": i} for n, e, i in cross],
                       "metrics": metrics_now}, f, ensure_ascii=False, indent=2)

    return 0 if not failed and not cross_failed else 1


if __name__ == "__main__":
    sys.exit(main())
