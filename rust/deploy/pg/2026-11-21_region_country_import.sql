-- 2026-11-21: 跨境国家/地区全量导入（一）国家层 —— Tigshop 论坛帖 407879（我方新增）。
--
-- 内容：新增 243 个国家行（level=1，parent_id=0），region_name 为中文国名。
-- 数据来源：国家骨干 dr5hn Countries States Cities Database v5.2.0-export.1
--   （ODbL-1.0，https://github.com/dr5hn/countries-states-cities-database）；
--   中文国名 Unicode CLDR zh territories（Unicode License），敏感项按 GB/T 2659 校对
--   （PS 用短名「巴勒斯坦」；HK/MO/TW 已在我方中国树内、XK 未收录 GB/T 2659，均不在国家层导入）。
-- 既有 CN/US/CA 三行不导入、不覆盖（名称/排序保持原种子口径）。
-- 排序：按中文国名拼音序写入，sort_order 自 100 起；getCountryList 按 region_id 升序，
--   故前端国家选择器即按此序展示。first_word = 国名拼音首字母（与存量口径一致，如 中国=Z、美国=M）。
--
-- 幂等：国家按 standard_code（ISO alpha-2）判重，可重复执行。
-- 执行顺序：本文件为第 2 步 —— 先执行 2026-11-21_region_level_fix.sql，
--   再执行本文件，最后按序执行 region_state_import_1..N.sql。
BEGIN;

INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '阿尔巴尼亚', 0, 0, 100, 'A', 'AL'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AL');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '阿尔及利亚', 0, 0, 101, 'A', 'DZ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'DZ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '阿富汗', 0, 0, 102, 'A', 'AF'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AF');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '阿根廷', 0, 0, 103, 'A', 'AR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '爱尔兰', 0, 0, 104, 'A', 'IE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'IE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '埃及', 0, 0, 105, 'A', 'EG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'EG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '埃塞俄比亚', 0, 0, 106, 'A', 'ET'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'ET');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '爱沙尼亚', 0, 0, 107, 'A', 'EE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'EE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '阿拉伯联合酋长国', 0, 0, 108, 'A', 'AE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '阿鲁巴', 0, 0, 109, 'A', 'AW'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AW');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '阿曼', 0, 0, 110, 'A', 'OM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'OM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '安道尔', 0, 0, 111, 'A', 'AD'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AD');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '安哥拉', 0, 0, 112, 'A', 'AO'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AO');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '安圭拉', 0, 0, 113, 'A', 'AI'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AI');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '安提瓜和巴布达', 0, 0, 114, 'A', 'AG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '澳大利亚', 0, 0, 115, 'A', 'AU'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AU');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '奥地利', 0, 0, 116, 'A', 'AT'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AT');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '奥兰群岛', 0, 0, 117, 'A', 'AX'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AX');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '阿塞拜疆', 0, 0, 118, 'A', 'AZ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AZ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '巴巴多斯', 0, 0, 119, 'B', 'BB'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BB');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '巴布亚新几内亚', 0, 0, 120, 'B', 'PG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '巴哈马', 0, 0, 121, 'B', 'BS'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BS');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '白俄罗斯', 0, 0, 122, 'B', 'BY'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BY');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '百慕大', 0, 0, 123, 'B', 'BM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '巴基斯坦', 0, 0, 124, 'B', 'PK'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PK');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '巴拉圭', 0, 0, 125, 'B', 'PY'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PY');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '巴勒斯坦', 0, 0, 126, 'B', 'PS'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PS');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '巴林', 0, 0, 127, 'B', 'BH'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BH');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '巴拿马', 0, 0, 128, 'B', 'PA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PA');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '保加利亚', 0, 0, 129, 'B', 'BG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '巴西', 0, 0, 130, 'B', 'BR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '北马里亚纳群岛', 0, 0, 131, 'B', 'MP'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MP');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '北马其顿', 0, 0, 132, 'B', 'MK'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MK');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '贝宁', 0, 0, 133, 'B', 'BJ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BJ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '比利时', 0, 0, 134, 'B', 'BE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '秘鲁', 0, 0, 135, 'B', 'PE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '冰岛', 0, 0, 136, 'B', 'IS'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'IS');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '博茨瓦纳', 0, 0, 137, 'B', 'BW'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BW');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '波多黎各', 0, 0, 138, 'B', 'PR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '波兰', 0, 0, 139, 'B', 'PL'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PL');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '玻利维亚', 0, 0, 140, 'B', 'BO'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BO');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '伯利兹', 0, 0, 141, 'B', 'BZ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BZ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '波斯尼亚和黑塞哥维那', 0, 0, 142, 'B', 'BA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BA');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '不丹', 0, 0, 143, 'B', 'BT'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BT');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '布基纳法索', 0, 0, 144, 'B', 'BF'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BF');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '布隆迪', 0, 0, 145, 'B', 'BI'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BI');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '布韦岛', 0, 0, 146, 'B', 'BV'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BV');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '朝鲜', 0, 0, 147, 'C', 'KP'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'KP');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '赤道几内亚', 0, 0, 148, 'C', 'GQ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GQ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '丹麦', 0, 0, 149, 'D', 'DK'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'DK');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '德国', 0, 0, 150, 'D', 'DE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'DE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '东帝汶', 0, 0, 151, 'D', 'TL'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TL');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '多哥', 0, 0, 152, 'D', 'TG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '多米尼加共和国', 0, 0, 153, 'D', 'DO'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'DO');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '多米尼克', 0, 0, 154, 'D', 'DM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'DM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '厄瓜多尔', 0, 0, 155, 'E', 'EC'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'EC');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '厄立特里亚', 0, 0, 156, 'E', 'ER'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'ER');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '俄罗斯', 0, 0, 157, 'E', 'RU'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'RU');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '法国', 0, 0, 158, 'F', 'FR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'FR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '法罗群岛', 0, 0, 159, 'F', 'FO'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'FO');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '梵蒂冈', 0, 0, 160, 'F', 'VA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'VA');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '法属波利尼西亚', 0, 0, 161, 'F', 'PF'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PF');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '法属圭亚那', 0, 0, 162, 'F', 'GF'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GF');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '法属南部领地', 0, 0, 163, 'F', 'TF'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TF');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '法属圣马丁', 0, 0, 164, 'F', 'MF'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MF');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '斐济', 0, 0, 165, 'F', 'FJ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'FJ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '菲律宾', 0, 0, 166, 'F', 'PH'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PH');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '芬兰', 0, 0, 167, 'F', 'FI'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'FI');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '佛得角', 0, 0, 168, 'F', 'CV'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CV');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '福克兰群岛', 0, 0, 169, 'F', 'FK'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'FK');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '冈比亚', 0, 0, 170, 'G', 'GM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '刚果（布）', 0, 0, 171, 'G', 'CG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '刚果（金）', 0, 0, 172, 'G', 'CD'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CD');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '格陵兰', 0, 0, 173, 'G', 'GL'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GL');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '格林纳达', 0, 0, 174, 'G', 'GD'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GD');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '格鲁吉亚', 0, 0, 175, 'G', 'GE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '哥伦比亚', 0, 0, 176, 'G', 'CO'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CO');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '根西岛', 0, 0, 177, 'G', 'GG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '哥斯达黎加', 0, 0, 178, 'G', 'CR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '瓜德罗普', 0, 0, 179, 'G', 'GP'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GP');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '关岛', 0, 0, 180, 'G', 'GU'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GU');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '古巴', 0, 0, 181, 'G', 'CU'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CU');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '圭亚那', 0, 0, 182, 'G', 'GY'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GY');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '海地', 0, 0, 183, 'H', 'HT'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'HT');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '韩国', 0, 0, 184, 'H', 'KR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'KR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '哈萨克斯坦', 0, 0, 185, 'H', 'KZ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'KZ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '赫德岛和麦克唐纳群岛', 0, 0, 186, 'H', 'HM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'HM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '黑山', 0, 0, 187, 'H', 'ME'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'ME');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '荷兰', 0, 0, 188, 'H', 'NL'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NL');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '荷属加勒比区', 0, 0, 189, 'H', 'BQ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BQ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '荷属圣马丁', 0, 0, 190, 'H', 'SX'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SX');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '洪都拉斯', 0, 0, 191, 'H', 'HN'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'HN');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '加纳', 0, 0, 192, 'J', 'GH'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GH');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '柬埔寨', 0, 0, 193, 'J', 'KH'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'KH');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '加蓬', 0, 0, 194, 'J', 'GA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GA');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '吉布提', 0, 0, 195, 'J', 'DJ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'DJ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '捷克', 0, 0, 196, 'J', 'CZ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CZ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '吉尔吉斯斯坦', 0, 0, 197, 'J', 'KG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'KG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '基里巴斯', 0, 0, 198, 'J', 'KI'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'KI');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '津巴布韦', 0, 0, 199, 'J', 'ZW'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'ZW');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '几内亚', 0, 0, 200, 'J', 'GN'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GN');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '几内亚比绍', 0, 0, 201, 'J', 'GW'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GW');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '开曼群岛', 0, 0, 202, 'K', 'KY'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'KY');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '喀麦隆', 0, 0, 203, 'K', 'CM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '卡塔尔', 0, 0, 204, 'K', 'QA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'QA');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '科科斯（基林）群岛', 0, 0, 205, 'K', 'CC'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CC');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '克罗地亚', 0, 0, 206, 'K', 'HR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'HR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '科摩罗', 0, 0, 207, 'K', 'KM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'KM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '肯尼亚', 0, 0, 208, 'K', 'KE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'KE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '科特迪瓦', 0, 0, 209, 'K', 'CI'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CI');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '科威特', 0, 0, 210, 'K', 'KW'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'KW');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '库克群岛', 0, 0, 211, 'K', 'CK'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CK');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '库拉索', 0, 0, 212, 'K', 'CW'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CW');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '莱索托', 0, 0, 213, 'L', 'LS'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'LS');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '老挝', 0, 0, 214, 'L', 'LA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'LA');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '拉脱维亚', 0, 0, 215, 'L', 'LV'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'LV');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '黎巴嫩', 0, 0, 216, 'L', 'LB'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'LB');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '利比里亚', 0, 0, 217, 'L', 'LR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'LR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '利比亚', 0, 0, 218, 'L', 'LY'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'LY');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '列支敦士登', 0, 0, 219, 'L', 'LI'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'LI');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '立陶宛', 0, 0, 220, 'L', 'LT'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'LT');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '留尼汪', 0, 0, 221, 'L', 'RE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'RE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '罗马尼亚', 0, 0, 222, 'L', 'RO'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'RO');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '卢森堡', 0, 0, 223, 'L', 'LU'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'LU');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '卢旺达', 0, 0, 224, 'L', 'RW'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'RW');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '马达加斯加', 0, 0, 225, 'M', 'MG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '马恩岛', 0, 0, 226, 'M', 'IM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'IM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '马尔代夫', 0, 0, 227, 'M', 'MV'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MV');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '马耳他', 0, 0, 228, 'M', 'MT'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MT');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '马来西亚', 0, 0, 229, 'M', 'MY'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MY');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '马拉维', 0, 0, 230, 'M', 'MW'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MW');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '马里', 0, 0, 231, 'M', 'ML'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'ML');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '毛里求斯', 0, 0, 232, 'M', 'MU'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MU');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '毛里塔尼亚', 0, 0, 233, 'M', 'MR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '马绍尔群岛', 0, 0, 234, 'M', 'MH'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MH');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '马提尼克', 0, 0, 235, 'M', 'MQ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MQ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '马约特', 0, 0, 236, 'M', 'YT'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'YT');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '美国本土外小岛屿', 0, 0, 237, 'M', 'UM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'UM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '美属萨摩亚', 0, 0, 238, 'M', 'AS'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AS');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '美属维尔京群岛', 0, 0, 239, 'M', 'VI'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'VI');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '蒙古', 0, 0, 240, 'M', 'MN'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MN');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '孟加拉国', 0, 0, 241, 'M', 'BD'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BD');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '蒙特塞拉特', 0, 0, 242, 'M', 'MS'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MS');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '缅甸', 0, 0, 243, 'M', 'MM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '密克罗尼西亚', 0, 0, 244, 'M', 'FM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'FM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '摩尔多瓦', 0, 0, 245, 'M', 'MD'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MD');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '摩洛哥', 0, 0, 246, 'M', 'MA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MA');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '摩纳哥', 0, 0, 247, 'M', 'MC'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MC');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '莫桑比克', 0, 0, 248, 'M', 'MZ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MZ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '墨西哥', 0, 0, 249, 'M', 'MX'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'MX');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '纳米比亚', 0, 0, 250, 'N', 'NA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NA');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '南非', 0, 0, 251, 'N', 'ZA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'ZA');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '南极洲', 0, 0, 252, 'N', 'AQ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AQ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '南乔治亚和南桑威奇群岛', 0, 0, 253, 'N', 'GS'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GS');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '南苏丹', 0, 0, 254, 'N', 'SS'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SS');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '瑙鲁', 0, 0, 255, 'N', 'NR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '尼加拉瓜', 0, 0, 256, 'N', 'NI'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NI');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '尼泊尔', 0, 0, 257, 'N', 'NP'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NP');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '尼日尔', 0, 0, 258, 'N', 'NE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '尼日利亚', 0, 0, 259, 'N', 'NG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '纽埃', 0, 0, 260, 'N', 'NU'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NU');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '诺福克岛', 0, 0, 261, 'N', 'NF'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NF');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '挪威', 0, 0, 262, 'N', 'NO'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NO');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '帕劳', 0, 0, 263, 'P', 'PW'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PW');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '皮特凯恩群岛', 0, 0, 264, 'P', 'PN'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PN');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '葡萄牙', 0, 0, 265, 'P', 'PT'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PT');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '日本', 0, 0, 266, 'R', 'JP'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'JP');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '瑞典', 0, 0, 267, 'R', 'SE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '瑞士', 0, 0, 268, 'R', 'CH'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CH');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '萨尔瓦多', 0, 0, 269, 'S', 'SV'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SV');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '塞尔维亚', 0, 0, 270, 'S', 'RS'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'RS');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '塞拉利昂', 0, 0, 271, 'S', 'SL'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SL');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '塞内加尔', 0, 0, 272, 'S', 'SN'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SN');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '塞浦路斯', 0, 0, 273, 'S', 'CY'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CY');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '塞舌尔', 0, 0, 274, 'S', 'SC'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SC');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '萨摩亚', 0, 0, 275, 'S', 'WS'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'WS');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '沙特阿拉伯', 0, 0, 276, 'S', 'SA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SA');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '圣巴泰勒米', 0, 0, 277, 'S', 'BL'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BL');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '圣诞岛', 0, 0, 278, 'S', 'CX'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CX');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '圣多美和普林西比', 0, 0, 279, 'S', 'ST'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'ST');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '圣赫勒拿', 0, 0, 280, 'S', 'SH'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SH');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '圣基茨和尼维斯', 0, 0, 281, 'S', 'KN'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'KN');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '圣卢西亚', 0, 0, 282, 'S', 'LC'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'LC');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '圣马力诺', 0, 0, 283, 'S', 'SM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '圣皮埃尔和密克隆群岛', 0, 0, 284, 'S', 'PM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'PM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '圣文森特和格林纳丁斯', 0, 0, 285, 'S', 'VC'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'VC');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '斯里兰卡', 0, 0, 286, 'S', 'LK'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'LK');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '斯洛伐克', 0, 0, 287, 'S', 'SK'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SK');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '斯洛文尼亚', 0, 0, 288, 'S', 'SI'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SI');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '斯瓦尔巴和扬马延', 0, 0, 289, 'S', 'SJ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SJ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '斯威士兰', 0, 0, 290, 'S', 'SZ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SZ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '苏丹', 0, 0, 291, 'S', 'SD'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SD');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '苏里南', 0, 0, 292, 'S', 'SR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '所罗门群岛', 0, 0, 293, 'S', 'SB'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SB');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '索马里', 0, 0, 294, 'S', 'SO'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SO');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '泰国', 0, 0, 295, 'T', 'TH'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TH');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '塔吉克斯坦', 0, 0, 296, 'T', 'TJ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TJ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '汤加', 0, 0, 297, 'T', 'TO'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TO');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '坦桑尼亚', 0, 0, 298, 'T', 'TZ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TZ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '特克斯和凯科斯群岛', 0, 0, 299, 'T', 'TC'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TC');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '特立尼达和多巴哥', 0, 0, 300, 'T', 'TT'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TT');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '土耳其', 0, 0, 301, 'T', 'TR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '土库曼斯坦', 0, 0, 302, 'T', 'TM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '突尼斯', 0, 0, 303, 'T', 'TN'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TN');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '托克劳', 0, 0, 304, 'T', 'TK'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TK');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '图瓦卢', 0, 0, 305, 'T', 'TV'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TV');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '瓦利斯和富图纳', 0, 0, 306, 'W', 'WF'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'WF');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '瓦努阿图', 0, 0, 307, 'W', 'VU'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'VU');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '危地马拉', 0, 0, 308, 'W', 'GT'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GT');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '委内瑞拉', 0, 0, 309, 'W', 'VE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'VE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '文莱', 0, 0, 310, 'W', 'BN'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'BN');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '乌干达', 0, 0, 311, 'W', 'UG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'UG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '乌克兰', 0, 0, 312, 'W', 'UA'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'UA');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '乌拉圭', 0, 0, 313, 'W', 'UY'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'UY');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '乌兹别克斯坦', 0, 0, 314, 'W', 'UZ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'UZ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '西班牙', 0, 0, 315, 'X', 'ES'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'ES');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '希腊', 0, 0, 316, 'X', 'GR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '新加坡', 0, 0, 317, 'X', 'SG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '新喀里多尼亚', 0, 0, 318, 'X', 'NC'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NC');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '新西兰', 0, 0, 319, 'X', 'NZ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'NZ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '匈牙利', 0, 0, 320, 'X', 'HU'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'HU');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '西撒哈拉', 0, 0, 321, 'X', 'EH'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'EH');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '叙利亚', 0, 0, 322, 'X', 'SY'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'SY');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '牙买加', 0, 0, 323, 'Y', 'JM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'JM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '亚美尼亚', 0, 0, 324, 'Y', 'AM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'AM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '也门', 0, 0, 325, 'Y', 'YE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'YE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '意大利', 0, 0, 326, 'Y', 'IT'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'IT');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '伊拉克', 0, 0, 327, 'Y', 'IQ'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'IQ');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '伊朗', 0, 0, 328, 'Y', 'IR'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'IR');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '印度', 0, 0, 329, 'Y', 'IN'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'IN');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '印度尼西亚', 0, 0, 330, 'Y', 'ID'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'ID');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '英国', 0, 0, 331, 'Y', 'GB'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GB');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '英属维尔京群岛', 0, 0, 332, 'Y', 'VG'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'VG');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '英属印度洋领地', 0, 0, 333, 'Y', 'IO'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'IO');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '以色列', 0, 0, 334, 'Y', 'IL'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'IL');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '约旦', 0, 0, 335, 'Y', 'JO'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'JO');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '越南', 0, 0, 336, 'Y', 'VN'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'VN');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '赞比亚', 0, 0, 337, 'Z', 'ZM'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'ZM');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '泽西岛', 0, 0, 338, 'Z', 'JE'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'JE');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '乍得', 0, 0, 339, 'Z', 'TD'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'TD');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '直布罗陀', 0, 0, 340, 'Z', 'GI'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'GI');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '智利', 0, 0, 341, 'Z', 'CL'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CL');
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 1, '中非共和国', 0, 0, 342, 'Z', 'CF'
WHERE NOT EXISTS (SELECT 1 FROM region WHERE standard_code = 'CF');

COMMIT;
