-- 2026-11-18: 跨境收货地址二期（Tigshop v5.8.62）— 美国 / 加拿大地区种子。
--
-- 内容：国家 2 行（level=1，standard_code = US / CA）
--      + 美国 50 州及哥伦比亚特区（level=2，standard_code = ISO 3166-2 代码，如 US-CA）
--      + 加拿大 10 省及 3 地区（level=2，standard_code = ISO 3166-2 代码，如 CA-ON）。
-- 数据来源：ISO 3166-2:US / ISO 3166-2:CA 官方 subdivision 清单（名称为官方英文名）。
-- 完整度：一级行政区全量；城市及以下不建种子行 —— 前端动态地区表单在
-- 无子级时自动退化为文本输入（与现有 8 国 text 退化机制一致），可后续按需补。
--
-- 幂等：国家按 standard_code 判重，州/省按 (parent_id, standard_code) 判重，
-- 可重复执行；不写死 region_id（各环境自增序列不同，父级一律按 standard_code 反查）。
BEGIN;

-- ── 国家 ──
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '美国', 0, 0, 90, 'M', 'US'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'US');

INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '加拿大', 0, 0, 91, 'J', 'CA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CA');

-- ── 美国：50 州 + 哥伦比亚特区 ──
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, us.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region us,
(VALUES
    ('Alabama', 'US-AL', 1), ('Alaska', 'US-AK', 2), ('Arizona', 'US-AZ', 3),
    ('Arkansas', 'US-AR', 4), ('California', 'US-CA', 5), ('Colorado', 'US-CO', 6),
    ('Connecticut', 'US-CT', 7), ('Delaware', 'US-DE', 8), ('District of Columbia', 'US-DC', 9),
    ('Florida', 'US-FL', 10), ('Georgia', 'US-GA', 11), ('Hawaii', 'US-HI', 12),
    ('Idaho', 'US-ID', 13), ('Illinois', 'US-IL', 14), ('Indiana', 'US-IN', 15),
    ('Iowa', 'US-IA', 16), ('Kansas', 'US-KS', 17), ('Kentucky', 'US-KY', 18),
    ('Louisiana', 'US-LA', 19), ('Maine', 'US-ME', 20), ('Maryland', 'US-MD', 21),
    ('Massachusetts', 'US-MA', 22), ('Michigan', 'US-MI', 23), ('Minnesota', 'US-MN', 24),
    ('Mississippi', 'US-MS', 25), ('Missouri', 'US-MO', 26), ('Montana', 'US-MT', 27),
    ('Nebraska', 'US-NE', 28), ('Nevada', 'US-NV', 29), ('New Hampshire', 'US-NH', 30),
    ('New Jersey', 'US-NJ', 31), ('New Mexico', 'US-NM', 32), ('New York', 'US-NY', 33),
    ('North Carolina', 'US-NC', 34), ('North Dakota', 'US-ND', 35), ('Ohio', 'US-OH', 36),
    ('Oklahoma', 'US-OK', 37), ('Oregon', 'US-OR', 38), ('Pennsylvania', 'US-PA', 39),
    ('Rhode Island', 'US-RI', 40), ('South Carolina', 'US-SC', 41), ('South Dakota', 'US-SD', 42),
    ('Tennessee', 'US-TN', 43), ('Texas', 'US-TX', 44), ('Utah', 'US-UT', 45),
    ('Vermont', 'US-VT', 46), ('Virginia', 'US-VA', 47), ('Washington', 'US-WA', 48),
    ('West Virginia', 'US-WV', 49), ('Wisconsin', 'US-WI', 50), ('Wyoming', 'US-WY', 51)
) AS v(name, code, ord)
WHERE us.standard_code = 'US'
  AND NOT EXISTS (
      SELECT 1 FROM region c WHERE c.parent_id = us.region_id AND c.standard_code = v.code
  );

-- ── 加拿大：10 省 + 3 地区 ──
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, ca.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region ca,
(VALUES
    ('Alberta', 'CA-AB', 1), ('British Columbia', 'CA-BC', 2), ('Manitoba', 'CA-MB', 3),
    ('New Brunswick', 'CA-NB', 4), ('Newfoundland and Labrador', 'CA-NL', 5),
    ('Nova Scotia', 'CA-NS', 6), ('Ontario', 'CA-ON', 7), ('Prince Edward Island', 'CA-PE', 8),
    ('Quebec', 'CA-QC', 9), ('Saskatchewan', 'CA-SK', 10), ('Northwest Territories', 'CA-NT', 11),
    ('Nunavut', 'CA-NU', 12), ('Yukon', 'CA-YT', 13)
) AS v(name, code, ord)
WHERE ca.standard_code = 'CA'
  AND NOT EXISTS (
      SELECT 1 FROM region c WHERE c.parent_id = ca.region_id AND c.standard_code = v.code
  );

COMMIT;
