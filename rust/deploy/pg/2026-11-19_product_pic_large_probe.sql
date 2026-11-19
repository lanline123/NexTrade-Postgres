-- product / product_gallery 的 pic_large 存量探针（只读，不写库）
--
-- 背景：product.pic_url / pic_large 的 size_400 / size_800 派生（Java
-- ProductServiceImpl#setImgList）写入侧未迁移，代码长期写裸值/空值；
-- 存量则混合了种子数据（Java pure.sql 血统：pic_url=h_500、pic_large=
-- h_800）、历次数据治理与代码裸写四个来源，形态不一致（2026-11-19
-- 本机普查：product 137 行中 pic_large 已派生 63 / 裸 28 / 空 46；
-- product_gallery 2855 行中已派生 443 / 裸 2241）。
-- 本探针输出归一目标集的行数与 md5，执行归一脚本前先跑本探针并记录
-- 结果，再把两个 md5 分别填入归一脚本的守卫占位符。
--
-- 归一范围（仅 pic_large，pic_url 不归一，理由见归一脚本头注）：
--   product：pic_large 不带 x-oss-process，且 pic_original 非空，且
--     pic_large 与 pic_original 逐字相同（裸同图）、或 pic_large 为空、
--     或 pic_large 与 pic_url 的裸基址相同（同图裸值）。
--   product_gallery：pic_large 不带 x-oss-process，且 pic_original 非空，
--     且 pic_large 与 pic_original 逐字相同、或与 pic_url 裸基址相同。
-- 异图裸 pic_large（与 pic_original 不同图，如门店覆盖图）不在归一范围，
-- 仅在第 4 段列清单人工核对。

-- 1) product 归一目标集行数 + md5（填入归一脚本 __EXPECTED_PRODUCT_MD5__）
SELECT count(*) AS target_count,
       md5(coalesce(string_agg(product_id::text || ':' || coalesce(pic_large, '') || ':' || coalesce(pic_original, ''), ',' ORDER BY product_id), '')) AS target_md5
FROM product
WHERE is_delete = 0
  AND coalesce(pic_large, '') NOT LIKE '%x-oss-process=%'
  AND coalesce(pic_original, '') <> ''
  AND (coalesce(pic_large, '') = ''
       OR pic_large = pic_original
       OR pic_large = split_part(coalesce(pic_url, ''), '?', 1));

-- 2) product_gallery 归一目标集行数 + md5（填入归一脚本 __EXPECTED_GALLERY_MD5__）
SELECT count(*) AS target_count,
       md5(coalesce(string_agg(pic_id::text || ':' || coalesce(pic_large, '') || ':' || coalesce(pic_original, ''), ',' ORDER BY pic_id), '')) AS target_md5
FROM product_gallery
WHERE coalesce(pic_large, '') NOT LIKE '%x-oss-process=%'
  AND coalesce(pic_original, '') <> ''
  AND (pic_large = pic_original
       OR pic_large = split_part(coalesce(pic_url, ''), '?', 1));

-- 3) product.pic_url 形态普查（仅留档，本轮不归一）
SELECT CASE
           WHEN coalesce(pic_url, '') = '' THEN 'empty'
           WHEN pic_url LIKE '%h_500%' THEN 'derived_size400'
           WHEN pic_url LIKE '%x-oss-process=%' THEN 'derived_other'
           ELSE 'bare'
       END AS pic_url_form,
       count(*)
FROM product
WHERE is_delete = 0
GROUP BY 1
ORDER BY 1;

-- 4) 异图裸 pic_large 清单（不归一，人工核对用）
SELECT 'product' AS src, product_id AS id, pic_url, pic_original, pic_large
FROM product
WHERE is_delete = 0
  AND coalesce(pic_large, '') <> ''
  AND pic_large NOT LIKE '%x-oss-process=%'
  AND coalesce(pic_original, '') <> ''
  AND pic_large <> pic_original
  AND pic_large <> split_part(coalesce(pic_url, ''), '?', 1)
UNION ALL
SELECT 'product_gallery', pic_id, pic_url, pic_original, pic_large
FROM product_gallery
WHERE coalesce(pic_large, '') <> ''
  AND pic_large NOT LIKE '%x-oss-process=%'
  AND coalesce(pic_original, '') <> ''
  AND pic_large <> pic_original
  AND pic_large <> split_part(coalesce(pic_url, ''), '?', 1)
ORDER BY 1, 2;
