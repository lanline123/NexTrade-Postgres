-- product / product_gallery 的 pic_large 存量归一（size_800 派生）
--
-- 口径：pic_large := pic_original + OSS 800 处理参数
--   （?x-oss-process=image/resize,m_pad,h_800,h_800，逐字取 Java
--   PicConstants.OSS_SIZE_800；基址已带 '?' 时以 '&' 续接）。
--   基址取 pic_original（恒裸值字段）；目标集定义见同目录 probe 脚本。
--
-- pic_url 本轮不归一：Java setImgList 对 pic_url 的派生是条件式的
--   （仅新增拼 size_400、编辑存裸值），存量裸 pic_url 无法区分「编辑后
--   裸值」（原版稳态）与「新增漏派生」，全量拼参会造出原版不存在的
--   状态；新增/编辑自本批代码起按原版分叉口径写入，自然收敛。
--
-- 执行时点：随 pic_url/pic_large 派生写入侧后端批次部署时，在服务器
--   本机执行（/www/server/pgsql/bin/psql -U postgres -d nextrade）。
--   先跑同目录 probe 脚本记录两个 target_md5，分别替换下方
--   __EXPECTED_PRODUCT_MD5__ / __EXPECTED_GALLERY_MD5__ 后再执行；
--   未替换会主动报错中止。建议在新后端换版完成后执行（旧代码仍会把
--   编辑过的商品 pic_large 写回裸值，换版后执行可避免归一成果被回退）。
-- 安全：单事务；先备份原值到 bak_product_pic_large_20261119 /
--   bak_product_gallery_pic_large_20261119（主键， 重跑不覆盖首份备份）；
--   md5 守卫与归一后校验任一不符即整事务回滚。
-- 回滚：UPDATE product p SET pic_large = b.pic_large
--         FROM bak_product_pic_large_20261119 b
--         WHERE p.product_id = b.product_id;
--       UPDATE product_gallery g SET pic_large = b.pic_large
--         FROM bak_product_gallery_pic_large_20261119 b
--         WHERE g.pic_id = b.pic_id;

\set ON_ERROR_STOP on
BEGIN;

-- 1) 备份目标集原值（幂等：已备份行不覆盖）
CREATE TABLE IF NOT EXISTS bak_product_pic_large_20261119 (
    product_id integer PRIMARY KEY,
    pic_url text,
    pic_original text,
    pic_large text,
    backed_up_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO bak_product_pic_large_20261119 (product_id, pic_url, pic_original, pic_large)
SELECT product_id, pic_url, pic_original, pic_large
FROM product
WHERE is_delete = 0
  AND coalesce(pic_large, '') NOT LIKE '%x-oss-process=%'
  AND coalesce(pic_original, '') <> ''
  AND (coalesce(pic_large, '') = ''
       OR pic_large = pic_original
       OR pic_large = split_part(coalesce(pic_url, ''), '?', 1))
ON CONFLICT (product_id) DO NOTHING;

CREATE TABLE IF NOT EXISTS bak_product_gallery_pic_large_20261119 (
    pic_id integer PRIMARY KEY,
    product_id integer,
    pic_url text,
    pic_original text,
    pic_large text,
    backed_up_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO bak_product_gallery_pic_large_20261119 (pic_id, product_id, pic_url, pic_original, pic_large)
SELECT pic_id, product_id, pic_url, pic_original, pic_large
FROM product_gallery
WHERE coalesce(pic_large, '') NOT LIKE '%x-oss-process=%'
  AND coalesce(pic_original, '') <> ''
  AND (pic_large = pic_original
       OR pic_large = split_part(coalesce(pic_url, ''), '?', 1))
ON CONFLICT (pic_id) DO NOTHING;

-- 2) md5 守卫：两个备份集必须与探针一致
DO $$
DECLARE
    expected_product text := '__EXPECTED_PRODUCT_MD5__';
    expected_gallery text := '__EXPECTED_GALLERY_MD5__';
    cur text;
    n int;
BEGIN
    IF position('EXPECTED_PRODUCT_MD5' in expected_product) > 0
       OR position('EXPECTED_GALLERY_MD5' in expected_gallery) > 0 THEN
        RAISE EXCEPTION 'md5 守卫未填：先跑 probe 脚本并替换两个占位符后再执行';
    END IF;
    SELECT md5(coalesce(string_agg(product_id::text || ':' || coalesce(pic_large, '') || ':' || coalesce(pic_original, ''), ',' ORDER BY product_id), '')), count(*)
      INTO cur, n
      FROM bak_product_pic_large_20261119;
    IF cur <> expected_product THEN
        RAISE EXCEPTION 'product md5 守卫不符：探针 % ≠ 当前备份集 %（行数 %），中止', expected_product, cur, n;
    END IF;
    SELECT md5(coalesce(string_agg(pic_id::text || ':' || coalesce(pic_large, '') || ':' || coalesce(pic_original, ''), ',' ORDER BY pic_id), '')), count(*)
      INTO cur, n
      FROM bak_product_gallery_pic_large_20261119;
    IF cur <> expected_gallery THEN
        RAISE EXCEPTION 'product_gallery md5 守卫不符：探针 % ≠ 当前备份集 %（行数 %），中止', expected_gallery, cur, n;
    END IF;
END $$;

-- 3) 归一（仅备份集行；基址 = pic_original，带 '?' 时 & 续接）
UPDATE product p
SET pic_large = CASE
        WHEN b.pic_original LIKE '%?%'
            THEN b.pic_original || '&x-oss-process=image/resize,m_pad,h_800,h_800'
        ELSE b.pic_original || '?x-oss-process=image/resize,m_pad,h_800,h_800'
    END
FROM bak_product_pic_large_20261119 b
WHERE p.product_id = b.product_id;

UPDATE product_gallery g
SET pic_large = CASE
        WHEN b.pic_original LIKE '%?%'
            THEN b.pic_original || '&x-oss-process=image/resize,m_pad,h_800,h_800'
        ELSE b.pic_original || '?x-oss-process=image/resize,m_pad,h_800,h_800'
    END
FROM bak_product_gallery_pic_large_20261119 b
WHERE g.pic_id = b.pic_id;

-- 4) 归一后校验：备份集内 pic_large 必须全部带 h_800 派生
DO $$
DECLARE n int;
BEGIN
    SELECT count(*) INTO n
    FROM product p
    JOIN bak_product_pic_large_20261119 b ON b.product_id = p.product_id
    WHERE coalesce(p.pic_large, '') NOT LIKE '%x-oss-process=image/resize,m_pad,h_800,h_800%';
    IF n > 0 THEN
        RAISE EXCEPTION 'product 归一后仍有 % 行不符合口径，中止回滚', n;
    END IF;
    SELECT count(*) INTO n
    FROM product_gallery g
    JOIN bak_product_gallery_pic_large_20261119 b ON b.pic_id = g.pic_id
    WHERE coalesce(g.pic_large, '') NOT LIKE '%x-oss-process=image/resize,m_pad,h_800,h_800%';
    IF n > 0 THEN
        RAISE EXCEPTION 'product_gallery 归一后仍有 % 行不符合口径，中止回滚', n;
    END IF;
END $$;

COMMIT;

-- 5) 执行结果留档（行数 + 归一后 md5，记入部署报告）
SELECT 'product' AS src, count(*) AS normalized_count,
       md5(coalesce(string_agg(p.product_id::text || ':' || p.pic_large, ',' ORDER BY p.product_id), '')) AS normalized_md5
FROM product p
JOIN bak_product_pic_large_20261119 b ON b.product_id = p.product_id
UNION ALL
SELECT 'product_gallery', count(*),
       md5(coalesce(string_agg(g.pic_id::text || ':' || g.pic_large, ',' ORDER BY g.pic_id), ''))
FROM product_gallery g
JOIN bak_product_gallery_pic_large_20261119 b ON b.pic_id = g.pic_id;
