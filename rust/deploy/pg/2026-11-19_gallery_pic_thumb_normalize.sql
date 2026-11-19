-- gallery_pic.pic_thumb 存量归一（68 条裸 thumb，2026-11-19 普查数）
--
-- 口径：pic_thumb := pic_url + OSS 200 处理参数
--   （?x-oss-process=image/resize,m_pad,h_200,w_200；pic_url 已带 '?' 时以 '&' 续接）。
--   与 2026-11-19 v22 数据治理已归一的 product 35 条同拼写（见 common::utils::OSS_SIZE_200 注释）。
-- 范围：仅探针定义的裸 thumb 目标集（pic_thumb 为空或与 pic_url 逐字相同）；
--   异图裸 thumb 不动（探针第 3 段清单人工核对）。
--
-- 执行时点：随 pic_thumb 写入侧后端批次部署时，在服务器本机执行
--   （/www/server/pgsql/bin/psql -U postgres -d nextrade）。
--   先跑同目录 probe 脚本记录 bare_count / bare_md5，把 bare_md5 替换下方
--   __EXPECTED_MD5__ 占位符后再执行本脚本；未替换会主动报错中止。
--   注意：旧二进制仍在运行时探针后新增的裸行会使 md5 守卫中止（预期行为，
--   重新探针即可）；建议在新后端换版完成后执行。
-- 安全：单事务；先备份原值到 bak_gallery_pic_thumb_20261119（pic_id 主键，
--   重跑不覆盖首份备份）；md5 守卫与归一后校验任一不符即整事务回滚。
-- 回滚：UPDATE gallery_pic g SET pic_thumb = b.pic_thumb
--         FROM bak_gallery_pic_thumb_20261119 b WHERE g.pic_id = b.pic_id;

\set ON_ERROR_STOP on
BEGIN;

-- 1) 备份目标集原值（幂等：已备份行不覆盖）
CREATE TABLE IF NOT EXISTS bak_gallery_pic_thumb_20261119 (
    pic_id integer PRIMARY KEY,
    pic_url text,
    pic_thumb text,
    backed_up_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO bak_gallery_pic_thumb_20261119 (pic_id, pic_url, pic_thumb)
SELECT pic_id, pic_url, pic_thumb
FROM gallery_pic
WHERE pic_url IS NOT NULL AND btrim(pic_url) <> ''
  AND (pic_thumb IS NULL OR btrim(pic_thumb) = '' OR pic_thumb = pic_url)
  AND coalesce(pic_thumb, '') NOT LIKE '%x-oss-process=%'
ON CONFLICT (pic_id) DO NOTHING;

-- 2) md5 守卫：备份集必须与探针一致
DO $$
DECLARE
    expected text := '__EXPECTED_MD5__';
    cur text;
    n int;
BEGIN
    SELECT md5(coalesce(string_agg(pic_id::text || ':' || pic_url || ':' || coalesce(pic_thumb, ''), ',' ORDER BY pic_id), '')),
           count(*)
      INTO cur, n
      FROM bak_gallery_pic_thumb_20261119;
    IF position('EXPECTED_MD5' in expected) > 0 THEN
        RAISE EXCEPTION 'md5 守卫未填：先跑 probe 脚本并替换 __EXPECTED_MD5__ 后再执行';
    END IF;
    IF cur <> expected THEN
        RAISE EXCEPTION 'md5 守卫不符：探针 % ≠ 当前备份集 %（行数 %），中止', expected, cur, n;
    END IF;
END $$;

-- 3) 归一（仅备份集行）
UPDATE gallery_pic g
SET pic_thumb = CASE
        WHEN g.pic_url LIKE '%?%'
            THEN g.pic_url || '&x-oss-process=image/resize,m_pad,h_200,w_200'
        ELSE g.pic_url || '?x-oss-process=image/resize,m_pad,h_200,w_200'
    END
FROM bak_gallery_pic_thumb_20261119 b
WHERE g.pic_id = b.pic_id;

-- 4) 归一后校验：备份集内不得再有裸 thumb / 无参 thumb
DO $$
DECLARE n int;
BEGIN
    SELECT count(*) INTO n
    FROM gallery_pic g
    JOIN bak_gallery_pic_thumb_20261119 b ON b.pic_id = g.pic_id
    WHERE g.pic_thumb IS NULL OR btrim(g.pic_thumb) = ''
       OR g.pic_thumb = g.pic_url
       OR g.pic_thumb NOT LIKE '%x-oss-process=%';
    IF n > 0 THEN
        RAISE EXCEPTION '归一后仍有 % 行不符合口径，中止回滚', n;
    END IF;
END $$;

COMMIT;

-- 5) 执行结果留档（行数 + 归一后 md5，记入部署报告）
SELECT count(*) AS normalized_count,
       md5(coalesce(string_agg(g.pic_id::text || ':' || g.pic_thumb, ',' ORDER BY g.pic_id), '')) AS normalized_md5
FROM gallery_pic g
JOIN bak_gallery_pic_thumb_20261119 b ON b.pic_id = g.pic_id;
