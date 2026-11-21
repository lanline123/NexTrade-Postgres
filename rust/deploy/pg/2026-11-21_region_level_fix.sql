-- 2026-11-21: region 表国家层 level 口径统一 —— Tigshop 论坛帖 407879 前置修复。
--
-- 问题（2026-11-21 现网实测）：存量国家行 level=0（中国 region_id=1 等），
-- 而 2026-11-18 美/加种子按 level=1 写入；getCountryList 按 level=1 查询，
-- 结果只返回美国、加拿大，中国被漏掉。
-- 实测层级分布（修复前）：level 0=2 行（中国 + 一行无 standard_code 的遗留「美国」
-- 空根行 region_id=820218，无子级）、1=2 行（美/加）、2=98、3=369、4=3476；
-- 中国子树本身即为 省=2 / 市=3 / 区县=4，与 Java 原版口径一致——错位仅在国家行本身，
-- 故本修复只动根行 level，不触碰任何子树。
--
-- 口径：凡带 standard_code 的根行（parent_id=0）统一为 level=1。
-- 无 standard_code 的遗留空根行（820218）刻意不动：它无子级、无代码，
-- 任何按 level 的消费方都不会把它当国家命中（getCountryList 取 level=1，
-- IP 定位国家匹配同轮改为 level=1，见 region_client.rs），保留原样
-- 行为中性；其清理与本批解耦。
--
-- 消费方影响（已逐一核查）：getCountryList(level=1) 修复后含中国；
-- getProvinceList(level=2) / getCityRegion(level=3) 数据未动、无影响；
-- 后台地区树 level>0（国内版）/ >=0（海外版）过滤：修复后中国根行在国内版
-- 亦可见（原先被 >0 过滤挡在树外，属一并修复）；按 region_id 查名的各仓储与 level 无关。
--
-- 幂等：重复执行无副作用（已为 1 的行不再命中条件）。
-- 执行顺序：本文件为第 1 步 —— 先于 region_country_import.sql 与
-- region_state_import_1..2.sql 执行；与后端二进制无强顺序（新旧口径均可降级运行）。
BEGIN;

UPDATE region
SET level = 1
WHERE parent_id = 0
  AND level = 0
  AND standard_code IS NOT NULL
  AND standard_code <> '';

COMMIT;
