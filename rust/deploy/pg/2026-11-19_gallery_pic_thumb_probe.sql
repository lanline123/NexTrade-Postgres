-- gallery_pic.pic_thumb 存量探针（只读，不写库）
--
-- 背景：图库上传写入侧曾把 pic_thumb 写成与 pic_url 逐字相同的裸值（无 OSS
-- 处理参数），存量普查为 68 条（2026-11-19 H5 v22 数据治理轮计数）。
-- 本探针输出归一目标集的行数与 md5，执行归一脚本前先跑本探针并记录结果，
-- 再把 bare_md5 填入归一脚本的守卫占位符。
--
-- 目标集（裸 thumb）定义：pic_url 非空，且 pic_thumb 为空或与 pic_url 逐字
-- 相同，且 pic_thumb 本身不带 x-oss-process 参数。
-- 异图裸 thumb（pic_thumb 非空、与 pic_url 不同图、无参数）不在归一范围，
-- 仅在第 3 段列清单人工核对。

-- 1) 目标集行数 + md5（填入归一脚本守卫）
SELECT count(*) AS bare_count,
       md5(coalesce(string_agg(pic_id::text || ':' || pic_url || ':' || coalesce(pic_thumb, ''), ',' ORDER BY pic_id), '')) AS bare_md5,
       max(length(pic_url)) AS max_pic_url_len
FROM gallery_pic
WHERE pic_url IS NOT NULL AND btrim(pic_url) <> ''
  AND (pic_thumb IS NULL OR btrim(pic_thumb) = '' OR pic_thumb = pic_url)
  AND coalesce(pic_thumb, '') NOT LIKE '%x-oss-process=%';

-- 2) 目标集明细（留档；执行时与归一后状态对照）
SELECT pic_id, pic_url, pic_thumb
FROM gallery_pic
WHERE pic_url IS NOT NULL AND btrim(pic_url) <> ''
  AND (pic_thumb IS NULL OR btrim(pic_thumb) = '' OR pic_thumb = pic_url)
  AND coalesce(pic_thumb, '') NOT LIKE '%x-oss-process=%'
ORDER BY pic_id;

-- 3) 异图裸 thumb 清单（不归一，人工核对用）
SELECT pic_id, pic_url, pic_thumb
FROM gallery_pic
WHERE pic_thumb IS NOT NULL AND btrim(pic_thumb) <> ''
  AND pic_thumb <> pic_url
  AND pic_thumb NOT LIKE '%x-oss-process=%'
ORDER BY pic_id;
