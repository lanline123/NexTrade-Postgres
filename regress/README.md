# 关键接口业务字段回归断言

> 解决《HANDOVER.md》§8 第 5 项「回归断言升级」。
> 起因：`/adminapi/panel/panel/statisticalData` 在 SQL 报错时被 `.unwrap_or_default()`
> 吞掉，HTTP 200 + 访问量全 0，与「当天没人访问」无法区分（审计 §30.3）。

## 这是什么

一个只依赖 Python 标准库的黑盒回归套件，对生产（或本机）后端跑 53 个用例 + 4 项跨接口
一致性校验。**只统计 HTTP 状态码的回归会漏掉「吞错误」的接口**，所以每层都往下钻：

| 层 | 断言 | 抓什么 |
|---|---|---|
| L0 | HTTP 200 | 崩溃、路由丢失 |
| L1 | `code == 0` | 200 里包着的业务错误 |
| L2 | `data` 非 null；分页 `total`/`records` 自洽；坐标轴长度对齐、日期升序 | 查询被吞、分页恒空、轴错位 |
| L3 | 关键字段存在且类型正确、主键 > 0、金额可解析 | 字段改名/丢字段/类型漂移 |
| L4 | 与 `baseline.json` 对比：基线 > 0 而本次为 0 → 失败 | **静默全 0**（本次要治的那个病） |

跨接口一致性（不依赖数据量，造数前后都成立）：

```text
statisticalData(days=30) 的访问量合计 ≈ accessStatistics 同区间点击量合计（比值 ≥ 0.9）
panel/index.awaitShip      == 订单列表 orderStatus=1 的 total
panel/index.productOnSaleNum / productTotalNum == 商品列表 productStatus=1 / 全量 total
cart/getCount              == cart/list 的 total.totalCount
```

## 用法

```bash
# 打生产
python regress/api_assert.py

# 打本机后端（会员 8181 / 管理 8080）
python regress/api_assert.py --member-base http://127.0.0.1:8181 \
                             --admin-base  http://127.0.0.1:8080

# 只看一侧 / 输出细节 / 存 JSON 结果
python regress/api_assert.py --only admin -v --json regress/last.json

# 断言原语自检（离线，不访问网络）：证明「该挂的时候会挂」
python regress/api_assert.py --self-test
```

退出码：`0` 全通过 / `1` 有失败 / `2` 登录或网络不可用。

## 基线怎么维护

`baseline.json` 记录 31 项「应当有值」的指标（各列表 total、轴合计、币种条数等）。

- 判定规则：**基线 > 0 而本次 ≤ 0 → 失败**（这就是 200 但静默丢数据）；
  加 `--strict` 时跌破基线一半也失败（适合发布后回归）。
- 业务数据正常增长不需要动基线；只有**换库 / 重置演示数据**后才需要
  `--update-baseline` 重建。
- 新增用例若带 `metrics`，首次跑完加 `--update-baseline` 把它纳入。

## 已知边界（别误判成缺陷）

1. **活数据会漂移**：`statisticalData` 与 `accessStatistics` 是两次请求，期间新访问会
   让后者略大，故跨接口用比值（≥0.9）而非严格相等。
2. **店铺口径差异**：`statisticalData` 不按店铺过滤，`accessStatistics` 平台账号走
   `shop_id = 0`。当前埋点行全是 `shop_id = 0`，两者一致；若将来出现 `shop_id ≠ 0`
   或 `NULL` 的埋点行，这条一致性校验需要改成按店铺对齐的口径。
3. **`dateType` 语义**：`addUserTrends` 的 `dateType=3`（日）横轴长度随区间漂移
   （24/31 都出现过，属 Java 既有的刻度错位行为），因此断言改用 `dateType=1`（恒 12 个月）。
4. **空数据集端点**（供应商、拼团、商户进件等）只做结构断言，`total=0` 属正常；
   这类端点不做基线守卫，避免把「本来没有数据」误判为丢数据。

## 与其它门禁的关系

| 手段 | 覆盖 | 何时跑 |
|---|---|---|
| `check-rust.yml` | 编译期 | 每次 push |
| `scratch/audit_type_compat.py` 等四道扫描 | PG 列类型 × Rust 字段 / text 化列日期函数 | 改 SQL 后，本地 |
| **本套件** | **运行期真实业务取值** | 每次部署后（打生产）、改接口后（打本机） |

类型/语法问题编译期全通过，只有真实 SQL 打到库里才暴露；本套件补的正是这一段。
