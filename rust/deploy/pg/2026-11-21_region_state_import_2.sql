-- 2026-11-21: 跨境国家/地区全量导入（二）州/省层 第 2 部分 —— Tigshop 论坛帖 407879（我方新增）。
--
-- 内容：新增国家的一级行政区（level=2，英文官方名，与既有美/加种子口径一致），
--   城市层不导入（前端无子级时退化文本输入）。
-- 数据来源：dr5hn Countries States Cities Database v5.2.0-export.1（ODbL-1.0，须署名）；
--   standard_code = ISO 3166-2 代码；源数据无 ISO 代码的少数州省以「国家码-源内代码」兜底
--   （如 ET-55），保证 (parent_id, standard_code) 判重幂等且不与真实 ISO 代码冲突。
-- 既有 CN/US/CA 的下级已由原版数据与 2026-11-18 种子定型，本批不触碰。
--
-- 幂等：州/省按 (parent_id, standard_code) 判重，父级按 standard_code 反查，不写死 region_id。
-- 执行顺序：本文件为第 3 步 —— 须在 region_level_fix.sql 与 region_country_import.sql 之后，
--   各部分之间顺序任意（均按国家代码反查父级）。
BEGIN;

-- 马耳他（MT）：74 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Attard', 'MT-01', 1),
    ('Balzan', 'MT-02', 2),
    ('Birgu', 'MT-03', 3),
    ('Birkirkara', 'MT-04', 4),
    ('Birżebbuġa', 'MT-05', 5),
    ('Cospicua', 'MT-06', 6),
    ('Dingli', 'MT-07', 7),
    ('Eastern Region', 'MT-R02', 8),
    ('Fgura', 'MT-08', 9),
    ('Floriana', 'MT-09', 10),
    ('Fontana', 'MT-10', 11),
    ('Għajnsielem', 'MT-13', 12),
    ('Għarb', 'MT-14', 13),
    ('Għargħur', 'MT-15', 14),
    ('Għasri', 'MT-16', 15),
    ('Għaxaq', 'MT-17', 16),
    ('Gozo Region', 'MT-R06', 17),
    ('Gudja', 'MT-11', 18),
    ('Gżira', 'MT-12', 19),
    ('Ħamrun', 'MT-18', 20),
    ('Iklin', 'MT-19', 21),
    ('Kalkara', 'MT-21', 22),
    ('Kerċem', 'MT-22', 23),
    ('Kirkop', 'MT-23', 24),
    ('Lija', 'MT-24', 25),
    ('Luqa', 'MT-25', 26),
    ('Marsa', 'MT-26', 27),
    ('Marsaskala', 'MT-27', 28),
    ('Marsaxlokk', 'MT-28', 29),
    ('Mdina', 'MT-29', 30),
    ('Mellieħa', 'MT-30', 31),
    ('Mġarr', 'MT-31', 32),
    ('Mosta', 'MT-32', 33),
    ('Mqabba', 'MT-33', 34),
    ('Msida', 'MT-34', 35),
    ('Mtarfa', 'MT-35', 36),
    ('Munxar', 'MT-36', 37),
    ('Nadur', 'MT-37', 38),
    ('Naxxar', 'MT-38', 39),
    ('Northern Region', 'MT-R01', 40),
    ('Paola', 'MT-39', 41),
    ('Pembroke', 'MT-40', 42),
    ('Pietà', 'MT-41', 43),
    ('Port Region', 'MT-R04', 44),
    ('Qala', 'MT-42', 45),
    ('Qormi', 'MT-43', 46),
    ('Qrendi', 'MT-44', 47),
    ('Rabat', 'MT-46', 48),
    ('Safi', 'MT-47', 49),
    ('San Ġwann', 'MT-49', 50),
    ('San Lawrenz', 'MT-50', 51),
    ('Sannat', 'MT-52', 52),
    ('Santa Luċija', 'MT-53', 53),
    ('Santa Venera', 'MT-54', 54),
    ('Senglea', 'MT-20', 55),
    ('Siġġiewi', 'MT-55', 56),
    ('Sliema', 'MT-56', 57),
    ('Southern Region', 'MT-R05', 58),
    ('St. Julian''s', 'MT-48', 59),
    ('St. Paul''s Bay', 'MT-51', 60),
    ('Swieqi', 'MT-57', 61),
    ('Ta'' Xbiex', 'MT-58', 62),
    ('Tarxien', 'MT-59', 63),
    ('Valletta', 'MT-60', 64),
    ('Victoria', 'MT-45', 65),
    ('Western Region', 'MT-R03', 66),
    ('Xagħra', 'MT-61', 67),
    ('Xewkija', 'MT-62', 68),
    ('Xgħajra', 'MT-63', 69),
    ('Żabbar', 'MT-64', 70),
    ('Żebbuġ Gozo', 'MT-65', 71),
    ('Żebbuġ Malta', 'MT-66', 72),
    ('Żejtun', 'MT-67', 73),
    ('Żurrieq', 'MT-68', 74)
) AS v(name, code, ord)
WHERE p.standard_code = 'MT' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 马来西亚（MY）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Johor', 'MY-01', 1),
    ('Kedah', 'MY-02', 2),
    ('Kelantan', 'MY-03', 3),
    ('Kuala Lumpur', 'MY-14', 4),
    ('Labuan', 'MY-15', 5),
    ('Malacca', 'MY-04', 6),
    ('Negeri Sembilan', 'MY-05', 7),
    ('Pahang', 'MY-06', 8),
    ('Penang', 'MY-07', 9),
    ('Perak', 'MY-08', 10),
    ('Perlis', 'MY-09', 11),
    ('Putrajaya', 'MY-16', 12),
    ('Sabah', 'MY-12', 13),
    ('Sarawak', 'MY-13', 14),
    ('Selangor', 'MY-10', 15),
    ('Terengganu', 'MY-11', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'MY' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 马拉维（MW）：31 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Balaka', 'MW-BA', 1),
    ('Blantyre', 'MW-BL', 2),
    ('Central', 'MW-C', 3),
    ('Chikwawa', 'MW-CK', 4),
    ('Chiradzulu', 'MW-CR', 5),
    ('Chitipa', 'MW-CT', 6),
    ('Dedza', 'MW-DE', 7),
    ('Dowa', 'MW-DO', 8),
    ('Karonga', 'MW-KR', 9),
    ('Kasungu', 'MW-KS', 10),
    ('Likoma', 'MW-LK', 11),
    ('Lilongwe', 'MW-LI', 12),
    ('Machinga', 'MW-MH', 13),
    ('Mangochi', 'MW-MG', 14),
    ('Mchinji', 'MW-MC', 15),
    ('Mulanje', 'MW-MU', 16),
    ('Mwanza', 'MW-MW', 17),
    ('Mzimba', 'MW-MZ', 18),
    ('Neno', 'MW-NE', 19),
    ('Nkhata Bay', 'MW-NB', 20),
    ('Nkhotakota', 'MW-NK', 21),
    ('Northern', 'MW-N', 22),
    ('Nsanje', 'MW-NS', 23),
    ('Ntcheu', 'MW-NU', 24),
    ('Ntchisi', 'MW-NI', 25),
    ('Phalombe', 'MW-PH', 26),
    ('Rumphi', 'MW-RU', 27),
    ('Salima', 'MW-SA', 28),
    ('Southern', 'MW-S', 29),
    ('Thyolo', 'MW-TH', 30),
    ('Zomba', 'MW-ZO', 31)
) AS v(name, code, ord)
WHERE p.standard_code = 'MW' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 马里（ML）：11 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bamako', 'ML-BKO', 1),
    ('Gao', 'ML-7', 2),
    ('Kayes', 'ML-1', 3),
    ('Kidal', 'ML-8', 4),
    ('Koulikoro', 'ML-2', 5),
    ('Ménaka', 'ML-9', 6),
    ('Mopti', 'ML-5', 7),
    ('Ségou', 'ML-4', 8),
    ('Sikasso', 'ML-3', 9),
    ('Taoudénit', 'ML-10', 10),
    ('Tombouctou', 'ML-6', 11)
) AS v(name, code, ord)
WHERE p.standard_code = 'ML' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 毛里求斯（MU）：12 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Agalega Islands', 'MU-AG', 1),
    ('Black River', 'MU-BL', 2),
    ('Flacq', 'MU-FL', 3),
    ('Grand Port', 'MU-GP', 4),
    ('Moka', 'MU-MO', 5),
    ('Pamplemousses', 'MU-PA', 6),
    ('Plaines Wilhems', 'MU-PW', 7),
    ('Port Louis', 'MU-PL', 8),
    ('Rivière du Rempart', 'MU-RR', 9),
    ('Rodrigues Island', 'MU-RO', 10),
    ('Saint Brandon Islands', 'MU-CC', 11),
    ('Savanne', 'MU-SA', 12)
) AS v(name, code, ord)
WHERE p.standard_code = 'MU' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 毛里塔尼亚（MR）：15 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Adrar', 'MR-07', 1),
    ('Assaba', 'MR-03', 2),
    ('Brakna', 'MR-05', 3),
    ('Dakhlet Nouadhibou', 'MR-08', 4),
    ('Gorgol', 'MR-04', 5),
    ('Guidimaka', 'MR-10', 6),
    ('Hodh Ech Chargui', 'MR-01', 7),
    ('Hodh El Gharbi', 'MR-02', 8),
    ('Inchiri', 'MR-12', 9),
    ('Nouakchott-Nord', 'MR-14', 10),
    ('Nouakchott-Ouest', 'MR-13', 11),
    ('Nouakchott-Sud', 'MR-15', 12),
    ('Tagant', 'MR-09', 13),
    ('Tiris Zemmour', 'MR-11', 14),
    ('Trarza', 'MR-06', 15)
) AS v(name, code, ord)
WHERE p.standard_code = 'MR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 马绍尔群岛（MH）：26 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ailinglaplap', 'MH-ALL', 1),
    ('Ailuk', 'MH-ALK', 2),
    ('Arno', 'MH-ARN', 3),
    ('Aur', 'MH-AUR', 4),
    ('Bikini & Kili', 'MH-KIL', 5),
    ('Ebon', 'MH-EBO', 6),
    ('Enewetak & Ujelang', 'MH-ENI', 7),
    ('Jabat', 'MH-JAB', 8),
    ('Jaluit', 'MH-JAL', 9),
    ('Kwajalein', 'MH-KWA', 10),
    ('Lae', 'MH-LAE', 11),
    ('Lib', 'MH-LIB', 12),
    ('Likiep', 'MH-LIK', 13),
    ('Majuro', 'MH-MAJ', 14),
    ('Maloelap', 'MH-MAL', 15),
    ('Mejit', 'MH-MEJ', 16),
    ('Mili', 'MH-MIL', 17),
    ('Namdrik', 'MH-NMK', 18),
    ('Namu', 'MH-NMU', 19),
    ('Ralik', 'MH-L', 20),
    ('Ratak', 'MH-T', 21),
    ('Rongelap', 'MH-RON', 22),
    ('Ujae', 'MH-UJA', 23),
    ('Utrik', 'MH-UTI', 24),
    ('Wotho', 'MH-WTH', 25),
    ('Wotje', 'MH-WTJ', 26)
) AS v(name, code, ord)
WHERE p.standard_code = 'MH' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 马提尼克（MQ）：4 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Fort-de-France', 'MQ-01', 1),
    ('La Trinité', 'MQ-02', 2),
    ('Le Marin', 'MQ-03', 3),
    ('Saint-Pierre', 'MQ-04', 4)
) AS v(name, code, ord)
WHERE p.standard_code = 'MQ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 马约特（YT）：17 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Acoua', 'YT-14', 1),
    ('Bandraboua', 'YT-16', 2),
    ('Bandrélé', 'YT-05', 3),
    ('Boueni', 'YT-07', 4),
    ('Chiconi', 'YT-11', 5),
    ('Chirongui', 'YT-08', 6),
    ('Dembeni', 'YT-04', 7),
    ('Dzaoudzi', 'YT-01', 8),
    ('Kani Keli', 'YT-06', 9),
    ('Koungou', 'YT-17', 10),
    ('M''Tsangamouji', 'YT-13', 11),
    ('Mamoudzou', 'YT-03', 12),
    ('Mtsamboro', 'YT-15', 13),
    ('Ouangani', 'YT-10', 14),
    ('Pamandzi', 'YT-02', 15),
    ('Sada', 'YT-09', 16),
    ('Tsingoni', 'YT-12', 17)
) AS v(name, code, ord)
WHERE p.standard_code = 'YT' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 美国本土外小岛屿（UM）：9 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Baker Island', 'UM-81', 1),
    ('Howland Island', 'UM-84', 2),
    ('Jarvis Island', 'UM-86', 3),
    ('Johnston Atoll', 'UM-67', 4),
    ('Kingman Reef', 'UM-89', 5),
    ('Midway Islands', 'UM-71', 6),
    ('Navassa Island', 'UM-76', 7),
    ('Palmyra Atoll', 'UM-95', 8),
    ('Wake Island', 'UM-79', 9)
) AS v(name, code, ord)
WHERE p.standard_code = 'UM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 美属萨摩亚（AS）：5 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Eastern', 'AS-02', 1),
    ('Manuʻa', 'AS-03', 2),
    ('Rose', 'AS-05', 3),
    ('Swains', 'AS-04', 4),
    ('Western', 'AS-01', 5)
) AS v(name, code, ord)
WHERE p.standard_code = 'AS' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 美属维尔京群岛（VI）：3 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Saint Croix', 'VI-SC', 1),
    ('Saint John', 'VI-SJ', 2),
    ('Saint Thomas', 'VI-ST', 3)
) AS v(name, code, ord)
WHERE p.standard_code = 'VI' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 蒙古（MN）：22 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Arkhangai', 'MN-073', 1),
    ('Bayan-Ölgii', 'MN-071', 2),
    ('Bayankhongor', 'MN-069', 3),
    ('Bulgan', 'MN-067', 4),
    ('Darkhan-Uul', 'MN-037', 5),
    ('Dornod', 'MN-061', 6),
    ('Dornogovi', 'MN-063', 7),
    ('Dundgovi', 'MN-059', 8),
    ('Govi-Altai', 'MN-065', 9),
    ('Govisümber', 'MN-064', 10),
    ('Khentii', 'MN-039', 11),
    ('Khovd', 'MN-043', 12),
    ('Khövsgöl', 'MN-041', 13),
    ('Ömnögovi', 'MN-053', 14),
    ('Orkhon', 'MN-035', 15),
    ('Övörkhangai', 'MN-055', 16),
    ('Selenge', 'MN-049', 17),
    ('Sükhbaatar', 'MN-051', 18),
    ('Töv', 'MN-047', 19),
    ('Ulaanbaatar', 'MN-1', 20),
    ('Uvs', 'MN-046', 21),
    ('Zavkhan', 'MN-057', 22)
) AS v(name, code, ord)
WHERE p.standard_code = 'MN' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 孟加拉国（BD）：72 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bagerhat', 'BD-05', 1),
    ('Bandarban', 'BD-01', 2),
    ('Barguna', 'BD-02', 3),
    ('Barisal', 'BD-A', 4),
    ('Barishal', 'BD-06', 5),
    ('Bhola', 'BD-07', 6),
    ('Bogura', 'BD-03', 7),
    ('Brahmanbaria', 'BD-04', 8),
    ('Chandpur', 'BD-09', 9),
    ('Chapai Nawabganj', 'BD-45', 10),
    ('Chattogram', 'BD-10', 11),
    ('Chittagong', 'BD-B', 12),
    ('Chuadanga', 'BD-12', 13),
    ('Cox''s Bazar', 'BD-11', 14),
    ('Cumilla', 'BD-08', 15),
    ('Dhaka', 'BD-13', 16),
    ('Dhaka', 'BD-C', 17),
    ('Dinajpur', 'BD-14', 18),
    ('Faridpur', 'BD-15', 19),
    ('Feni', 'BD-16', 20),
    ('Gaibandha', 'BD-19', 21),
    ('Gazipur', 'BD-18', 22),
    ('Gopalganj', 'BD-17', 23),
    ('Habiganj', 'BD-20', 24),
    ('Jamalpur', 'BD-21', 25),
    ('Jashore', 'BD-22', 26),
    ('Jhalakathi', 'BD-25', 27),
    ('Jhenaidah', 'BD-23', 28),
    ('Joypurhat', 'BD-24', 29),
    ('Khagrachhari', 'BD-29', 30),
    ('Khulna', 'BD-27', 31),
    ('Khulna', 'BD-D', 32),
    ('Kishoreganj', 'BD-26', 33),
    ('Kurigram', 'BD-28', 34),
    ('Kushtia', 'BD-30', 35),
    ('Lakshmipur', 'BD-31', 36),
    ('Lalmonirhat', 'BD-32', 37),
    ('Madaripur', 'BD-36', 38),
    ('Magura', 'BD-37', 39),
    ('Manikganj', 'BD-33', 40),
    ('Meherpur', 'BD-39', 41),
    ('Moulvibazar', 'BD-38', 42),
    ('Munshiganj', 'BD-35', 43),
    ('Mymensingh', 'BD-34', 44),
    ('Mymensingh', 'BD-H', 45),
    ('Naogaon', 'BD-48', 46),
    ('Narail', 'BD-43', 47),
    ('Narayanganj', 'BD-40', 48),
    ('Narsingdi', 'BD-42', 49),
    ('Natore', 'BD-44', 50),
    ('Netrakona', 'BD-41', 51),
    ('Nilphamari', 'BD-46', 52),
    ('Noakhali', 'BD-47', 53),
    ('Pabna', 'BD-49', 54),
    ('Panchagarh', 'BD-52', 55),
    ('Patuakhali', 'BD-51', 56),
    ('Pirojpur', 'BD-50', 57),
    ('Rajbari', 'BD-53', 58),
    ('Rajshahi', 'BD-54', 59),
    ('Rajshahi', 'BD-E', 60),
    ('Rangamati', 'BD-56', 61),
    ('Rangpur', 'BD-F', 62),
    ('Rangpur', 'BD-55', 63),
    ('Satkhira', 'BD-58', 64),
    ('Shariatpur', 'BD-62', 65),
    ('Sherpur', 'BD-57', 66),
    ('Sirajganj', 'BD-59', 67),
    ('Sunamganj', 'BD-61', 68),
    ('Sylhet', 'BD-G', 69),
    ('Sylhet', 'BD-60', 70),
    ('Tangail', 'BD-63', 71),
    ('Thakurgaon', 'BD-64', 72)
) AS v(name, code, ord)
WHERE p.standard_code = 'BD' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 蒙特塞拉特（MS）：3 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Saint Anthony', 'MS-03', 1),
    ('Saint Georges', 'MS-02', 2),
    ('Saint Peter', 'MS-01', 3)
) AS v(name, code, ord)
WHERE p.standard_code = 'MS' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 缅甸（MM）：15 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ayeyarwady', 'MM-07', 1),
    ('Bago', 'MM-02', 2),
    ('Chin', 'MM-14', 3),
    ('Kachin', 'MM-11', 4),
    ('Kayah', 'MM-12', 5),
    ('Kayin', 'MM-13', 6),
    ('Magway', 'MM-03', 7),
    ('Mandalay', 'MM-04', 8),
    ('Mon State', 'MM-15', 9),
    ('Naypyidaw', 'MM-18', 10),
    ('Rakhine', 'MM-16', 11),
    ('Sagaing', 'MM-01', 12),
    ('Shan', 'MM-17', 13),
    ('Tanintharyi', 'MM-05', 14),
    ('Yangon', 'MM-06', 15)
) AS v(name, code, ord)
WHERE p.standard_code = 'MM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 密克罗尼西亚（FM）：4 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Chuuk', 'FM-TRK', 1),
    ('Kosrae', 'FM-KSA', 2),
    ('Pohnpei', 'FM-PNI', 3),
    ('Yap', 'FM-YAP', 4)
) AS v(name, code, ord)
WHERE p.standard_code = 'FM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 摩尔多瓦（MD）：37 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Anenii Noi', 'MD-AN', 1),
    ('Bălți', 'MD-BA', 2),
    ('Basarabeasca', 'MD-BS', 3),
    ('Bender', 'MD-BD', 4),
    ('Briceni', 'MD-BR', 5),
    ('Cahul', 'MD-CA', 6),
    ('Călărași', 'MD-CL', 7),
    ('Cantemir', 'MD-CT', 8),
    ('Căușeni', 'MD-CS', 9),
    ('Chișinău', 'MD-CU', 10),
    ('Cimișlia', 'MD-CM', 11),
    ('Criuleni', 'MD-CR', 12),
    ('Dondușeni', 'MD-DO', 13),
    ('Drochia', 'MD-DR', 14),
    ('Dubăsari', 'MD-DU', 15),
    ('Edineț', 'MD-ED', 16),
    ('Fălești', 'MD-FA', 17),
    ('Florești', 'MD-FL', 18),
    ('Gagauzia', 'MD-GA', 19),
    ('Glodeni', 'MD-GL', 20),
    ('Hîncești', 'MD-HI', 21),
    ('Ialoveni', 'MD-IA', 22),
    ('Leova', 'MD-LE', 23),
    ('Nisporeni', 'MD-NI', 24),
    ('Ocnița', 'MD-OC', 25),
    ('Orhei', 'MD-OR', 26),
    ('Rezina', 'MD-RE', 27),
    ('Rîșcani', 'MD-RI', 28),
    ('Sîngerei', 'MD-SI', 29),
    ('Șoldănești', 'MD-SD', 30),
    ('Soroca', 'MD-SO', 31),
    ('Ștefan Vodă', 'MD-SV', 32),
    ('Strășeni', 'MD-ST', 33),
    ('Taraclia', 'MD-TA', 34),
    ('Telenești', 'MD-TE', 35),
    ('Transnistria', 'MD-SN', 36),
    ('Ungheni', 'MD-UN', 37)
) AS v(name, code, ord)
WHERE p.standard_code = 'MD' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 摩洛哥（MA）：87 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Agadir-Ida-Ou-Tanane', 'MA-AGD', 1),
    ('Al Haouz', 'MA-HAO', 2),
    ('Al Hoceïma', 'MA-HOC', 3),
    ('Aousserd (EH)', 'MA-AOU', 4),
    ('Assa-Zag (EH-partial)', 'MA-ASZ', 5),
    ('Azilal', 'MA-AZI', 6),
    ('Béni Mellal', 'MA-BEM', 7),
    ('Béni Mellal-Khénifra', 'MA-05', 8),
    ('Benslimane', 'MA-BES', 9),
    ('Berkane', 'MA-BER', 10),
    ('Berrechid', 'MA-BRR', 11),
    ('Boujdour (EH)', 'MA-BOD', 12),
    ('Boulemane', 'MA-BOM', 13),
    ('Casablanca', 'MA-CAS', 14),
    ('Casablanca-Settat', 'MA-06', 15),
    ('Chefchaouen', 'MA-CHE', 16),
    ('Chichaoua', 'MA-CHI', 17),
    ('Chtouka-Ait Baha', 'MA-CHT', 18),
    ('Dakhla-Oued Ed-Dahab (EH)', 'MA-12', 19),
    ('Drâa-Tafilalet', 'MA-08', 20),
    ('Driouch', 'MA-DRI', 21),
    ('El Hajeb', 'MA-HAJ', 22),
    ('El Jadida', 'MA-JDI', 23),
    ('El Kelâa des Sraghna', 'MA-KES', 24),
    ('Errachidia', 'MA-ERR', 25),
    ('Es-Semara (EH-partial)', 'MA-ESM', 26),
    ('Essaouira', 'MA-ESI', 27),
    ('Fahs-Anjra', 'MA-FAH', 28),
    ('Fès', 'MA-FES', 29),
    ('Fès-Meknès', 'MA-03', 30),
    ('Figuig', 'MA-FIG', 31),
    ('Fquih Ben Salah', 'MA-FQH', 32),
    ('Guelmim', 'MA-GUE', 33),
    ('Guelmim-Oued Noun (EH-partial)', 'MA-10', 34),
    ('Guercif', 'MA-GUF', 35),
    ('Ifrane', 'MA-IFR', 36),
    ('Inezgane-Ait Melloul', 'MA-INE', 37),
    ('Jerada', 'MA-JRA', 38),
    ('Kénitra', 'MA-KEN', 39),
    ('Khémisset', 'MA-KHE', 40),
    ('Khénifra', 'MA-KHN', 41),
    ('Khouribga', 'MA-KHO', 42),
    ('L''Oriental', 'MA-02', 43),
    ('Laâyoune (EH)', 'MA-LAA', 44),
    ('Laâyoune-Sakia El Hamra (EH-partial)', 'MA-11', 45),
    ('Larache', 'MA-LAR', 46),
    ('M’diq-Fnideq', 'MA-MDF', 47),
    ('Marrakech', 'MA-MAR', 48),
    ('Marrakesh-Safi', 'MA-07', 49),
    ('Médiouna', 'MA-MED', 50),
    ('Meknès', 'MA-MEK', 51),
    ('Midelt', 'MA-MID', 52),
    ('Mohammadia', 'MA-MOH', 53),
    ('Moulay Yacoub', 'MA-MOU', 54),
    ('Nador', 'MA-NAD', 55),
    ('Nouaceur', 'MA-NOU', 56),
    ('Ouarzazate', 'MA-OUA', 57),
    ('Oued Ed-Dahab (EH)', 'MA-OUD', 58),
    ('Ouezzane', 'MA-OUZ', 59),
    ('Oujda-Angad', 'MA-OUJ', 60),
    ('Rabat', 'MA-RAB', 61),
    ('Rabat-Salé-Kénitra', 'MA-04', 62),
    ('Rehamna', 'MA-REH', 63),
    ('Safi', 'MA-SAF', 64),
    ('Salé', 'MA-SAL', 65),
    ('Sefrou', 'MA-SEF', 66),
    ('Settat', 'MA-SET', 67),
    ('Sidi Bennour', 'MA-SIB', 68),
    ('Sidi Ifni', 'MA-SIF', 69),
    ('Sidi Kacem', 'MA-SIK', 70),
    ('Sidi Slimane', 'MA-SIL', 71),
    ('Skhirate-Témara', 'MA-SKH', 72),
    ('Souss-Massa', 'MA-09', 73),
    ('Tan-Tan (EH-partial)', 'MA-TNT', 74),
    ('Tanger-Assilah', 'MA-TNG', 75),
    ('Tanger-Tétouan-Al Hoceïma', 'MA-01', 76),
    ('Taounate', 'MA-TAO', 77),
    ('Taourirt', 'MA-TAI', 78),
    ('Tarfaya (EH-partial)', 'MA-TAF', 79),
    ('Taroudannt', 'MA-TAR', 80),
    ('Tata', 'MA-TAT', 81),
    ('Taza', 'MA-TAZ', 82),
    ('Tétouan', 'MA-TET', 83),
    ('Tinghir', 'MA-TIN', 84),
    ('Tiznit', 'MA-TIZ', 85),
    ('Youssoufia', 'MA-YUS', 86),
    ('Zagora', 'MA-ZAG', 87)
) AS v(name, code, ord)
WHERE p.standard_code = 'MA' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 摩纳哥（MC）：17 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Fontvieille', 'MC-FO', 1),
    ('Jardin Exotique', 'MC-JE', 2),
    ('La Colle', 'MC-CL', 3),
    ('La Condamine', 'MC-CO', 4),
    ('La Gare', 'MC-GA', 5),
    ('La Source', 'MC-SO', 6),
    ('Larvotto', 'MC-LA', 7),
    ('Malbousquet', 'MC-MA', 8),
    ('Monaco-Ville', 'MC-MO', 9),
    ('Moneghetti', 'MC-MG', 10),
    ('Monte-Carlo', 'MC-MC', 11),
    ('Moulins', 'MC-MU', 12),
    ('Port-Hercule', 'MC-PH', 13),
    ('Saint-Roman', 'MC-SR', 14),
    ('Sainte-Dévote', 'MC-SD', 15),
    ('Spélugues', 'MC-SP', 16),
    ('Vallon de la Rousse', 'MC-VR', 17)
) AS v(name, code, ord)
WHERE p.standard_code = 'MC' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 莫桑比克（MZ）：11 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Cabo Delgado', 'MZ-P', 1),
    ('Gaza', 'MZ-G', 2),
    ('Inhambane', 'MZ-I', 3),
    ('Manica', 'MZ-B', 4),
    ('Maputo', 'MZ-L', 5),
    ('Maputo', 'MZ-MPM', 6),
    ('Nampula', 'MZ-N', 7),
    ('Niassa', 'MZ-A', 8),
    ('Sofala', 'MZ-S', 9),
    ('Tete', 'MZ-T', 10),
    ('Zambezia', 'MZ-Q', 11)
) AS v(name, code, ord)
WHERE p.standard_code = 'MZ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 墨西哥（MX）：32 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Aguascalientes', 'MX-AGU', 1),
    ('Baja California', 'MX-BCN', 2),
    ('Baja California Sur', 'MX-BCS', 3),
    ('Campeche', 'MX-CAM', 4),
    ('Chiapas', 'MX-CHP', 5),
    ('Chihuahua', 'MX-CHH', 6),
    ('Ciudad de México', 'MX-CMX', 7),
    ('Coahuila de Zaragoza', 'MX-COA', 8),
    ('Colima', 'MX-COL', 9),
    ('Durango', 'MX-DUR', 10),
    ('Estado de México', 'MX-MEX', 11),
    ('Guanajuato', 'MX-GUA', 12),
    ('Guerrero', 'MX-GRO', 13),
    ('Hidalgo', 'MX-HID', 14),
    ('Jalisco', 'MX-JAL', 15),
    ('Michoacán de Ocampo', 'MX-MIC', 16),
    ('Morelos', 'MX-MOR', 17),
    ('Nayarit', 'MX-NAY', 18),
    ('Nuevo León', 'MX-NLE', 19),
    ('Oaxaca', 'MX-OAX', 20),
    ('Puebla', 'MX-PUE', 21),
    ('Querétaro', 'MX-QUE', 22),
    ('Quintana Roo', 'MX-ROO', 23),
    ('San Luis Potosí', 'MX-SLP', 24),
    ('Sinaloa', 'MX-SIN', 25),
    ('Sonora', 'MX-SON', 26),
    ('Tabasco', 'MX-TAB', 27),
    ('Tamaulipas', 'MX-TAM', 28),
    ('Tlaxcala', 'MX-TLA', 29),
    ('Veracruz de Ignacio de la Llave', 'MX-VER', 30),
    ('Yucatán', 'MX-YUC', 31),
    ('Zacatecas', 'MX-ZAC', 32)
) AS v(name, code, ord)
WHERE p.standard_code = 'MX' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 纳米比亚（NA）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Erongo', 'NA-ER', 1),
    ('Hardap', 'NA-HA', 2),
    ('Karas', 'NA-KA', 3),
    ('Kavango East', 'NA-KE', 4),
    ('Kavango West', 'NA-KW', 5),
    ('Khomas', 'NA-KH', 6),
    ('Kunene', 'NA-KU', 7),
    ('Ohangwena', 'NA-OW', 8),
    ('Omaheke', 'NA-OH', 9),
    ('Omusati', 'NA-OS', 10),
    ('Oshana', 'NA-ON', 11),
    ('Oshikoto', 'NA-OT', 12),
    ('Otjozondjupa', 'NA-OD', 13),
    ('Zambezi', 'NA-CA', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'NA' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 南非（ZA）：9 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Eastern Cape', 'ZA-EC', 1),
    ('Free State', 'ZA-FS', 2),
    ('Gauteng', 'ZA-GP', 3),
    ('KwaZulu-Natal', 'ZA-KZN', 4),
    ('Limpopo', 'ZA-LP', 5),
    ('Mpumalanga', 'ZA-MP', 6),
    ('North West', 'ZA-NW', 7),
    ('Northern Cape', 'ZA-NC', 8),
    ('Western Cape', 'ZA-WC', 9)
) AS v(name, code, ord)
WHERE p.standard_code = 'ZA' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 南苏丹（SS）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Central Equatoria', 'SS-EC', 1),
    ('Eastern Equatoria', 'SS-EE', 2),
    ('Jonglei State', 'SS-JG', 3),
    ('Lakes', 'SS-LK', 4),
    ('Northern Bahr el Ghazal', 'SS-BN', 5),
    ('Unity', 'SS-UY', 6),
    ('Upper Nile', 'SS-NU', 7),
    ('Warrap', 'SS-WR', 8),
    ('Western Bahr el Ghazal', 'SS-BW', 9),
    ('Western Equatoria', 'SS-EW', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'SS' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 瑙鲁（NR）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Aiwo', 'NR-01', 1),
    ('Anabar', 'NR-02', 2),
    ('Anetan', 'NR-03', 3),
    ('Anibare', 'NR-04', 4),
    ('Baiti', 'NR-05', 5),
    ('Boe', 'NR-06', 6),
    ('Buada', 'NR-07', 7),
    ('Denigomodu', 'NR-08', 8),
    ('Ewa', 'NR-09', 9),
    ('Ijuw', 'NR-10', 10),
    ('Meneng', 'NR-11', 11),
    ('Nibok', 'NR-12', 12),
    ('Uaboe', 'NR-13', 13),
    ('Yaren', 'NR-14', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'NR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 尼加拉瓜（NI）：17 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Boaco', 'NI-BO', 1),
    ('Carazo', 'NI-CA', 2),
    ('Chinandega', 'NI-CI', 3),
    ('Chontales', 'NI-CO', 4),
    ('Estelí', 'NI-ES', 5),
    ('Granada', 'NI-GR', 6),
    ('Jinotega', 'NI-JI', 7),
    ('León', 'NI-LE', 8),
    ('Madriz', 'NI-MD', 9),
    ('Managua', 'NI-MN', 10),
    ('Masaya', 'NI-MS', 11),
    ('Matagalpa', 'NI-MT', 12),
    ('North Caribbean Coast', 'NI-AN', 13),
    ('Nueva Segovia', 'NI-NS', 14),
    ('Río San Juan', 'NI-SJ', 15),
    ('Rivas', 'NI-RI', 16),
    ('South Caribbean Coast', 'NI-AS', 17)
) AS v(name, code, ord)
WHERE p.standard_code = 'NI' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 尼泊尔（NP）：7 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bagmati', 'NP-P3', 1),
    ('Gandaki', 'NP-P4', 2),
    ('Karnali', 'NP-P6', 3),
    ('Koshi', 'NP-P1', 4),
    ('Lumbini', 'NP-P5', 5),
    ('Madhesh', 'NP-P2', 6),
    ('Sudurpashchim', 'NP-P7', 7)
) AS v(name, code, ord)
WHERE p.standard_code = 'NP' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 尼日尔（NE）：8 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Agadez', 'NE-1', 1),
    ('Diffa', 'NE-2', 2),
    ('Dosso', 'NE-3', 3),
    ('Maradi', 'NE-4', 4),
    ('Niamey', 'NE-8', 5),
    ('Tahoua', 'NE-5', 6),
    ('Tillabéri', 'NE-6', 7),
    ('Zinder', 'NE-7', 8)
) AS v(name, code, ord)
WHERE p.standard_code = 'NE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 尼日利亚（NG）：37 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Abia', 'NG-AB', 1),
    ('Abuja Federal Capital Territory', 'NG-FC', 2),
    ('Adamawa', 'NG-AD', 3),
    ('Akwa Ibom', 'NG-AK', 4),
    ('Anambra', 'NG-AN', 5),
    ('Bauchi', 'NG-BA', 6),
    ('Bayelsa', 'NG-BY', 7),
    ('Benue', 'NG-BE', 8),
    ('Borno', 'NG-BO', 9),
    ('Cross River', 'NG-CR', 10),
    ('Delta', 'NG-DE', 11),
    ('Ebonyi', 'NG-EB', 12),
    ('Edo', 'NG-ED', 13),
    ('Ekiti', 'NG-EK', 14),
    ('Enugu', 'NG-EN', 15),
    ('Gombe', 'NG-GO', 16),
    ('Imo', 'NG-IM', 17),
    ('Jigawa', 'NG-JI', 18),
    ('Kaduna', 'NG-KD', 19),
    ('Kano', 'NG-KN', 20),
    ('Katsina', 'NG-KT', 21),
    ('Kebbi', 'NG-KE', 22),
    ('Kogi', 'NG-KO', 23),
    ('Kwara', 'NG-KW', 24),
    ('Lagos', 'NG-LA', 25),
    ('Nasarawa', 'NG-NA', 26),
    ('Niger', 'NG-NI', 27),
    ('Ogun', 'NG-OG', 28),
    ('Ondo', 'NG-ON', 29),
    ('Osun', 'NG-OS', 30),
    ('Oyo', 'NG-OY', 31),
    ('Plateau', 'NG-PL', 32),
    ('Rivers', 'NG-RI', 33),
    ('Sokoto', 'NG-SO', 34),
    ('Taraba', 'NG-TA', 35),
    ('Yobe', 'NG-YO', 36),
    ('Zamfara', 'NG-ZA', 37)
) AS v(name, code, ord)
WHERE p.standard_code = 'NG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 纽埃（NU）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alofi North', 'NU-14', 1),
    ('Alofi South', 'NU-13', 2),
    ('Avatele', 'NU-11', 3),
    ('Hakupu', 'NU-09', 4),
    ('Hikutavake', 'NU-04', 5),
    ('Lakepa', 'NU-07', 6),
    ('Liku', 'NU-08', 7),
    ('Makefu', 'NU-01', 8),
    ('Mutalau', 'NU-06', 9),
    ('Namukulu', 'NU-03', 10),
    ('Tamakautoga', 'NU-12', 11),
    ('Toi', 'NU-05', 12),
    ('Tuapa', 'NU-02', 13),
    ('Vaiea', 'NU-10', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'NU' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 挪威（NO）：17 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Agder', 'NO-42', 1),
    ('Akershus', 'NO-32', 2),
    ('Buskerud', 'NO-33', 3),
    ('Finnmark', 'NO-56', 4),
    ('Innlandet', 'NO-34', 5),
    ('Jan Mayen', 'NO-22', 6),
    ('Møre og Romsdal', 'NO-15', 7),
    ('Nordland', 'NO-18', 8),
    ('Oslo', 'NO-03', 9),
    ('Østfold', 'NO-31', 10),
    ('Rogaland', 'NO-11', 11),
    ('Svalbard', 'NO-21', 12),
    ('Telemark', 'NO-40', 13),
    ('Troms', 'NO-55', 14),
    ('Trøndelag', 'NO-50', 15),
    ('Vestfold', 'NO-39', 16),
    ('Vestland', 'NO-46', 17)
) AS v(name, code, ord)
WHERE p.standard_code = 'NO' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 帕劳（PW）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Aimeliik', 'PW-002', 1),
    ('Airai', 'PW-004', 2),
    ('Angaur', 'PW-010', 3),
    ('Hatohobei', 'PW-050', 4),
    ('Kayangel', 'PW-100', 5),
    ('Koror', 'PW-150', 6),
    ('Melekeok', 'PW-212', 7),
    ('Ngaraard', 'PW-214', 8),
    ('Ngarchelong', 'PW-218', 9),
    ('Ngardmau', 'PW-222', 10),
    ('Ngatpang', 'PW-224', 11),
    ('Ngchesar', 'PW-226', 12),
    ('Ngeremlengui', 'PW-227', 13),
    ('Ngiwal', 'PW-228', 14),
    ('Peleliu', 'PW-350', 15),
    ('Sonsorol', 'PW-370', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'PW' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 葡萄牙（PT）：20 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Açores', 'PT-20', 1),
    ('Aveiro', 'PT-01', 2),
    ('Beja', 'PT-02', 3),
    ('Braga', 'PT-03', 4),
    ('Bragança', 'PT-04', 5),
    ('Castelo Branco', 'PT-05', 6),
    ('Coimbra', 'PT-06', 7),
    ('Évora', 'PT-07', 8),
    ('Faro', 'PT-08', 9),
    ('Guarda', 'PT-09', 10),
    ('Leiria', 'PT-10', 11),
    ('Lisbon', 'PT-11', 12),
    ('Madeira', 'PT-30', 13),
    ('Portalegre', 'PT-12', 14),
    ('Porto', 'PT-13', 15),
    ('Santarém', 'PT-14', 16),
    ('Setúbal', 'PT-15', 17),
    ('Viana do Castelo', 'PT-16', 18),
    ('Vila Real', 'PT-17', 19),
    ('Viseu', 'PT-18', 20)
) AS v(name, code, ord)
WHERE p.standard_code = 'PT' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 日本（JP）：47 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Aichi', 'JP-23', 1),
    ('Akita', 'JP-05', 2),
    ('Aomori', 'JP-02', 3),
    ('Chiba', 'JP-12', 4),
    ('Ehime', 'JP-38', 5),
    ('Fukui', 'JP-18', 6),
    ('Fukuoka', 'JP-40', 7),
    ('Fukushima', 'JP-07', 8),
    ('Gifu', 'JP-21', 9),
    ('Gunma', 'JP-10', 10),
    ('Hiroshima', 'JP-34', 11),
    ('Hokkaidō', 'JP-01', 12),
    ('Hyōgo', 'JP-28', 13),
    ('Ibaraki', 'JP-08', 14),
    ('Ishikawa', 'JP-17', 15),
    ('Iwate', 'JP-03', 16),
    ('Kagawa', 'JP-37', 17),
    ('Kagoshima', 'JP-46', 18),
    ('Kanagawa', 'JP-14', 19),
    ('Kōchi', 'JP-39', 20),
    ('Kumamoto', 'JP-43', 21),
    ('Kyōto', 'JP-26', 22),
    ('Mie', 'JP-24', 23),
    ('Miyagi', 'JP-04', 24),
    ('Miyazaki', 'JP-45', 25),
    ('Nagano', 'JP-20', 26),
    ('Nagasaki', 'JP-42', 27),
    ('Nara', 'JP-29', 28),
    ('Niigata', 'JP-15', 29),
    ('Ōita', 'JP-44', 30),
    ('Okayama', 'JP-33', 31),
    ('Okinawa', 'JP-47', 32),
    ('Ōsaka', 'JP-27', 33),
    ('Saga', 'JP-41', 34),
    ('Saitama', 'JP-11', 35),
    ('Shiga', 'JP-25', 36),
    ('Shimane', 'JP-32', 37),
    ('Shizuoka', 'JP-22', 38),
    ('Tochigi', 'JP-09', 39),
    ('Tokushima', 'JP-36', 40),
    ('Tokyo', 'JP-13', 41),
    ('Tottori', 'JP-31', 42),
    ('Toyama', 'JP-16', 43),
    ('Wakayama', 'JP-30', 44),
    ('Yamagata', 'JP-06', 45),
    ('Yamaguchi', 'JP-35', 46),
    ('Yamanashi', 'JP-19', 47)
) AS v(name, code, ord)
WHERE p.standard_code = 'JP' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 瑞典（SE）：21 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Blekinge', 'SE-K', 1),
    ('Dalarna', 'SE-W', 2),
    ('Gävleborg', 'SE-X', 3),
    ('Gotland', 'SE-I', 4),
    ('Halland', 'SE-N', 5),
    ('Jämtland', 'SE-Z', 6),
    ('Jönköping', 'SE-F', 7),
    ('Kalmar', 'SE-H', 8),
    ('Kronoberg', 'SE-G', 9),
    ('Norrbotten', 'SE-BD', 10),
    ('Örebro', 'SE-T', 11),
    ('Östergötland', 'SE-E', 12),
    ('Skåne', 'SE-M', 13),
    ('Södermanland', 'SE-D', 14),
    ('Stockholm', 'SE-AB', 15),
    ('Uppsala', 'SE-C', 16),
    ('Värmland', 'SE-S', 17),
    ('Västerbotten', 'SE-AC', 18),
    ('Västernorrland', 'SE-Y', 19),
    ('Västmanland', 'SE-U', 20),
    ('Västra Götaland', 'SE-O', 21)
) AS v(name, code, ord)
WHERE p.standard_code = 'SE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 瑞士（CH）：26 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Aargau', 'CH-AG', 1),
    ('Appenzell Ausserrhoden', 'CH-AR', 2),
    ('Appenzell Innerrhoden', 'CH-AI', 3),
    ('Basel-Land', 'CH-BL', 4),
    ('Basel-Stadt', 'CH-BS', 5),
    ('Bern', 'CH-BE', 6),
    ('Fribourg', 'CH-FR', 7),
    ('Geneva', 'CH-GE', 8),
    ('Glarus', 'CH-GL', 9),
    ('Graubünden', 'CH-GR', 10),
    ('Jura', 'CH-JU', 11),
    ('Lucerne', 'CH-LU', 12),
    ('Neuchâtel', 'CH-NE', 13),
    ('Nidwalden', 'CH-NW', 14),
    ('Obwalden', 'CH-OW', 15),
    ('Schaffhausen', 'CH-SH', 16),
    ('Schwyz', 'CH-SZ', 17),
    ('Solothurn', 'CH-SO', 18),
    ('St. Gallen', 'CH-SG', 19),
    ('Thurgau', 'CH-TG', 20),
    ('Ticino', 'CH-TI', 21),
    ('Uri', 'CH-UR', 22),
    ('Valais', 'CH-VS', 23),
    ('Vaud', 'CH-VD', 24),
    ('Zug', 'CH-ZG', 25),
    ('Zürich', 'CH-ZH', 26)
) AS v(name, code, ord)
WHERE p.standard_code = 'CH' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 萨尔瓦多（SV）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ahuachapán', 'SV-AH', 1),
    ('Cabañas', 'SV-CA', 2),
    ('Chalatenango', 'SV-CH', 3),
    ('Cuscatlán', 'SV-CU', 4),
    ('La Libertad', 'SV-LI', 5),
    ('La Paz', 'SV-PA', 6),
    ('La Unión', 'SV-UN', 7),
    ('Morazán', 'SV-MO', 8),
    ('San Miguel', 'SV-SM', 9),
    ('San Salvador', 'SV-SS', 10),
    ('San Vicente', 'SV-SV', 11),
    ('Santa Ana', 'SV-SA', 12),
    ('Sonsonate', 'SV-SO', 13),
    ('Usulután', 'SV-US', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'SV' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 塞尔维亚（RS）：32 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Belgrade', 'RS-00', 1),
    ('Bor', 'RS-14', 2),
    ('Braničevo', 'RS-11', 3),
    ('Central Banat', 'RS-02', 4),
    ('Jablanica', 'RS-23', 5),
    ('Kolubara', 'RS-09', 6),
    ('Kosovo', 'RS-25', 7),
    ('Kosovo-Metohija', 'RS-KM', 8),
    ('Kosovo-Pomoravlje', 'RS-29', 9),
    ('Kosovska Mitrovica', 'RS-28', 10),
    ('Mačva', 'RS-08', 11),
    ('Moravica', 'RS-17', 12),
    ('Nišava', 'RS-20', 13),
    ('North Bačka', 'RS-01', 14),
    ('North Banat', 'RS-03', 15),
    ('Pčinja', 'RS-24', 16),
    ('Peć', 'RS-26', 17),
    ('Pirot', 'RS-22', 18),
    ('Podunavlje', 'RS-10', 19),
    ('Pomoravlje', 'RS-13', 20),
    ('Prizren', 'RS-27', 21),
    ('Rasina', 'RS-19', 22),
    ('Raška', 'RS-18', 23),
    ('South Bačka', 'RS-06', 24),
    ('South Banat', 'RS-04', 25),
    ('Srem', 'RS-07', 26),
    ('Šumadija', 'RS-12', 27),
    ('Toplica', 'RS-21', 28),
    ('Vojvodina', 'RS-VO', 29),
    ('West Bačka', 'RS-05', 30),
    ('Zaječar', 'RS-15', 31),
    ('Zlatibor', 'RS-16', 32)
) AS v(name, code, ord)
WHERE p.standard_code = 'RS' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 塞拉利昂（SL）：5 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Eastern', 'SL-E', 1),
    ('North Western', 'SL-NW', 2),
    ('Northern', 'SL-N', 3),
    ('Southern', 'SL-S', 4),
    ('Western', 'SL-W', 5)
) AS v(name, code, ord)
WHERE p.standard_code = 'SL' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 塞内加尔（SN）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Dakar', 'SN-DK', 1),
    ('Diourbel Region', 'SN-DB', 2),
    ('Fatick', 'SN-FK', 3),
    ('Kaffrine', 'SN-KA', 4),
    ('Kaolack', 'SN-KL', 5),
    ('Kédougou', 'SN-KE', 6),
    ('Kolda', 'SN-KD', 7),
    ('Louga', 'SN-LG', 8),
    ('Matam', 'SN-MT', 9),
    ('Saint-Louis', 'SN-SL', 10),
    ('Sédhiou', 'SN-SE', 11),
    ('Tambacounda Region', 'SN-TC', 12),
    ('Thiès Region', 'SN-TH', 13),
    ('Ziguinchor', 'SN-ZG', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'SN' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 塞浦路斯（CY）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Famagusta (Mağusa)', 'CY-04', 1),
    ('Kyrenia (Keryneia)', 'CY-06', 2),
    ('Larnaca (Larnaka)', 'CY-03', 3),
    ('Limassol (Leymasun)', 'CY-02', 4),
    ('Nicosia (Lefkoşa)', 'CY-01', 5),
    ('Paphos (Pafos)', 'CY-05', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'CY' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 塞舌尔（SC）：27 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Anse Boileau', 'SC-02', 1),
    ('Anse Etoile', 'SC-03', 2),
    ('Anse Royale', 'SC-05', 3),
    ('Anse-aux-Pins', 'SC-01', 4),
    ('Au Cap', 'SC-04', 5),
    ('Baie Lazare', 'SC-06', 6),
    ('Baie Sainte Anne', 'SC-07', 7),
    ('Beau Vallon', 'SC-08', 8),
    ('Bel Air', 'SC-09', 9),
    ('Bel Ombre', 'SC-10', 10),
    ('Cascade', 'SC-11', 11),
    ('Glacis', 'SC-12', 12),
    ('Grand''Anse Mahé', 'SC-13', 13),
    ('Grand''Anse Praslin', 'SC-14', 14),
    ('Ile Perseverance I', 'SC-26', 15),
    ('Ile Perseverance II', 'SC-27', 16),
    ('La Digue', 'SC-15', 17),
    ('La Rivière Anglaise', 'SC-16', 18),
    ('Les Mamelles', 'SC-24', 19),
    ('Mont Buxton', 'SC-17', 20),
    ('Mont Fleuri', 'SC-18', 21),
    ('Plaisance', 'SC-19', 22),
    ('Pointe La Rue', 'SC-20', 23),
    ('Port Glaud', 'SC-21', 24),
    ('Roche Caiman', 'SC-25', 25),
    ('Saint Louis', 'SC-22', 26),
    ('Takamaka', 'SC-23', 27)
) AS v(name, code, ord)
WHERE p.standard_code = 'SC' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 萨摩亚（WS）：11 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('A''ana', 'WS-AA', 1),
    ('Aiga-i-le-Tai', 'WS-AL', 2),
    ('Atua', 'WS-AT', 3),
    ('Fa''asaleleaga', 'WS-FA', 4),
    ('Gaga''emauga', 'WS-GE', 5),
    ('Gaga''ifomauga', 'WS-GI', 6),
    ('Palauli', 'WS-PA', 7),
    ('Satupa''itea', 'WS-SA', 8),
    ('Tuamasaga', 'WS-TU', 9),
    ('Va''a-o-Fonoti', 'WS-VF', 10),
    ('Vaisigano', 'WS-VS', 11)
) AS v(name, code, ord)
WHERE p.standard_code = 'WS' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 沙特阿拉伯（SA）：13 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Al Bahah', 'SA-11', 1),
    ('Al Jawf', 'SA-12', 2),
    ('Al Madinah', 'SA-03', 3),
    ('Al-Qassim', 'SA-05', 4),
    ('Asir', 'SA-14', 5),
    ('Eastern Province', 'SA-04', 6),
    ('Ha''il', 'SA-06', 7),
    ('Jizan', 'SA-09', 8),
    ('Makkah', 'SA-02', 9),
    ('Najran', 'SA-10', 10),
    ('Northern Borders', 'SA-08', 11),
    ('Riyadh', 'SA-01', 12),
    ('Tabuk', 'SA-07', 13)
) AS v(name, code, ord)
WHERE p.standard_code = 'SA' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 圣巴泰勒米（BL）：1 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Saint-Barthélemy', 'FR-BL', 1)
) AS v(name, code, ord)
WHERE p.standard_code = 'BL' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 圣多美和普林西比（ST）：7 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Água Grande', 'ST-01', 1),
    ('Cantagalo', 'ST-02', 2),
    ('Caué', 'ST-03', 3),
    ('Lemba', 'ST-04', 4),
    ('Lobata', 'ST-05', 5),
    ('Mé-Zóchi', 'ST-06', 6),
    ('Príncipe', 'ST-P', 7)
) AS v(name, code, ord)
WHERE p.standard_code = 'ST' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 圣赫勒拿（SH）：11 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alarm Forest', 'SH-01', 1),
    ('Ascension', 'SH-AC', 2),
    ('Blue Hill', 'SH-02', 3),
    ('Half Tree Hollow', 'SH-03', 4),
    ('Jamestown', 'SH-04', 5),
    ('Levelwood', 'SH-05', 6),
    ('Longwood', 'SH-06', 7),
    ('Saint Helena', 'SH-HL', 8),
    ('Saint Paul''s', 'SH-08', 9),
    ('Sandy Bay', 'SH-07', 10),
    ('Tristan da Cunha', 'SH-TA', 11)
) AS v(name, code, ord)
WHERE p.standard_code = 'SH' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 圣基茨和尼维斯（KN）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Christ Church Nichola Town', 'KN-01', 1),
    ('Nevis', 'KN-N', 2),
    ('Saint Anne Sandy Point', 'KN-02', 3),
    ('Saint George Basseterre', 'KN-03', 4),
    ('Saint George Gingerland', 'KN-04', 5),
    ('Saint James Windward', 'KN-05', 6),
    ('Saint John Capisterre', 'KN-06', 7),
    ('Saint John Figtree', 'KN-07', 8),
    ('Saint Kitts', 'KN-K', 9),
    ('Saint Mary Cayon', 'KN-08', 10),
    ('Saint Paul Capisterre', 'KN-09', 11),
    ('Saint Paul Charlestown', 'KN-10', 12),
    ('Saint Peter Basseterre', 'KN-11', 13),
    ('Saint Thomas Lowland', 'KN-12', 14),
    ('Saint Thomas Middle Island', 'KN-13', 15),
    ('Trinity Palmetto Point', 'KN-15', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'KN' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 圣卢西亚（LC）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Anse la Raye', 'LC-01', 1),
    ('Canaries', 'LC-12', 2),
    ('Castries', 'LC-02', 3),
    ('Choiseul', 'LC-03', 4),
    ('Dennery', 'LC-05', 5),
    ('Gros Islet', 'LC-06', 6),
    ('Laborie', 'LC-07', 7),
    ('Micoud', 'LC-08', 8),
    ('Soufrière', 'LC-10', 9),
    ('Vieux Fort', 'LC-11', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'LC' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 圣马力诺（SM）：9 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Acquaviva', 'SM-01', 1),
    ('Borgo Maggiore', 'SM-06', 2),
    ('Chiesanuova', 'SM-02', 3),
    ('Domagnano', 'SM-03', 4),
    ('Faetano', 'SM-04', 5),
    ('Fiorentino', 'SM-05', 6),
    ('Montegiardino', 'SM-08', 7),
    ('San Marino', 'SM-07', 8),
    ('Serravalle', 'SM-09', 9)
) AS v(name, code, ord)
WHERE p.standard_code = 'SM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 圣皮埃尔和密克隆群岛（PM）：1 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Saint-Pierre and Miquelon', 'FR-PM', 1)
) AS v(name, code, ord)
WHERE p.standard_code = 'PM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 圣文森特和格林纳丁斯（VC）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Charlotte', 'VC-01', 1),
    ('Grenadines', 'VC-06', 2),
    ('Saint Andrew', 'VC-02', 3),
    ('Saint David', 'VC-03', 4),
    ('Saint George', 'VC-04', 5),
    ('Saint Patrick', 'VC-05', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'VC' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 斯里兰卡（LK）：34 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ampara', 'LK-52', 1),
    ('Anuradhapura', 'LK-71', 2),
    ('Badulla', 'LK-81', 3),
    ('Batticaloa', 'LK-51', 4),
    ('Central', 'LK-2', 5),
    ('Colombo', 'LK-11', 6),
    ('Eastern', 'LK-5', 7),
    ('Galle', 'LK-31', 8),
    ('Gampaha', 'LK-12', 9),
    ('Hambantota', 'LK-33', 10),
    ('Jaffna', 'LK-41', 11),
    ('Kalutara', 'LK-13', 12),
    ('Kandy', 'LK-21', 13),
    ('Kegalle', 'LK-92', 14),
    ('Kilinochchi', 'LK-42', 15),
    ('Kurunegala', 'LK-61', 16),
    ('Mannar', 'LK-43', 17),
    ('Matale', 'LK-22', 18),
    ('Matara', 'LK-32', 19),
    ('Monaragala', 'LK-82', 20),
    ('Mullaitivu', 'LK-45', 21),
    ('North Central', 'LK-7', 22),
    ('North Western', 'LK-6', 23),
    ('Northern', 'LK-4', 24),
    ('Nuwara Eliya', 'LK-23', 25),
    ('Polonnaruwa', 'LK-72', 26),
    ('Puttalam', 'LK-62', 27),
    ('Ratnapura', 'LK-91', 28),
    ('Sabaragamuwa', 'LK-9', 29),
    ('Southern', 'LK-3', 30),
    ('Trincomalee', 'LK-53', 31),
    ('Uva', 'LK-8', 32),
    ('Vavuniya', 'LK-44', 33),
    ('Western', 'LK-1', 34)
) AS v(name, code, ord)
WHERE p.standard_code = 'LK' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 斯洛伐克（SK）：8 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Banská Bystrica', 'SK-BC', 1),
    ('Bratislava', 'SK-BL', 2),
    ('Košice', 'SK-KI', 3),
    ('Nitra', 'SK-NI', 4),
    ('Prešov', 'SK-PV', 5),
    ('Trenčín', 'SK-TC', 6),
    ('Trnava', 'SK-TA', 7),
    ('Žilina', 'SK-ZI', 8)
) AS v(name, code, ord)
WHERE p.standard_code = 'SK' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 斯洛文尼亚（SI）：212 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ajdovščina', 'SI-001', 1),
    ('Ankaran', 'SI-213', 2),
    ('Apače', 'SI-195', 3),
    ('Beltinci', 'SI-002', 4),
    ('Benedikt', 'SI-148', 5),
    ('Bistrica ob Sotli', 'SI-149', 6),
    ('Bled', 'SI-003', 7),
    ('Bloke', 'SI-150', 8),
    ('Bohinj', 'SI-004', 9),
    ('Borovnica', 'SI-005', 10),
    ('Bovec', 'SI-006', 11),
    ('Braslovče', 'SI-151', 12),
    ('Brda', 'SI-007', 13),
    ('Brežice', 'SI-009', 14),
    ('Brezovica', 'SI-008', 15),
    ('Cankova', 'SI-152', 16),
    ('Celje', 'SI-011', 17),
    ('Cerklje na Gorenjskem', 'SI-012', 18),
    ('Cerknica', 'SI-013', 19),
    ('Cerkno', 'SI-014', 20),
    ('Cerkvenjak', 'SI-153', 21),
    ('Cirkulane', 'SI-196', 22),
    ('Črenšovci', 'SI-015', 23),
    ('Črna na Koroškem', 'SI-016', 24),
    ('Črnomelj', 'SI-017', 25),
    ('Destrnik', 'SI-018', 26),
    ('Divača', 'SI-019', 27),
    ('Dobje', 'SI-154', 28),
    ('Dobrepolje', 'SI-020', 29),
    ('Dobrna', 'SI-155', 30),
    ('Dobrova–Polhov Gradec', 'SI-021', 31),
    ('Dobrovnik', 'SI-156', 32),
    ('Dol pri Ljubljani', 'SI-022', 33),
    ('Dolenjske Toplice', 'SI-157', 34),
    ('Domžale', 'SI-023', 35),
    ('Dornava', 'SI-024', 36),
    ('Dravograd', 'SI-025', 37),
    ('Duplek', 'SI-026', 38),
    ('Gorenja Vas–Poljane', 'SI-027', 39),
    ('Gorišnica', 'SI-028', 40),
    ('Gorje', 'SI-207', 41),
    ('Gornja Radgona', 'SI-029', 42),
    ('Gornji Grad', 'SI-030', 43),
    ('Gornji Petrovci', 'SI-031', 44),
    ('Grad', 'SI-158', 45),
    ('Grosuplje', 'SI-032', 46),
    ('Hajdina', 'SI-159', 47),
    ('Hoče–Slivnica', 'SI-160', 48),
    ('Hodoš', 'SI-161', 49),
    ('Horjul', 'SI-162', 50),
    ('Hrastnik', 'SI-034', 51),
    ('Hrpelje–Kozina', 'SI-035', 52),
    ('Idrija', 'SI-036', 53),
    ('Ig', 'SI-037', 54),
    ('Ilirska Bistrica', 'SI-038', 55),
    ('Ivančna Gorica', 'SI-039', 56),
    ('Izola', 'SI-040', 57),
    ('Jesenice', 'SI-041', 58),
    ('Jezersko', 'SI-163', 59),
    ('Juršinci', 'SI-042', 60),
    ('Kamnik', 'SI-043', 61),
    ('Kanal ob Soči', 'SI-044', 62),
    ('Kidričevo', 'SI-045', 63),
    ('Kobarid', 'SI-046', 64),
    ('Kobilje', 'SI-047', 65),
    ('Kočevje', 'SI-048', 66),
    ('Komen', 'SI-049', 67),
    ('Komenda', 'SI-164', 68),
    ('Koper', 'SI-050', 69),
    ('Kostanjevica na Krki', 'SI-197', 70),
    ('Kostel', 'SI-165', 71),
    ('Kozje', 'SI-051', 72),
    ('Kranj', 'SI-052', 73),
    ('Kranjska Gora', 'SI-053', 74),
    ('Križevci', 'SI-166', 75),
    ('Krško', 'SI-054', 76),
    ('Kungota', 'SI-055', 77),
    ('Kuzma', 'SI-056', 78),
    ('Laško', 'SI-057', 79),
    ('Lenart', 'SI-058', 80),
    ('Lendava', 'SI-059', 81),
    ('Litija', 'SI-060', 82),
    ('Ljubljana', 'SI-061', 83),
    ('Ljubno', 'SI-062', 84),
    ('Ljutomer', 'SI-063', 85),
    ('Log–Dragomer', 'SI-208', 86),
    ('Logatec', 'SI-064', 87),
    ('Loška Dolina', 'SI-065', 88),
    ('Loški Potok', 'SI-066', 89),
    ('Lovrenc na Pohorju', 'SI-167', 90),
    ('Luče', 'SI-067', 91),
    ('Lukovica', 'SI-068', 92),
    ('Majšperk', 'SI-069', 93),
    ('Makole', 'SI-198', 94),
    ('Maribor', 'SI-070', 95),
    ('Markovci', 'SI-168', 96),
    ('Medvode', 'SI-071', 97),
    ('Mengeš', 'SI-072', 98),
    ('Metlika', 'SI-073', 99),
    ('Mežica', 'SI-074', 100),
    ('Miklavž na Dravskem Polju', 'SI-169', 101),
    ('Miren–Kostanjevica', 'SI-075', 102),
    ('Mirna', 'SI-212', 103),
    ('Mirna Peč', 'SI-170', 104),
    ('Mislinja', 'SI-076', 105),
    ('Mokronog–Trebelno', 'SI-199', 106),
    ('Moravče', 'SI-077', 107),
    ('Moravske Toplice', 'SI-078', 108),
    ('Mozirje', 'SI-079', 109),
    ('Murska Sobota', 'SI-080', 110),
    ('Muta', 'SI-081', 111),
    ('Naklo', 'SI-082', 112),
    ('Nazarje', 'SI-083', 113),
    ('Nova Gorica', 'SI-084', 114),
    ('Novo Mesto', 'SI-085', 115),
    ('Odranci', 'SI-086', 116),
    ('Oplotnica', 'SI-171', 117),
    ('Ormož', 'SI-087', 118),
    ('Osilnica', 'SI-088', 119),
    ('Pesnica', 'SI-089', 120),
    ('Piran', 'SI-090', 121),
    ('Pivka', 'SI-091', 122),
    ('Podčetrtek', 'SI-092', 123),
    ('Podlehnik', 'SI-172', 124),
    ('Podvelka', 'SI-093', 125),
    ('Poljčane', 'SI-200', 126),
    ('Polzela', 'SI-173', 127),
    ('Postojna', 'SI-094', 128),
    ('Prebold', 'SI-174', 129),
    ('Preddvor', 'SI-095', 130),
    ('Prevalje', 'SI-175', 131),
    ('Ptuj', 'SI-096', 132),
    ('Puconci', 'SI-097', 133),
    ('Rače–Fram', 'SI-098', 134),
    ('Radeče', 'SI-099', 135),
    ('Radenci', 'SI-100', 136),
    ('Radlje ob Dravi', 'SI-101', 137),
    ('Radovljica', 'SI-102', 138),
    ('Ravne na Koroškem', 'SI-103', 139),
    ('Razkrižje', 'SI-176', 140),
    ('Rečica ob Savinji', 'SI-209', 141),
    ('Renče–Vogrsko', 'SI-201', 142),
    ('Ribnica', 'SI-104', 143),
    ('Ribnica na Pohorju', 'SI-177', 144),
    ('Rogaška Slatina', 'SI-106', 145),
    ('Rogašovci', 'SI-105', 146),
    ('Rogatec', 'SI-107', 147),
    ('Ruše', 'SI-108', 148),
    ('Šalovci', 'SI-033', 149),
    ('Selnica ob Dravi', 'SI-178', 150),
    ('Semič', 'SI-109', 151),
    ('Šempeter–Vrtojba', 'SI-183', 152),
    ('Šenčur', 'SI-117', 153),
    ('Šentilj', 'SI-118', 154),
    ('Šentjernej', 'SI-119', 155),
    ('Šentjur', 'SI-120', 156),
    ('Šentrupert', 'SI-211', 157),
    ('Sevnica', 'SI-110', 158),
    ('Sežana', 'SI-111', 159),
    ('Škocjan', 'SI-121', 160),
    ('Škofja Loka', 'SI-122', 161),
    ('Škofljica', 'SI-123', 162),
    ('Slovenj Gradec', 'SI-112', 163),
    ('Slovenska Bistrica', 'SI-113', 164),
    ('Slovenske Konjice', 'SI-114', 165),
    ('Šmarje pri Jelšah', 'SI-124', 166),
    ('Šmarješke Toplice', 'SI-206', 167),
    ('Šmartno ob Paki', 'SI-125', 168),
    ('Šmartno pri Litiji', 'SI-194', 169),
    ('Sodražica', 'SI-179', 170),
    ('Solčava', 'SI-180', 171),
    ('Šoštanj', 'SI-126', 172),
    ('Središče ob Dravi', 'SI-202', 173),
    ('Starše', 'SI-115', 174),
    ('Štore', 'SI-127', 175),
    ('Straža', 'SI-203', 176),
    ('Sveta Ana', 'SI-181', 177),
    ('Sveta Trojica v Slovenskih Goricah', 'SI-204', 178),
    ('Sveti Andraž v Slovenskih Goricah', 'SI-182', 179),
    ('Sveti Jurij ob Ščavnici', 'SI-116', 180),
    ('Sveti Jurij v Slovenskih Goricah', 'SI-210', 181),
    ('Sveti Tomaž', 'SI-205', 182),
    ('Tabor', 'SI-184', 183),
    ('Tišina', 'SI-010', 184),
    ('Tolmin', 'SI-128', 185),
    ('Trbovlje', 'SI-129', 186),
    ('Trebnje', 'SI-130', 187),
    ('Trnovska Vas', 'SI-185', 188),
    ('Tržič', 'SI-131', 189),
    ('Trzin', 'SI-186', 190),
    ('Turnišče', 'SI-132', 191),
    ('Velenje', 'SI-133', 192),
    ('Velika Polana', 'SI-187', 193),
    ('Velike Lašče', 'SI-134', 194),
    ('Veržej', 'SI-188', 195),
    ('Videm', 'SI-135', 196),
    ('Vipava', 'SI-136', 197),
    ('Vitanje', 'SI-137', 198),
    ('Vodice', 'SI-138', 199),
    ('Vojnik', 'SI-139', 200),
    ('Vransko', 'SI-189', 201),
    ('Vrhnika', 'SI-140', 202),
    ('Vuzenica', 'SI-141', 203),
    ('Zagorje ob Savi', 'SI-142', 204),
    ('Žalec', 'SI-190', 205),
    ('Zavrč', 'SI-143', 206),
    ('Železniki', 'SI-146', 207),
    ('Žetale', 'SI-191', 208),
    ('Žiri', 'SI-147', 209),
    ('Žirovnica', 'SI-192', 210),
    ('Zreče', 'SI-144', 211),
    ('Žužemberk', 'SI-193', 212)
) AS v(name, code, ord)
WHERE p.standard_code = 'SI' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 斯威士兰（SZ）：4 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Hhohho', 'SZ-HH', 1),
    ('Lubombo', 'SZ-LU', 2),
    ('Manzini', 'SZ-MA', 3),
    ('Shiselweni', 'SZ-SH', 4)
) AS v(name, code, ord)
WHERE p.standard_code = 'SZ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 苏丹（SD）：18 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Al Jazirah', 'SD-GZ', 1),
    ('Al Qadarif', 'SD-GD', 2),
    ('Blue Nile', 'SD-NB', 3),
    ('Central Darfur', 'SD-DC', 4),
    ('East Darfur', 'SD-DE', 5),
    ('Kassala', 'SD-KA', 6),
    ('Khartoum', 'SD-KH', 7),
    ('North Darfur', 'SD-DN', 8),
    ('North Kordofan', 'SD-KN', 9),
    ('Northern', 'SD-NO', 10),
    ('Red Sea', 'SD-RS', 11),
    ('River Nile', 'SD-NR', 12),
    ('Sennar', 'SD-SI', 13),
    ('South Darfur', 'SD-DS', 14),
    ('South Kordofan', 'SD-KS', 15),
    ('West Darfur', 'SD-DW', 16),
    ('West Kordofan', 'SD-GK', 17),
    ('White Nile', 'SD-NW', 18)
) AS v(name, code, ord)
WHERE p.standard_code = 'SD' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 苏里南（SR）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Brokopondo', 'SR-BR', 1),
    ('Commewijne', 'SR-CM', 2),
    ('Coronie', 'SR-CR', 3),
    ('Marowijne', 'SR-MA', 4),
    ('Nickerie', 'SR-NI', 5),
    ('Para', 'SR-PR', 6),
    ('Paramaribo', 'SR-PM', 7),
    ('Saramacca', 'SR-SA', 8),
    ('Sipaliwini', 'SR-SI', 9),
    ('Wanica', 'SR-WA', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'SR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 所罗门群岛（SB）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Central', 'SB-CE', 1),
    ('Choiseul', 'SB-CH', 2),
    ('Guadalcanal', 'SB-GU', 3),
    ('Honiara', 'SB-CT', 4),
    ('Isabel', 'SB-IS', 5),
    ('Makira-Ulawa', 'SB-MK', 6),
    ('Malaita', 'SB-ML', 7),
    ('Rennell and Bellona', 'SB-RB', 8),
    ('Temotu', 'SB-TE', 9),
    ('Western', 'SB-WE', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'SB' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 索马里（SO）：18 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Awdal', 'SO-AW', 1),
    ('Bakool', 'SO-BK', 2),
    ('Banaadir', 'SO-BN', 3),
    ('Bari', 'SO-BR', 4),
    ('Bay', 'SO-BY', 5),
    ('Galguduud', 'SO-GA', 6),
    ('Gedo', 'SO-GE', 7),
    ('Hiran', 'SO-HI', 8),
    ('Lower Juba', 'SO-JH', 9),
    ('Lower Shebelle', 'SO-SH', 10),
    ('Middle Juba', 'SO-JD', 11),
    ('Middle Shebelle', 'SO-SD', 12),
    ('Mudug', 'SO-MU', 13),
    ('Nugal', 'SO-NU', 14),
    ('Sanaag', 'SO-SA', 15),
    ('Sool', 'SO-SO', 16),
    ('Togdheer', 'SO-TO', 17),
    ('Woqooyi Galbeed', 'SO-WO', 18)
) AS v(name, code, ord)
WHERE p.standard_code = 'SO' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 泰国（TH）：78 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Amnat Charoen', 'TH-37', 1),
    ('Ang Thong', 'TH-15', 2),
    ('Bangkok', 'TH-10', 3),
    ('Bueng Kan', 'TH-38', 4),
    ('Buri Ram', 'TH-31', 5),
    ('Chachoengsao', 'TH-24', 6),
    ('Chai Nat', 'TH-18', 7),
    ('Chaiyaphum', 'TH-36', 8),
    ('Chanthaburi', 'TH-22', 9),
    ('Chiang Mai', 'TH-50', 10),
    ('Chiang Rai', 'TH-57', 11),
    ('Chon Buri', 'TH-20', 12),
    ('Chumphon', 'TH-86', 13),
    ('Kalasin', 'TH-46', 14),
    ('Kamphaeng Phet', 'TH-62', 15),
    ('Kanchanaburi', 'TH-71', 16),
    ('Khon Kaen', 'TH-40', 17),
    ('Krabi', 'TH-81', 18),
    ('Lampang', 'TH-52', 19),
    ('Lamphun', 'TH-51', 20),
    ('Loei', 'TH-42', 21),
    ('Lop Buri', 'TH-16', 22),
    ('Mae Hong Son', 'TH-58', 23),
    ('Maha Sarakham', 'TH-44', 24),
    ('Mukdahan', 'TH-49', 25),
    ('Nakhon Nayok', 'TH-26', 26),
    ('Nakhon Pathom', 'TH-73', 27),
    ('Nakhon Phanom', 'TH-48', 28),
    ('Nakhon Ratchasima', 'TH-30', 29),
    ('Nakhon Sawan', 'TH-60', 30),
    ('Nakhon Si Thammarat', 'TH-80', 31),
    ('Nan', 'TH-55', 32),
    ('Narathiwat', 'TH-96', 33),
    ('Nong Bua Lam Phu', 'TH-39', 34),
    ('Nong Khai', 'TH-43', 35),
    ('Nonthaburi', 'TH-12', 36),
    ('Pathum Thani', 'TH-13', 37),
    ('Pattani', 'TH-94', 38),
    ('Pattaya', 'TH-S', 39),
    ('Phangnga', 'TH-82', 40),
    ('Phatthalung', 'TH-93', 41),
    ('Phayao', 'TH-56', 42),
    ('Phetchabun', 'TH-67', 43),
    ('Phetchaburi', 'TH-76', 44),
    ('Phichit', 'TH-66', 45),
    ('Phitsanulok', 'TH-65', 46),
    ('Phra Nakhon Si Ayutthaya', 'TH-14', 47),
    ('Phrae', 'TH-54', 48),
    ('Phuket', 'TH-83', 49),
    ('Prachin Buri', 'TH-25', 50),
    ('Prachuap Khiri Khan', 'TH-77', 51),
    ('Ranong', 'TH-85', 52),
    ('Ratchaburi', 'TH-70', 53),
    ('Rayong', 'TH-21', 54),
    ('Roi Et', 'TH-45', 55),
    ('Sa Kaeo', 'TH-27', 56),
    ('Sakon Nakhon', 'TH-47', 57),
    ('Samut Prakan', 'TH-11', 58),
    ('Samut Sakhon', 'TH-74', 59),
    ('Samut Songkhram', 'TH-75', 60),
    ('Saraburi', 'TH-19', 61),
    ('Satun', 'TH-91', 62),
    ('Si Sa Ket', 'TH-33', 63),
    ('Sing Buri', 'TH-17', 64),
    ('Songkhla', 'TH-90', 65),
    ('Sukhothai', 'TH-64', 66),
    ('Suphan Buri', 'TH-72', 67),
    ('Surat Thani', 'TH-84', 68),
    ('Surin', 'TH-32', 69),
    ('Tak', 'TH-63', 70),
    ('Trang', 'TH-92', 71),
    ('Trat', 'TH-23', 72),
    ('Ubon Ratchathani', 'TH-34', 73),
    ('Udon Thani', 'TH-41', 74),
    ('Uthai Thani', 'TH-61', 75),
    ('Uttaradit', 'TH-53', 76),
    ('Yala', 'TH-95', 77),
    ('Yasothon', 'TH-35', 78)
) AS v(name, code, ord)
WHERE p.standard_code = 'TH' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 塔吉克斯坦（TJ）：5 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Dushanbe', 'TJ-DU', 1),
    ('Gorno-Badakhshan', 'TJ-GB', 2),
    ('Khatlon', 'TJ-KT', 3),
    ('Nohiyahoi Tobei Jumhurí', 'TJ-RA', 4),
    ('Sughd', 'TJ-SU', 5)
) AS v(name, code, ord)
WHERE p.standard_code = 'TJ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 汤加（TO）：5 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Haʻapai', 'TO-02', 1),
    ('ʻEua', 'TO-01', 2),
    ('Niuas', 'TO-03', 3),
    ('Tongatapu', 'TO-04', 4),
    ('Vavaʻu', 'TO-05', 5)
) AS v(name, code, ord)
WHERE p.standard_code = 'TO' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 坦桑尼亚（TZ）：31 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Arusha', 'TZ-01', 1),
    ('Dar es Salaam', 'TZ-02', 2),
    ('Dodoma', 'TZ-03', 3),
    ('Geita', 'TZ-27', 4),
    ('Iringa', 'TZ-04', 5),
    ('Kagera', 'TZ-05', 6),
    ('Katavi', 'TZ-28', 7),
    ('Kigoma', 'TZ-08', 8),
    ('Kilimanjaro', 'TZ-09', 9),
    ('Lindi', 'TZ-12', 10),
    ('Manyara', 'TZ-26', 11),
    ('Mara', 'TZ-13', 12),
    ('Mbeya', 'TZ-14', 13),
    ('Morogoro', 'TZ-16', 14),
    ('Mtwara', 'TZ-17', 15),
    ('Mwanza', 'TZ-18', 16),
    ('Njombe', 'TZ-29', 17),
    ('Pemba North', 'TZ-06', 18),
    ('Pemba South', 'TZ-10', 19),
    ('Pwani', 'TZ-19', 20),
    ('Rukwa', 'TZ-20', 21),
    ('Ruvuma', 'TZ-21', 22),
    ('Shinyanga', 'TZ-22', 23),
    ('Simiyu', 'TZ-30', 24),
    ('Singida', 'TZ-23', 25),
    ('Songwe', 'TZ-31', 26),
    ('Tabora', 'TZ-24', 27),
    ('Tanga', 'TZ-25', 28),
    ('Zanzibar North', 'TZ-07', 29),
    ('Zanzibar South', 'TZ-11', 30),
    ('Zanzibar West', 'TZ-15', 31)
) AS v(name, code, ord)
WHERE p.standard_code = 'TZ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 特克斯和凯科斯群岛（TC）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Grand Turk', 'TC-05', 1),
    ('Middle Caicos', 'TC-03', 2),
    ('North Caicos', 'TC-02', 3),
    ('Providenciales', 'TC-01', 4),
    ('Salt Cay', 'TC-06', 5),
    ('South Caicos', 'TC-04', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'TC' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 特立尼达和多巴哥（TT）：15 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Arima', 'TT-ARI', 1),
    ('Chaguanas', 'TT-CHA', 2),
    ('Couva-Tabaquite-Talparo', 'TT-CTT', 3),
    ('Diego Martin', 'TT-DMN', 4),
    ('Penal-Debe', 'TT-PED', 5),
    ('Point Fortin', 'TT-PTF', 6),
    ('Port of Spain', 'TT-POS', 7),
    ('Princes Town', 'TT-PRT', 8),
    ('Rio Claro-Mayaro', 'TT-MRC', 9),
    ('San Fernando', 'TT-SFO', 10),
    ('San Juan-Laventille', 'TT-SJL', 11),
    ('Sangre Grande', 'TT-SGE', 12),
    ('Siparia', 'TT-SIP', 13),
    ('Tobago', 'TT-TOB', 14),
    ('Tunapuna-Piarco', 'TT-TUP', 15)
) AS v(name, code, ord)
WHERE p.standard_code = 'TT' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 土耳其（TR）：81 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Adana', 'TR-01', 1),
    ('Adıyaman', 'TR-02', 2),
    ('Afyonkarahisar', 'TR-03', 3),
    ('Ağrı', 'TR-04', 4),
    ('Aksaray', 'TR-68', 5),
    ('Amasya', 'TR-05', 6),
    ('Ankara', 'TR-06', 7),
    ('Antalya', 'TR-07', 8),
    ('Ardahan', 'TR-75', 9),
    ('Artvin', 'TR-08', 10),
    ('Aydın', 'TR-09', 11),
    ('Balıkesir', 'TR-10', 12),
    ('Bartın', 'TR-74', 13),
    ('Batman', 'TR-72', 14),
    ('Bayburt', 'TR-69', 15),
    ('Bilecik', 'TR-11', 16),
    ('Bingöl', 'TR-12', 17),
    ('Bitlis', 'TR-13', 18),
    ('Bolu', 'TR-14', 19),
    ('Burdur', 'TR-15', 20),
    ('Bursa', 'TR-16', 21),
    ('Çanakkale', 'TR-17', 22),
    ('Çankırı', 'TR-18', 23),
    ('Çorum', 'TR-19', 24),
    ('Denizli', 'TR-20', 25),
    ('Diyarbakır', 'TR-21', 26),
    ('Düzce', 'TR-81', 27),
    ('Edirne', 'TR-22', 28),
    ('Elazığ', 'TR-23', 29),
    ('Erzincan', 'TR-24', 30),
    ('Erzurum', 'TR-25', 31),
    ('Eskişehir', 'TR-26', 32),
    ('Gaziantep', 'TR-27', 33),
    ('Giresun', 'TR-28', 34),
    ('Gümüşhane', 'TR-29', 35),
    ('Hakkâri', 'TR-30', 36),
    ('Hatay', 'TR-31', 37),
    ('Iğdır', 'TR-76', 38),
    ('Isparta', 'TR-32', 39),
    ('İstanbul', 'TR-34', 40),
    ('İzmir', 'TR-35', 41),
    ('Kahramanmaraş', 'TR-46', 42),
    ('Karabük', 'TR-78', 43),
    ('Karaman', 'TR-70', 44),
    ('Kars', 'TR-36', 45),
    ('Kastamonu', 'TR-37', 46),
    ('Kayseri', 'TR-38', 47),
    ('Kilis', 'TR-79', 48),
    ('Kırıkkale', 'TR-71', 49),
    ('Kırklareli', 'TR-39', 50),
    ('Kırşehir', 'TR-40', 51),
    ('Kocaeli', 'TR-41', 52),
    ('Konya', 'TR-42', 53),
    ('Kütahya', 'TR-43', 54),
    ('Malatya', 'TR-44', 55),
    ('Manisa', 'TR-45', 56),
    ('Mardin', 'TR-47', 57),
    ('Mersin', 'TR-33', 58),
    ('Muğla', 'TR-48', 59),
    ('Muş', 'TR-49', 60),
    ('Nevşehir', 'TR-50', 61),
    ('Niğde', 'TR-51', 62),
    ('Ordu', 'TR-52', 63),
    ('Osmaniye', 'TR-80', 64),
    ('Rize', 'TR-53', 65),
    ('Sakarya', 'TR-54', 66),
    ('Samsun', 'TR-55', 67),
    ('Şanlıurfa', 'TR-63', 68),
    ('Siirt', 'TR-56', 69),
    ('Sinop', 'TR-57', 70),
    ('Sivas', 'TR-58', 71),
    ('Şırnak', 'TR-73', 72),
    ('Tekirdağ', 'TR-59', 73),
    ('Tokat', 'TR-60', 74),
    ('Trabzon', 'TR-61', 75),
    ('Tunceli', 'TR-62', 76),
    ('Uşak', 'TR-64', 77),
    ('Van', 'TR-65', 78),
    ('Yalova', 'TR-77', 79),
    ('Yozgat', 'TR-66', 80),
    ('Zonguldak', 'TR-67', 81)
) AS v(name, code, ord)
WHERE p.standard_code = 'TR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 土库曼斯坦（TM）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ahal', 'TM-A', 1),
    ('Ashgabat', 'TM-S', 2),
    ('Balkan', 'TM-B', 3),
    ('Daşoguz', 'TM-D', 4),
    ('Lebap', 'TM-L', 5),
    ('Mary', 'TM-M', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'TM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 突尼斯（TN）：29 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ariana', 'TN-12', 1),
    ('Béja', 'TN-31', 2),
    ('Ben Arous', 'TN-13', 3),
    ('Bizerte', 'TN-23', 4),
    ('District 1', 'TN-D1', 5),
    ('District 2', 'TN-D2', 6),
    ('District 3', 'TN-D3', 7),
    ('District 4', 'TN-D4', 8),
    ('District 5', 'TN-D5', 9),
    ('Gabès', 'TN-81', 10),
    ('Gafsa', 'TN-71', 11),
    ('Jendouba', 'TN-32', 12),
    ('Kairouan', 'TN-41', 13),
    ('Kasserine', 'TN-42', 14),
    ('Kebili', 'TN-73', 15),
    ('Kef', 'TN-33', 16),
    ('Mahdia', 'TN-53', 17),
    ('Manouba', 'TN-14', 18),
    ('Medenine', 'TN-82', 19),
    ('Monastir', 'TN-52', 20),
    ('Nabeul', 'TN-21', 21),
    ('Sfax', 'TN-61', 22),
    ('Sidi Bouzid', 'TN-43', 23),
    ('Siliana', 'TN-34', 24),
    ('Sousse', 'TN-51', 25),
    ('Tataouine', 'TN-83', 26),
    ('Tozeur', 'TN-72', 27),
    ('Tunis', 'TN-11', 28),
    ('Zaghouan', 'TN-22', 29)
) AS v(name, code, ord)
WHERE p.standard_code = 'TN' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 图瓦卢（TV）：8 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Funafuti', 'TV-FUN', 1),
    ('Nanumanga', 'TV-NMG', 2),
    ('Nanumea', 'TV-NMA', 3),
    ('Niutao Island Council', 'TV-NIT', 4),
    ('Nui', 'TV-NUI', 5),
    ('Nukufetau', 'TV-NKF', 6),
    ('Nukulaelae', 'TV-NKL', 7),
    ('Vaitupu', 'TV-VAI', 8)
) AS v(name, code, ord)
WHERE p.standard_code = 'TV' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 瓦利斯和富图纳（WF）：3 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alo', 'WF-AL', 1),
    ('Sigave', 'WF-SG', 2),
    ('Uvea', 'WF-UV', 3)
) AS v(name, code, ord)
WHERE p.standard_code = 'WF' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 瓦努阿图（VU）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Malampa', 'VU-MAP', 1),
    ('Penama', 'VU-PAM', 2),
    ('Sanma', 'VU-SAM', 3),
    ('Shefa', 'VU-SEE', 4),
    ('Tafea', 'VU-TAE', 5),
    ('Torba', 'VU-TOB', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'VU' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 危地马拉（GT）：22 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alta Verapaz', 'GT-16', 1),
    ('Baja Verapaz', 'GT-15', 2),
    ('Chimaltenango', 'GT-04', 3),
    ('Chiquimula', 'GT-20', 4),
    ('El Progreso', 'GT-02', 5),
    ('Escuintla', 'GT-05', 6),
    ('Guatemala', 'GT-01', 7),
    ('Huehuetenango', 'GT-13', 8),
    ('Izabal', 'GT-18', 9),
    ('Jalapa', 'GT-21', 10),
    ('Jutiapa', 'GT-22', 11),
    ('Petén', 'GT-17', 12),
    ('Quetzaltenango', 'GT-09', 13),
    ('Quiché', 'GT-14', 14),
    ('Retalhuleu', 'GT-11', 15),
    ('Sacatepéquez', 'GT-03', 16),
    ('San Marcos', 'GT-12', 17),
    ('Santa Rosa', 'GT-06', 18),
    ('Sololá', 'GT-07', 19),
    ('Suchitepéquez', 'GT-10', 20),
    ('Totonicapán', 'GT-08', 21),
    ('Zacapa', 'GT-19', 22)
) AS v(name, code, ord)
WHERE p.standard_code = 'GT' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 委内瑞拉（VE）：25 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Amazonas', 'VE-Z', 1),
    ('Anzoátegui', 'VE-B', 2),
    ('Apure', 'VE-C', 3),
    ('Aragua', 'VE-D', 4),
    ('Barinas', 'VE-E', 5),
    ('Bolívar', 'VE-F', 6),
    ('Carabobo', 'VE-G', 7),
    ('Cojedes', 'VE-H', 8),
    ('Delta Amacuro', 'VE-Y', 9),
    ('Distrito Capital', 'VE-A', 10),
    ('Falcón', 'VE-I', 11),
    ('Federal Dependencies', 'VE-W', 12),
    ('Guárico', 'VE-J', 13),
    ('La Guaira', 'VE-X', 14),
    ('Lara', 'VE-K', 15),
    ('Mérida', 'VE-L', 16),
    ('Miranda', 'VE-M', 17),
    ('Monagas', 'VE-N', 18),
    ('Nueva Esparta', 'VE-O', 19),
    ('Portuguesa', 'VE-P', 20),
    ('Sucre', 'VE-R', 21),
    ('Táchira', 'VE-S', 22),
    ('Trujillo', 'VE-T', 23),
    ('Yaracuy', 'VE-U', 24),
    ('Zulia', 'VE-V', 25)
) AS v(name, code, ord)
WHERE p.standard_code = 'VE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 文莱（BN）：4 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Belait', 'BN-BE', 1),
    ('Brunei-Muara', 'BN-BM', 2),
    ('Temburong', 'BN-TE', 3),
    ('Tutong', 'BN-TU', 4)
) AS v(name, code, ord)
WHERE p.standard_code = 'BN' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 乌干达（UG）：139 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Abim', 'UG-314', 1),
    ('Adjumani', 'UG-301', 2),
    ('Agago', 'UG-322', 3),
    ('Alebtong', 'UG-323', 4),
    ('Amolatar', 'UG-315', 5),
    ('Amudat', 'UG-324', 6),
    ('Amuria', 'UG-216', 7),
    ('Amuru', 'UG-316', 8),
    ('Apac', 'UG-302', 9),
    ('Arua', 'UG-303', 10),
    ('Budaka', 'UG-217', 11),
    ('Bududa', 'UG-218', 12),
    ('Bugiri', 'UG-201', 13),
    ('Bugweri', 'UG-235', 14),
    ('Buhweju', 'UG-420', 15),
    ('Buikwe', 'UG-117', 16),
    ('Bukedea', 'UG-219', 17),
    ('Bukomansimbi', 'UG-118', 18),
    ('Bukwo', 'UG-220', 19),
    ('Bulambuli', 'UG-225', 20),
    ('Buliisa', 'UG-416', 21),
    ('Bundibugyo', 'UG-401', 22),
    ('Bunyangabu', 'UG-430', 23),
    ('Bushenyi', 'UG-402', 24),
    ('Busia', 'UG-202', 25),
    ('Butaleja', 'UG-221', 26),
    ('Butambala', 'UG-119', 27),
    ('Butebo', 'UG-233', 28),
    ('Buvuma', 'UG-120', 29),
    ('Buyende', 'UG-226', 30),
    ('Central', 'UG-C', 31),
    ('Dokolo', 'UG-317', 32),
    ('Eastern', 'UG-E', 33),
    ('Gomba', 'UG-121', 34),
    ('Gulu', 'UG-304', 35),
    ('Hoima', 'UG-403', 36),
    ('Ibanda', 'UG-417', 37),
    ('Iganga', 'UG-203', 38),
    ('Isingiro', 'UG-418', 39),
    ('Jinja', 'UG-204', 40),
    ('Kaabong', 'UG-318', 41),
    ('Kabale', 'UG-404', 42),
    ('Kabarole', 'UG-405', 43),
    ('Kaberamaido', 'UG-213', 44),
    ('Kagadi', 'UG-427', 45),
    ('Kakumiro', 'UG-428', 46),
    ('Kalaki', 'UG-237', 47),
    ('Kalangala', 'UG-101', 48),
    ('Kaliro', 'UG-222', 49),
    ('Kalungu', 'UG-122', 50),
    ('Kampala', 'UG-102', 51),
    ('Kamuli', 'UG-205', 52),
    ('Kamwenge', 'UG-413', 53),
    ('Kanungu', 'UG-414', 54),
    ('Kapchorwa', 'UG-206', 55),
    ('Kapelebyong', 'UG-236', 56),
    ('Karenga', 'UG-335', 57),
    ('Kasanda', 'UG-126', 58),
    ('Kasese', 'UG-406', 59),
    ('Katakwi', 'UG-207', 60),
    ('Kayunga', 'UG-112', 61),
    ('Kazo', 'UG-433', 62),
    ('Kibaale', 'UG-407', 63),
    ('Kiboga', 'UG-103', 64),
    ('Kibuku', 'UG-227', 65),
    ('Kikuube', 'UG-432', 66),
    ('Kiruhura', 'UG-419', 67),
    ('Kiryandongo', 'UG-421', 68),
    ('Kisoro', 'UG-408', 69),
    ('Kitagwenda', 'UG-434', 70),
    ('Kitgum', 'UG-305', 71),
    ('Koboko', 'UG-319', 72),
    ('Kole', 'UG-325', 73),
    ('Kotido', 'UG-306', 74),
    ('Kumi', 'UG-208', 75),
    ('Kwania', 'UG-333', 76),
    ('Kween', 'UG-228', 77),
    ('Kyankwanzi', 'UG-123', 78),
    ('Kyegegwa', 'UG-422', 79),
    ('Kyenjojo', 'UG-415', 80),
    ('Kyotera', 'UG-125', 81),
    ('Lamwo', 'UG-326', 82),
    ('Lira', 'UG-307', 83),
    ('Luuka', 'UG-229', 84),
    ('Luwero', 'UG-104', 85),
    ('Lwengo', 'UG-124', 86),
    ('Lyantonde', 'UG-114', 87),
    ('Madi-Okollo', 'UG-336', 88),
    ('Manafwa', 'UG-223', 89),
    ('Maracha', 'UG-320', 90),
    ('Masaka', 'UG-105', 91),
    ('Masindi', 'UG-409', 92),
    ('Mayuge', 'UG-214', 93),
    ('Mbale', 'UG-209', 94),
    ('Mbarara', 'UG-410', 95),
    ('Mitooma', 'UG-423', 96),
    ('Mityana', 'UG-115', 97),
    ('Moroto', 'UG-308', 98),
    ('Moyo', 'UG-309', 99),
    ('Mpigi', 'UG-106', 100),
    ('Mubende', 'UG-107', 101),
    ('Mukono', 'UG-108', 102),
    ('Nabilatuk', 'UG-334', 103),
    ('Nakapiripirit', 'UG-311', 104),
    ('Nakaseke', 'UG-116', 105),
    ('Nakasongola', 'UG-109', 106),
    ('Namayingo', 'UG-230', 107),
    ('Namisindwa', 'UG-234', 108),
    ('Namutumba', 'UG-224', 109),
    ('Napak', 'UG-327', 110),
    ('Nebbi', 'UG-310', 111),
    ('Ngora', 'UG-231', 112),
    ('Northern', 'UG-N', 113),
    ('Ntoroko', 'UG-424', 114),
    ('Ntungamo', 'UG-411', 115),
    ('Nwoya', 'UG-328', 116),
    ('Obongi', 'UG-337', 117),
    ('Omoro', 'UG-331', 118),
    ('Otuke', 'UG-329', 119),
    ('Oyam', 'UG-321', 120),
    ('Pader', 'UG-312', 121),
    ('Pakwach', 'UG-332', 122),
    ('Pallisa', 'UG-210', 123),
    ('Rakai', 'UG-110', 124),
    ('Rubanda', 'UG-429', 125),
    ('Rubirizi', 'UG-425', 126),
    ('Rukiga', 'UG-431', 127),
    ('Rukungiri', 'UG-412', 128),
    ('Rwampara', 'UG-435', 129),
    ('Sembabule', 'UG-111', 130),
    ('Serere', 'UG-232', 131),
    ('Sheema', 'UG-426', 132),
    ('Sironko', 'UG-215', 133),
    ('Soroti', 'UG-211', 134),
    ('Tororo', 'UG-212', 135),
    ('Wakiso', 'UG-113', 136),
    ('Western', 'UG-W', 137),
    ('Yumbe', 'UG-313', 138),
    ('Zombo', 'UG-330', 139)
) AS v(name, code, ord)
WHERE p.standard_code = 'UG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 乌克兰（UA）：27 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Autonomous Republic of Crimea', 'UA-43', 1),
    ('Cherkaska', 'UA-71', 2),
    ('Chernihivska', 'UA-74', 3),
    ('Chernivetska', 'UA-77', 4),
    ('Dnipropetrovska', 'UA-12', 5),
    ('Donetska', 'UA-14', 6),
    ('Ivano-Frankivska', 'UA-26', 7),
    ('Kharkivska', 'UA-63', 8),
    ('Khersonska', 'UA-65', 9),
    ('Khmelnytska', 'UA-68', 10),
    ('Kirovohradska', 'UA-35', 11),
    ('Kyiv', 'UA-30', 12),
    ('Kyivska', 'UA-32', 13),
    ('Luhanska', 'UA-09', 14),
    ('Lvivska', 'UA-46', 15),
    ('Mykolaivska', 'UA-48', 16),
    ('Odeska', 'UA-51', 17),
    ('Poltavska', 'UA-53', 18),
    ('Rivnenska', 'UA-56', 19),
    ('Sevastopol', 'UA-40', 20),
    ('Sumska', 'UA-59', 21),
    ('Ternopilska', 'UA-61', 22),
    ('Vinnytska', 'UA-05', 23),
    ('Volynska', 'UA-07', 24),
    ('Zakarpatska', 'UA-21', 25),
    ('Zaporizka', 'UA-23', 26),
    ('Zhytomyrska', 'UA-18', 27)
) AS v(name, code, ord)
WHERE p.standard_code = 'UA' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 乌拉圭（UY）：19 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Artigas', 'UY-AR', 1),
    ('Canelones', 'UY-CA', 2),
    ('Cerro Largo', 'UY-CL', 3),
    ('Colonia', 'UY-CO', 4),
    ('Durazno', 'UY-DU', 5),
    ('Flores', 'UY-FS', 6),
    ('Florida', 'UY-FD', 7),
    ('Lavalleja', 'UY-LA', 8),
    ('Maldonado', 'UY-MA', 9),
    ('Montevideo', 'UY-MO', 10),
    ('Paysandú', 'UY-PA', 11),
    ('Río Negro', 'UY-RN', 12),
    ('Rivera', 'UY-RV', 13),
    ('Rocha', 'UY-RO', 14),
    ('Salto', 'UY-SA', 15),
    ('San José', 'UY-SJ', 16),
    ('Soriano', 'UY-SO', 17),
    ('Tacuarembó', 'UY-TA', 18),
    ('Treinta y Tres', 'UY-TT', 19)
) AS v(name, code, ord)
WHERE p.standard_code = 'UY' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 乌兹别克斯坦（UZ）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Andijan', 'UZ-AN', 1),
    ('Bukhara', 'UZ-BU', 2),
    ('Fergana', 'UZ-FA', 3),
    ('Jizzakh', 'UZ-JI', 4),
    ('Karakalpakstan', 'UZ-QR', 5),
    ('Namangan', 'UZ-NG', 6),
    ('Navoiy', 'UZ-NW', 7),
    ('Qashqadaryo', 'UZ-QA', 8),
    ('Samarqand', 'UZ-SA', 9),
    ('Sirdaryo', 'UZ-SI', 10),
    ('Surxondaryo', 'UZ-SU', 11),
    ('Tashkent', 'UZ-TK', 12),
    ('Tashkent', 'UZ-TO', 13),
    ('Xorazm', 'UZ-XO', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'UZ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 西班牙（ES）：69 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('A Coruña', 'ES-C', 1),
    ('Albacete', 'ES-AB', 2),
    ('Alicante', 'ES-A', 3),
    ('Almeria', 'ES-AL', 4),
    ('Andalusia', 'ES-AN', 5),
    ('Araba', 'ES-VI', 6),
    ('Aragon', 'ES-AR', 7),
    ('Asturias', 'ES-O', 8),
    ('Asturias, Principality of', 'ES-AS', 9),
    ('Ávila', 'ES-AV', 10),
    ('Badajoz', 'ES-BA', 11),
    ('Balearic Islands', 'ES-IB', 12),
    ('Barcelona', 'ES-B', 13),
    ('Basque Country', 'ES-PV', 14),
    ('Bizkaia', 'ES-BI', 15),
    ('Burgos', 'ES-BU', 16),
    ('Caceres', 'ES-CC', 17),
    ('Cádiz', 'ES-CA', 18),
    ('Canary Islands', 'ES-CN', 19),
    ('Cantabria', 'ES-S', 20),
    ('Cantabria', 'ES-CB', 21),
    ('Castellón', 'ES-CS', 22),
    ('Castile and Leon', 'ES-CL', 23),
    ('Castilla-La Mancha', 'ES-CM', 24),
    ('Catalonia', 'ES-CT', 25),
    ('Ceuta', 'ES-CE', 26),
    ('Ciudad Real', 'ES-CR', 27),
    ('Community of Madrid', 'ES-MD', 28),
    ('Córdoba', 'ES-CO', 29),
    ('Cuenca', 'ES-CU', 30),
    ('Extremadura', 'ES-EX', 31),
    ('Galicia', 'ES-GA', 32),
    ('Gipuzkoa', 'ES-SS', 33),
    ('Girona', 'ES-GI', 34),
    ('Granada', 'ES-GR', 35),
    ('Guadalajara', 'ES-GU', 36),
    ('Huelva', 'ES-H', 37),
    ('Huesca', 'ES-HU', 38),
    ('Islas Baleares', 'ES-PM', 39),
    ('Jaén', 'ES-J', 40),
    ('La Rioja', 'ES-RI', 41),
    ('La Rioja', 'ES-LO', 42),
    ('Las Palmas', 'ES-GC', 43),
    ('León', 'ES-LE', 44),
    ('Lleida', 'ES-L', 45),
    ('Lugo', 'ES-LU', 46),
    ('Madrid', 'ES-M', 47),
    ('Málaga', 'ES-MA', 48),
    ('Melilla', 'ES-ML', 49),
    ('Murcia', 'ES-MU', 50),
    ('Navarra', 'ES-NA', 51),
    ('Navarre', 'ES-NC', 52),
    ('Ourense', 'ES-OR', 53),
    ('Palencia', 'ES-P', 54),
    ('Pontevedra', 'ES-PO', 55),
    ('Region of Murcia', 'ES-MC', 56),
    ('Salamanca', 'ES-SA', 57),
    ('Santa Cruz de Tenerife', 'ES-TF', 58),
    ('Segovia', 'ES-SG', 59),
    ('Sevilla', 'ES-SE', 60),
    ('Soria', 'ES-SO', 61),
    ('Tarragona', 'ES-T', 62),
    ('Teruel', 'ES-TE', 63),
    ('Toledo', 'ES-TO', 64),
    ('Valencia', 'ES-V', 65),
    ('Valencian Community', 'ES-VC', 66),
    ('Valladolid', 'ES-VA', 67),
    ('Zamora', 'ES-ZA', 68),
    ('Zaragoza', 'ES-Z', 69)
) AS v(name, code, ord)
WHERE p.standard_code = 'ES' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 希腊（GR）：15 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Attica', 'GR-I', 1),
    ('Central Greece', 'GR-H', 2),
    ('Central Macedonia', 'GR-B', 3),
    ('Crete', 'GR-M', 4),
    ('East Attica', 'GR-A2', 5),
    ('East Macedonia and Thrace', 'GR-A', 6),
    ('Epirus', 'GR-D', 7),
    ('Ionian Islands', 'GR-F', 8),
    ('Mount Athos', 'GR-69', 9),
    ('North Aegean', 'GR-K', 10),
    ('Peloponnese', 'GR-J', 11),
    ('South Aegean', 'GR-L', 12),
    ('Thessaly', 'GR-E', 13),
    ('West Greece', 'GR-G', 14),
    ('West Macedonia', 'GR-C', 15)
) AS v(name, code, ord)
WHERE p.standard_code = 'GR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 新加坡（SG）：5 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Central Singapore', 'SG-01', 1),
    ('North East', 'SG-02', 2),
    ('North West', 'SG-03', 3),
    ('South East', 'SG-04', 4),
    ('South West', 'SG-05', 5)
) AS v(name, code, ord)
WHERE p.standard_code = 'SG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 新喀里多尼亚（NC）：3 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Loyalty Islands Province', 'NC-03', 1),
    ('North Province', 'NC-02', 2),
    ('South Province', 'NC-01', 3)
) AS v(name, code, ord)
WHERE p.standard_code = 'NC' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 新西兰（NZ）：17 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Auckland', 'NZ-AUK', 1),
    ('Bay of Plenty', 'NZ-BOP', 2),
    ('Canterbury', 'NZ-CAN', 3),
    ('Chatham Islands', 'NZ-CIT', 4),
    ('Gisborne', 'NZ-GIS', 5),
    ('Hawke''s Bay', 'NZ-HKB', 6),
    ('Manawatu-Whanganui', 'NZ-MWT', 7),
    ('Marlborough', 'NZ-MBH', 8),
    ('Nelson', 'NZ-NSN', 9),
    ('Northland', 'NZ-NTL', 10),
    ('Otago', 'NZ-OTA', 11),
    ('Southland', 'NZ-STL', 12),
    ('Taranaki', 'NZ-TKI', 13),
    ('Tasman', 'NZ-TAS', 14),
    ('Waikato', 'NZ-WKO', 15),
    ('Wellington', 'NZ-WGN', 16),
    ('West Coast', 'NZ-WTC', 17)
) AS v(name, code, ord)
WHERE p.standard_code = 'NZ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 匈牙利（HU）：43 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bács-Kiskun', 'HU-BK', 1),
    ('Baranya', 'HU-BA', 2),
    ('Békés', 'HU-BE', 3),
    ('Békéscsaba', 'HU-BC', 4),
    ('Borsod-Abaúj-Zemplén', 'HU-BZ', 5),
    ('Budapest', 'HU-BU', 6),
    ('Csongrád County', 'HU-CS', 7),
    ('Debrecen', 'HU-DE', 8),
    ('Dunaújváros', 'HU-DU', 9),
    ('Eger', 'HU-EG', 10),
    ('Érd', 'HU-ER', 11),
    ('Fejér County', 'HU-FE', 12),
    ('Győr', 'HU-GY', 13),
    ('Győr-Moson-Sopron County', 'HU-GS', 14),
    ('Hajdú-Bihar County', 'HU-HB', 15),
    ('Heves County', 'HU-HE', 16),
    ('Hódmezővásárhely', 'HU-HV', 17),
    ('Jász-Nagykun-Szolnok County', 'HU-JN', 18),
    ('Kaposvár', 'HU-KV', 19),
    ('Kecskemét', 'HU-KM', 20),
    ('Komárom-Esztergom', 'HU-KE', 21),
    ('Miskolc', 'HU-MI', 22),
    ('Nagykanizsa', 'HU-NK', 23),
    ('Nógrád County', 'HU-NO', 24),
    ('Nyíregyháza', 'HU-NY', 25),
    ('Pécs', 'HU-PS', 26),
    ('Pest County', 'HU-PE', 27),
    ('Salgótarján', 'HU-ST', 28),
    ('Somogy County', 'HU-SO', 29),
    ('Sopron', 'HU-SN', 30),
    ('Szabolcs-Szatmár-Bereg County', 'HU-SZ', 31),
    ('Szeged', 'HU-SD', 32),
    ('Székesfehérvár', 'HU-SF', 33),
    ('Szekszárd', 'HU-SS', 34),
    ('Szolnok', 'HU-SK', 35),
    ('Szombathely', 'HU-SH', 36),
    ('Tatabánya', 'HU-TB', 37),
    ('Tolna County', 'HU-TO', 38),
    ('Vas County', 'HU-VA', 39),
    ('Veszprém', 'HU-VM', 40),
    ('Veszprém County', 'HU-VE', 41),
    ('Zala County', 'HU-ZA', 42),
    ('Zalaegerszeg', 'HU-ZE', 43)
) AS v(name, code, ord)
WHERE p.standard_code = 'HU' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 叙利亚（SY）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Al-Hasakah', 'SY-HA', 1),
    ('Al-Raqqah', 'SY-RA', 2),
    ('Aleppo', 'SY-HL', 3),
    ('As-Suwayda', 'SY-SU', 4),
    ('Damascus', 'SY-DI', 5),
    ('Daraa', 'SY-DR', 6),
    ('Deir ez-Zor', 'SY-DY', 7),
    ('Hama', 'SY-HM', 8),
    ('Homs', 'SY-HI', 9),
    ('Idlib', 'SY-ID', 10),
    ('Latakia', 'SY-LA', 11),
    ('Quneitra', 'SY-QU', 12),
    ('Rif Dimashq', 'SY-RD', 13),
    ('Tartus', 'SY-TA', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'SY' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 牙买加（JM）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Clarendon', 'JM-13', 1),
    ('Hanover', 'JM-09', 2),
    ('Kingston', 'JM-01', 3),
    ('Manchester', 'JM-12', 4),
    ('Portland', 'JM-04', 5),
    ('Saint Andrew', 'JM-02', 6),
    ('Saint Ann', 'JM-06', 7),
    ('Saint Catherine', 'JM-14', 8),
    ('Saint Elizabeth', 'JM-11', 9),
    ('Saint James', 'JM-08', 10),
    ('Saint Mary', 'JM-05', 11),
    ('Saint Thomas', 'JM-03', 12),
    ('Trelawny', 'JM-07', 13),
    ('Westmoreland', 'JM-10', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'JM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 亚美尼亚（AM）：11 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Aragatsotn', 'AM-AG', 1),
    ('Ararat', 'AM-AR', 2),
    ('Armavir', 'AM-AV', 3),
    ('Gegharkunik', 'AM-GR', 4),
    ('Kotayk', 'AM-KT', 5),
    ('Lori', 'AM-LO', 6),
    ('Shirak', 'AM-SH', 7),
    ('Syunik', 'AM-SU', 8),
    ('Tavush', 'AM-TV', 9),
    ('Vayots Dzor', 'AM-VD', 10),
    ('Yerevan', 'AM-ER', 11)
) AS v(name, code, ord)
WHERE p.standard_code = 'AM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 也门（YE）：22 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Abyan', 'YE-AB', 1),
    ('Ad Dali''', 'YE-DA', 2),
    ('Adan', 'YE-AD', 3),
    ('Al Bayda''', 'YE-BA', 4),
    ('Al Hudaydah', 'YE-HU', 5),
    ('Al Jawf', 'YE-JA', 6),
    ('Al Mahrah', 'YE-MR', 7),
    ('Al Mahwit', 'YE-MW', 8),
    ('Amanat Al Asimah', 'YE-SA', 9),
    ('Amran', 'YE-AM', 10),
    ('Dhamar', 'YE-DH', 11),
    ('Hadhramaut', 'YE-HD', 12),
    ('Hajjah', 'YE-HJ', 13),
    ('Ibb', 'YE-IB', 14),
    ('Lahij', 'YE-LA', 15),
    ('Ma''rib', 'YE-MA', 16),
    ('Raymah', 'YE-RA', 17),
    ('Saada', 'YE-SD', 18),
    ('Sana''a', 'YE-SN', 19),
    ('Shabwah', 'YE-SH', 20),
    ('Socotra', 'YE-SU', 21),
    ('Ta''izz', 'YE-TA', 22)
) AS v(name, code, ord)
WHERE p.standard_code = 'YE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 意大利（IT）：129 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Abruzzo', 'IT-65', 1),
    ('Agrigento', 'IT-AG', 2),
    ('Alessandria', 'IT-AL', 3),
    ('Ancona', 'IT-AN', 4),
    ('Aosta Valley', 'IT-23', 5),
    ('Apulia', 'IT-75', 6),
    ('Arezzo', 'IT-AR', 7),
    ('Ascoli Piceno', 'IT-AP', 8),
    ('Asti', 'IT-AT', 9),
    ('Avellino', 'IT-AV', 10),
    ('Bari', 'IT-BA', 11),
    ('Barletta-Andria-Trani', 'IT-BT', 12),
    ('Basilicata', 'IT-77', 13),
    ('Belluno', 'IT-BL', 14),
    ('Benevento', 'IT-BN', 15),
    ('Bergamo', 'IT-BG', 16),
    ('Biella', 'IT-BI', 17),
    ('Bologna', 'IT-BO', 18),
    ('Brescia', 'IT-BS', 19),
    ('Brindisi', 'IT-BR', 20),
    ('Cagliari', 'IT-CA', 21),
    ('Calabria', 'IT-78', 22),
    ('Caltanissetta', 'IT-CL', 23),
    ('Campania', 'IT-72', 24),
    ('Campobasso', 'IT-CB', 25),
    ('Caserta', 'IT-CE', 26),
    ('Catania', 'IT-CT', 27),
    ('Catanzaro', 'IT-CZ', 28),
    ('Chieti', 'IT-CH', 29),
    ('Como', 'IT-CO', 30),
    ('Cosenza', 'IT-CS', 31),
    ('Cremona', 'IT-CR', 32),
    ('Crotone', 'IT-KR', 33),
    ('Cuneo', 'IT-CN', 34),
    ('Emilia-Romagna', 'IT-45', 35),
    ('Enna', 'IT-EN', 36),
    ('Fermo', 'IT-FM', 37),
    ('Ferrara', 'IT-FE', 38),
    ('Florence', 'IT-FI', 39),
    ('Foggia', 'IT-FG', 40),
    ('Forlì-Cesena', 'IT-FC', 41),
    ('Friuli–Venezia Giulia', 'IT-36', 42),
    ('Frosinone', 'IT-FR', 43),
    ('Gallura Nord-Est Sardegna', 'IT-OT', 44),
    ('Genoa', 'IT-GE', 45),
    ('Gorizia', 'IT-GO', 46),
    ('Grosseto', 'IT-GR', 47),
    ('Imperia', 'IT-IM', 48),
    ('Isernia', 'IT-IS', 49),
    ('L''Aquila', 'IT-AQ', 50),
    ('La Spezia', 'IT-SP', 51),
    ('Latina', 'IT-LT', 52),
    ('Lazio', 'IT-62', 53),
    ('Lecce', 'IT-LE', 54),
    ('Lecco', 'IT-LC', 55),
    ('Liguria', 'IT-42', 56),
    ('Livorno', 'IT-LI', 57),
    ('Lodi', 'IT-LO', 58),
    ('Lombardy', 'IT-25', 59),
    ('Lucca', 'IT-LU', 60),
    ('Macerata', 'IT-MC', 61),
    ('Mantua', 'IT-MN', 62),
    ('Marche', 'IT-57', 63),
    ('Massa and Carrara', 'IT-MS', 64),
    ('Matera', 'IT-MT', 65),
    ('Medio Campidano', 'IT-VS', 66),
    ('Messina', 'IT-ME', 67),
    ('Milan', 'IT-MI', 68),
    ('Modena', 'IT-MO', 69),
    ('Molise', 'IT-67', 70),
    ('Monza and Brianza', 'IT-MB', 71),
    ('Naples', 'IT-NA', 72),
    ('Novara', 'IT-NO', 73),
    ('Nuoro', 'IT-NU', 74),
    ('Ogliastra', 'IT-OG', 75),
    ('Oristano', 'IT-OR', 76),
    ('Padua', 'IT-PD', 77),
    ('Palermo', 'IT-PA', 78),
    ('Parma', 'IT-PR', 79),
    ('Pavia', 'IT-PV', 80),
    ('Perugia', 'IT-PG', 81),
    ('Pesaro and Urbino', 'IT-PU', 82),
    ('Pescara', 'IT-PE', 83),
    ('Piacenza', 'IT-PC', 84),
    ('Piedmont', 'IT-21', 85),
    ('Pisa', 'IT-PI', 86),
    ('Pistoia', 'IT-PT', 87),
    ('Pordenone', 'IT-PN', 88),
    ('Potenza', 'IT-PZ', 89),
    ('Prato', 'IT-PO', 90),
    ('Ragusa', 'IT-RG', 91),
    ('Ravenna', 'IT-RA', 92),
    ('Reggio Calabria', 'IT-RC', 93),
    ('Reggio Emilia', 'IT-RE', 94),
    ('Rieti', 'IT-RI', 95),
    ('Rimini', 'IT-RN', 96),
    ('Rome', 'IT-RM', 97),
    ('Rovigo', 'IT-RO', 98),
    ('Salerno', 'IT-SA', 99),
    ('Sardinia', 'IT-88', 100),
    ('Sassari', 'IT-SS', 101),
    ('Savona', 'IT-SV', 102),
    ('Sicily', 'IT-82', 103),
    ('Siena', 'IT-SI', 104),
    ('Siracusa', 'IT-SR', 105),
    ('Sondrio', 'IT-SO', 106),
    ('South Tyrol', 'IT-BZ', 107),
    ('Sulcis Iglesiente', 'IT-CI', 108),
    ('Taranto', 'IT-TA', 109),
    ('Teramo', 'IT-TE', 110),
    ('Terni', 'IT-TR', 111),
    ('Trapani', 'IT-TP', 112),
    ('Trentino', 'IT-TN', 113),
    ('Trentino-South Tyrol', 'IT-32', 114),
    ('Treviso', 'IT-TV', 115),
    ('Trieste', 'IT-TS', 116),
    ('Turin', 'IT-TO', 117),
    ('Tuscany', 'IT-52', 118),
    ('Udine', 'IT-UD', 119),
    ('Umbria', 'IT-55', 120),
    ('Varese', 'IT-VA', 121),
    ('Veneto', 'IT-34', 122),
    ('Venice', 'IT-VE', 123),
    ('Verbano-Cusio-Ossola', 'IT-VB', 124),
    ('Vercelli', 'IT-VC', 125),
    ('Verona', 'IT-VR', 126),
    ('Vibo Valentia', 'IT-VV', 127),
    ('Vicenza', 'IT-VI', 128),
    ('Viterbo', 'IT-VT', 129)
) AS v(name, code, ord)
WHERE p.standard_code = 'IT' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 伊拉克（IQ）：20 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Al Anbar', 'IQ-AN', 1),
    ('Al Muthanna', 'IQ-MU', 2),
    ('Al-Qādisiyyah', 'IQ-QA', 3),
    ('Babylon', 'IQ-BB', 4),
    ('Baghdad', 'IQ-BG', 5),
    ('Basra', 'IQ-BA', 6),
    ('Dhi Qar', 'IQ-DQ', 7),
    ('Diyala', 'IQ-DI', 8),
    ('Dohuk', 'IQ-DA', 9),
    ('Erbil', 'IQ-AR', 10),
    ('Halabja', 'IQ-HA', 11),
    ('Iqlim Kurdistan', 'IQ-KR', 12),
    ('Karbala', 'IQ-KA', 13),
    ('Kirkuk', 'IQ-KI', 14),
    ('Maysan', 'IQ-MA', 15),
    ('Najaf', 'IQ-NA', 16),
    ('Nineveh', 'IQ-NI', 17),
    ('Saladin', 'IQ-SD', 18),
    ('Sulaymaniyah', 'IQ-SU', 19),
    ('Wasit', 'IQ-WA', 20)
) AS v(name, code, ord)
WHERE p.standard_code = 'IQ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 伊朗（IR）：31 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alborz', 'IR-30', 1),
    ('Ardabil', 'IR-24', 2),
    ('Bushehr', 'IR-18', 3),
    ('Chaharmahal and Bakhtiari', 'IR-14', 4),
    ('East Azerbaijan', 'IR-03', 5),
    ('Fars', 'IR-07', 6),
    ('Gilan', 'IR-01', 7),
    ('Golestan', 'IR-27', 8),
    ('Hamadan', 'IR-13', 9),
    ('Hormozgan', 'IR-22', 10),
    ('Ilam', 'IR-16', 11),
    ('Isfahan', 'IR-10', 12),
    ('Kerman', 'IR-08', 13),
    ('Kermanshah', 'IR-05', 14),
    ('Khuzestan', 'IR-06', 15),
    ('Kohgiluyeh and Boyer-Ahmad', 'IR-17', 16),
    ('Kurdistan', 'IR-12', 17),
    ('Lorestan', 'IR-15', 18),
    ('Markazi', 'IR-00', 19),
    ('Mazandaran', 'IR-02', 20),
    ('North Khorasan', 'IR-28', 21),
    ('Qazvin', 'IR-26', 22),
    ('Qom', 'IR-25', 23),
    ('Razavi Khorasan', 'IR-09', 24),
    ('Semnan', 'IR-20', 25),
    ('Sistan and Baluchestan', 'IR-11', 26),
    ('South Khorasan', 'IR-29', 27),
    ('Tehran', 'IR-23', 28),
    ('West Azarbaijan', 'IR-04', 29),
    ('Yazd', 'IR-21', 30),
    ('Zanjan', 'IR-19', 31)
) AS v(name, code, ord)
WHERE p.standard_code = 'IR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 印度（IN）：36 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Andaman and Nicobar Islands', 'IN-AN', 1),
    ('Andhra Pradesh', 'IN-AP', 2),
    ('Arunachal Pradesh', 'IN-AR', 3),
    ('Assam', 'IN-AS', 4),
    ('Bihar', 'IN-BR', 5),
    ('Chandigarh', 'IN-CH', 6),
    ('Chhattisgarh', 'IN-CG', 7),
    ('Dadra and Nagar Haveli and Daman and Diu', 'IN-DH', 8),
    ('Delhi', 'IN-DL', 9),
    ('Goa', 'IN-GA', 10),
    ('Gujarat', 'IN-GJ', 11),
    ('Haryana', 'IN-HR', 12),
    ('Himachal Pradesh', 'IN-HP', 13),
    ('Jammu and Kashmir', 'IN-JK', 14),
    ('Jharkhand', 'IN-JH', 15),
    ('Karnataka', 'IN-KA', 16),
    ('Kerala', 'IN-KL', 17),
    ('Ladakh', 'IN-LA', 18),
    ('Lakshadweep', 'IN-LD', 19),
    ('Madhya Pradesh', 'IN-MP', 20),
    ('Maharashtra', 'IN-MH', 21),
    ('Manipur', 'IN-MN', 22),
    ('Meghalaya', 'IN-ML', 23),
    ('Mizoram', 'IN-MZ', 24),
    ('Nagaland', 'IN-NL', 25),
    ('Odisha', 'IN-OD', 26),
    ('Puducherry', 'IN-PY', 27),
    ('Punjab', 'IN-PB', 28),
    ('Rajasthan', 'IN-RJ', 29),
    ('Sikkim', 'IN-SK', 30),
    ('Tamil Nadu', 'IN-TN', 31),
    ('Telangana', 'IN-TS', 32),
    ('Tripura', 'IN-TR', 33),
    ('Uttar Pradesh', 'IN-UP', 34),
    ('Uttarakhand', 'IN-UK', 35),
    ('West Bengal', 'IN-WB', 36)
) AS v(name, code, ord)
WHERE p.standard_code = 'IN' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 印度尼西亚（ID）：45 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Aceh', 'ID-AC', 1),
    ('Bali', 'ID-BA', 2),
    ('Banten', 'ID-BT', 3),
    ('Bengkulu', 'ID-BE', 4),
    ('DI Yogyakarta', 'ID-YO', 5),
    ('DKI Jakarta', 'ID-JK', 6),
    ('Gorontalo', 'ID-GO', 7),
    ('Jambi', 'ID-JA', 8),
    ('Jawa', 'ID-JW', 9),
    ('Jawa Barat', 'ID-JB', 10),
    ('Jawa Tengah', 'ID-JT', 11),
    ('Jawa Timur', 'ID-JI', 12),
    ('Kalimantan', 'ID-KA', 13),
    ('Kalimantan Barat', 'ID-KB', 14),
    ('Kalimantan Selatan', 'ID-KS', 15),
    ('Kalimantan Tengah', 'ID-KT', 16),
    ('Kalimantan Timur', 'ID-KI', 17),
    ('Kalimantan Utara', 'ID-KU', 18),
    ('Kepulauan Bangka Belitung', 'ID-BB', 19),
    ('Kepulauan Riau', 'ID-KR', 20),
    ('Lampung', 'ID-LA', 21),
    ('Maluku', 'ID-ML', 22),
    ('Maluku', 'ID-MA', 23),
    ('Maluku Utara', 'ID-MU', 24),
    ('Nusa Tenggara', 'ID-NU', 25),
    ('Nusa Tenggara Barat', 'ID-NB', 26),
    ('Nusa Tenggara Timur', 'ID-NT', 27),
    ('Papua', 'ID-PA', 28),
    ('Papua', 'ID-PP', 29),
    ('Papua Barat', 'ID-PB', 30),
    ('Papua Barat Daya', 'ID-PD', 31),
    ('Papua Pegunungan', 'ID-PE', 32),
    ('Papua Selatan', 'ID-PS', 33),
    ('Papua Tengah', 'ID-PT', 34),
    ('Riau', 'ID-RI', 35),
    ('Sulawesi', 'ID-SL', 36),
    ('Sulawesi Barat', 'ID-SR', 37),
    ('Sulawesi Selatan', 'ID-SN', 38),
    ('Sulawesi Tengah', 'ID-ST', 39),
    ('Sulawesi Tenggara', 'ID-SG', 40),
    ('Sulawesi Utara', 'ID-SA', 41),
    ('Sumatera', 'ID-SM', 42),
    ('Sumatera Barat', 'ID-SB', 43),
    ('Sumatera Selatan', 'ID-SS', 44),
    ('Sumatera Utara', 'ID-SU', 45)
) AS v(name, code, ord)
WHERE p.standard_code = 'ID' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 英国（GB）：221 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Aberdeen', 'GB-ABE', 1),
    ('Aberdeenshire', 'GB-ABD', 2),
    ('Angus', 'GB-ANS', 3),
    ('Antrim and Newtownabbey', 'GB-ANN', 4),
    ('Ards and North Down', 'GB-AND', 5),
    ('Argyll and Bute', 'GB-AGB', 6),
    ('Armagh, Banbridge and Craigavon', 'GB-ABC', 7),
    ('Barking and Dagenham', 'GB-BDG', 8),
    ('Barnet', 'GB-BNE', 9),
    ('Barnsley', 'GB-BNS', 10),
    ('Bath and North East Somerset', 'GB-BAS', 11),
    ('Bedford', 'GB-BDF', 12),
    ('Belfast', 'GB-BFS', 13),
    ('Bexley', 'GB-BEX', 14),
    ('Birmingham', 'GB-BIR', 15),
    ('Blackburn with Darwen', 'GB-BBD', 16),
    ('Blackpool', 'GB-BPL', 17),
    ('Blaenau Gwent', 'GB-BGW', 18),
    ('Bolton', 'GB-BOL', 19),
    ('Bournemouth, Christchurch and Poole', 'GB-BCP', 20),
    ('Bracknell Forest', 'GB-BRC', 21),
    ('Bradford', 'GB-BRD', 22),
    ('Brent', 'GB-BEN', 23),
    ('Bridgend', 'GB-BGE', 24),
    ('Brighton and Hove', 'GB-BNH', 25),
    ('Bristol', 'GB-BST', 26),
    ('Bromley', 'GB-BRY', 27),
    ('Buckinghamshire', 'GB-BKM', 28),
    ('Bury', 'GB-BUR', 29),
    ('Caerphilly', 'GB-CAY', 30),
    ('Calderdale', 'GB-CLD', 31),
    ('Cambridgeshire', 'GB-CAM', 32),
    ('Camden', 'GB-CMD', 33),
    ('Cardiff', 'GB-CRF', 34),
    ('Carmarthenshire', 'GB-CMN', 35),
    ('Causeway Coast and Glens', 'GB-CCG', 36),
    ('Central Bedfordshire', 'GB-CBF', 37),
    ('Ceredigion', 'GB-CGN', 38),
    ('Cheshire East', 'GB-CHE', 39),
    ('Cheshire West and Chester', 'GB-CHW', 40),
    ('City of Kingston upon Hull', 'GB-KHL', 41),
    ('City of Southampton', 'GB-STH', 42),
    ('Clackmannanshire', 'GB-CLK', 43),
    ('Conwy', 'GB-CWY', 44),
    ('Cornwall', 'GB-CON', 45),
    ('Coventry', 'GB-COV', 46),
    ('Croydon', 'GB-CRY', 47),
    ('Cumbria', 'GB-CMA', 48),
    ('Darlington', 'GB-DAL', 49),
    ('Denbighshire', 'GB-DEN', 50),
    ('Derby', 'GB-DER', 51),
    ('Derbyshire', 'GB-DBY', 52),
    ('Derry City and Strabane', 'GB-DRS', 53),
    ('Devon', 'GB-DEV', 54),
    ('Doncaster', 'GB-DNC', 55),
    ('Dorset', 'GB-DOR', 56),
    ('Dudley', 'GB-DUD', 57),
    ('Dumfries and Galloway', 'GB-DGY', 58),
    ('Dundee', 'GB-DND', 59),
    ('Durham', 'GB-DUR', 60),
    ('Ealing', 'GB-EAL', 61),
    ('East Ayrshire', 'GB-EAY', 62),
    ('East Dunbartonshire', 'GB-EDU', 63),
    ('East Lothian', 'GB-ELN', 64),
    ('East Renfrewshire', 'GB-ERW', 65),
    ('East Riding of Yorkshire', 'GB-ERY', 66),
    ('East Sussex', 'GB-ESX', 67),
    ('Edinburgh', 'GB-EDH', 68),
    ('Enfield', 'GB-ENF', 69),
    ('England', 'GB-ENG', 70),
    ('Essex', 'GB-ESS', 71),
    ('Falkirk', 'GB-FAL', 72),
    ('Fermanagh and Omagh', 'GB-FMO', 73),
    ('Fife', 'GB-FIF', 74),
    ('Flintshire', 'GB-FLN', 75),
    ('Gateshead', 'GB-GAT', 76),
    ('Glasgow', 'GB-GLG', 77),
    ('Gloucestershire', 'GB-GLS', 78),
    ('Greenwich', 'GB-GRE', 79),
    ('Gwynedd', 'GB-GWN', 80),
    ('Hackney', 'GB-HCK', 81),
    ('Halton', 'GB-HAL', 82),
    ('Hammersmith and Fulham', 'GB-HMF', 83),
    ('Hampshire', 'GB-HAM', 84),
    ('Haringey', 'GB-HRY', 85),
    ('Harrow', 'GB-HRW', 86),
    ('Hartlepool', 'GB-HPL', 87),
    ('Havering', 'GB-HAV', 88),
    ('Herefordshire', 'GB-HEF', 89),
    ('Hertfordshire', 'GB-HRT', 90),
    ('Highland', 'GB-HLD', 91),
    ('Hillingdon', 'GB-HIL', 92),
    ('Hounslow', 'GB-HNS', 93),
    ('Inverclyde', 'GB-IVC', 94),
    ('Isle of Anglesey', 'GB-AGY', 95),
    ('Isle of Wight', 'GB-IOW', 96),
    ('Isles of Scilly', 'GB-IOS', 97),
    ('Islington', 'GB-ISL', 98),
    ('Kensington and Chelsea', 'GB-KEC', 99),
    ('Kent', 'GB-KEN', 100),
    ('Kingston upon Thames', 'GB-KTT', 101),
    ('Kirklees', 'GB-KIR', 102),
    ('Knowsley', 'GB-KWL', 103),
    ('Lambeth', 'GB-LBH', 104),
    ('Lancashire', 'GB-LAN', 105),
    ('Leeds', 'GB-LDS', 106),
    ('Leicester', 'GB-LCE', 107),
    ('Leicestershire', 'GB-LEC', 108),
    ('Lewisham', 'GB-LEW', 109),
    ('Lincolnshire', 'GB-LIN', 110),
    ('Lisburn and Castlereagh', 'GB-LBC', 111),
    ('Liverpool', 'GB-LIV', 112),
    ('London', 'GB-LND', 113),
    ('Luton', 'GB-LUT', 114),
    ('Manchester', 'GB-MAN', 115),
    ('Medway', 'GB-MDW', 116),
    ('Merthyr Tydfil', 'GB-MTY', 117),
    ('Merton', 'GB-MRT', 118),
    ('Mid and East Antrim', 'GB-MEA', 119),
    ('Mid Ulster', 'GB-MUL', 120),
    ('Middlesbrough', 'GB-MDB', 121),
    ('Midlothian', 'GB-MLN', 122),
    ('Milton Keynes', 'GB-MIK', 123),
    ('Monmouthshire', 'GB-MON', 124),
    ('Moray', 'GB-MRY', 125),
    ('Neath Port Talbot', 'GB-NTL', 126),
    ('Newcastle upon Tyne', 'GB-NET', 127),
    ('Newham', 'GB-NWM', 128),
    ('Newport', 'GB-NWP', 129),
    ('Newry, Mourne and Down', 'GB-NMD', 130),
    ('Norfolk', 'GB-NFK', 131),
    ('North Ayrshire', 'GB-NAY', 132),
    ('North East Lincolnshire', 'GB-NEL', 133),
    ('North Lanarkshire', 'GB-NLK', 134),
    ('North Lincolnshire', 'GB-NLN', 135),
    ('North Northamptonshire', 'GB-NNH', 136),
    ('North Somerset', 'GB-NSM', 137),
    ('North Tyneside', 'GB-NTY', 138),
    ('North Yorkshire', 'GB-NYK', 139),
    ('Northern Ireland', 'GB-NIR', 140),
    ('Northumberland', 'GB-NBL', 141),
    ('Nottingham', 'GB-NGM', 142),
    ('Nottinghamshire', 'GB-NTT', 143),
    ('Oldham', 'GB-OLD', 144),
    ('Orkney Islands', 'GB-ORK', 145),
    ('Outer Hebrides', 'GB-ELS', 146),
    ('Oxfordshire', 'GB-OXF', 147),
    ('Pembrokeshire', 'GB-PEM', 148),
    ('Perth and Kinross', 'GB-PKN', 149),
    ('Peterborough', 'GB-PTE', 150),
    ('Plymouth', 'GB-PLY', 151),
    ('Portsmouth', 'GB-POR', 152),
    ('Powys', 'GB-POW', 153),
    ('Reading', 'GB-RDG', 154),
    ('Redbridge', 'GB-RDB', 155),
    ('Redcar and Cleveland', 'GB-RCC', 156),
    ('Renfrewshire', 'GB-RFW', 157),
    ('Rhondda Cynon Taf', 'GB-RCT', 158),
    ('Richmond upon Thames', 'GB-RIC', 159),
    ('Rochdale', 'GB-RCH', 160),
    ('Rotherham', 'GB-ROT', 161),
    ('Rutland', 'GB-RUT', 162),
    ('Salford', 'GB-SLF', 163),
    ('Sandwell', 'GB-SAW', 164),
    ('Scotland', 'GB-SCT', 165),
    ('Scottish Borders', 'GB-SCB', 166),
    ('Sefton', 'GB-SFT', 167),
    ('Sheffield', 'GB-SHF', 168),
    ('Shetland Islands', 'GB-ZET', 169),
    ('Shropshire', 'GB-SHR', 170),
    ('Slough', 'GB-SLG', 171),
    ('Solihull', 'GB-SOL', 172),
    ('Somerset', 'GB-SOM', 173),
    ('South Ayrshire', 'GB-SAY', 174),
    ('South Gloucestershire', 'GB-SGC', 175),
    ('South Lanarkshire', 'GB-SLK', 176),
    ('South Tyneside', 'GB-STY', 177),
    ('Southend-on-Sea', 'GB-SOS', 178),
    ('Southwark', 'GB-SWK', 179),
    ('St Helens', 'GB-SHN', 180),
    ('Staffordshire', 'GB-STS', 181),
    ('Stirling', 'GB-STG', 182),
    ('Stockport', 'GB-SKP', 183),
    ('Stockton-on-Tees', 'GB-STT', 184),
    ('Stoke-on-Trent', 'GB-STE', 185),
    ('Suffolk', 'GB-SFK', 186),
    ('Sunderland', 'GB-SND', 187),
    ('Surrey', 'GB-SRY', 188),
    ('Sutton', 'GB-STN', 189),
    ('Swansea', 'GB-SWA', 190),
    ('Swindon', 'GB-SWD', 191),
    ('Tameside', 'GB-TAM', 192),
    ('Telford and Wrekin', 'GB-TFW', 193),
    ('Thurrock', 'GB-THR', 194),
    ('Torbay', 'GB-TOB', 195),
    ('Torfaen', 'GB-TOF', 196),
    ('Tower Hamlets', 'GB-TWH', 197),
    ('Trafford', 'GB-TRF', 198),
    ('Vale of Glamorgan', 'GB-VGL', 199),
    ('Wakefield', 'GB-WKF', 200),
    ('Wales', 'GB-WLS', 201),
    ('Walsall', 'GB-WLL', 202),
    ('Waltham Forest', 'GB-WFT', 203),
    ('Wandsworth', 'GB-WND', 204),
    ('Warrington', 'GB-WRT', 205),
    ('Warwickshire', 'GB-WAR', 206),
    ('West Berkshire', 'GB-WBK', 207),
    ('West Dunbartonshire', 'GB-WDU', 208),
    ('West Lothian', 'GB-WLN', 209),
    ('West Northamptonshire', 'GB-WNH', 210),
    ('West Sussex', 'GB-WSX', 211),
    ('Westminster', 'GB-WSM', 212),
    ('Wigan', 'GB-WGN', 213),
    ('Wiltshire', 'GB-WIL', 214),
    ('Windsor and Maidenhead', 'GB-WNM', 215),
    ('Wirral', 'GB-WRL', 216),
    ('Wokingham', 'GB-WOK', 217),
    ('Wolverhampton', 'GB-WLV', 218),
    ('Worcestershire', 'GB-WOR', 219),
    ('Wrexham', 'GB-WRX', 220),
    ('York', 'GB-YOR', 221)
) AS v(name, code, ord)
WHERE p.standard_code = 'GB' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 以色列（IL）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Central', 'IL-M', 1),
    ('Haifa', 'IL-HA', 2),
    ('Jerusalem', 'IL-JM', 3),
    ('Northern', 'IL-Z', 4),
    ('Southern', 'IL-D', 5),
    ('Tel Aviv', 'IL-TA', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'IL' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 约旦（JO）：12 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ajloun', 'JO-AJ', 1),
    ('Amman', 'JO-AM', 2),
    ('Aqaba', 'JO-AQ', 3),
    ('Balqa', 'JO-BA', 4),
    ('Irbid', 'JO-IR', 5),
    ('Jerash', 'JO-JA', 6),
    ('Karak', 'JO-KA', 7),
    ('Ma''an', 'JO-MN', 8),
    ('Madaba', 'JO-MD', 9),
    ('Mafraq', 'JO-MA', 10),
    ('Tafilah', 'JO-AT', 11),
    ('Zarqa', 'JO-AZ', 12)
) AS v(name, code, ord)
WHERE p.standard_code = 'JO' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 越南（VN）：34 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('An Giang', 'VN-44', 1),
    ('Bắc Ninh', 'VN-56', 2),
    ('Cà Mau', 'VN-59', 3),
    ('Cần Thơ', 'VN-CT', 4),
    ('Cao Bằng', 'VN-04', 5),
    ('Đà Nẵng', 'VN-DN', 6),
    ('Đắk Lắk', 'VN-33', 7),
    ('Điện Biên', 'VN-71', 8),
    ('Đồng Nai', 'VN-39', 9),
    ('Đồng Tháp', 'VN-45', 10),
    ('Gia Lai', 'VN-30', 11),
    ('Hà Nội', 'VN-HN', 12),
    ('Hà Tĩnh', 'VN-23', 13),
    ('Hải Phòng', 'VN-HP', 14),
    ('Hồ Chí Minh', 'VN-SG', 15),
    ('Huế', 'VN-26', 16),
    ('Hưng Yên', 'VN-66', 17),
    ('Khánh Hòa', 'VN-34', 18),
    ('Lai Châu', 'VN-01', 19),
    ('Lâm Đồng', 'VN-35', 20),
    ('Lạng Sơn', 'VN-09', 21),
    ('Lào Cai', 'VN-02', 22),
    ('Nghệ An', 'VN-22', 23),
    ('Ninh Bình', 'VN-18', 24),
    ('Phú Thọ', 'VN-68', 25),
    ('Quảng Ngãi', 'VN-29', 26),
    ('Quảng Ninh', 'VN-13', 27),
    ('Quảng Trị', 'VN-25', 28),
    ('Sơn La', 'VN-05', 29),
    ('Tây Ninh', 'VN-37', 30),
    ('Thái Nguyên', 'VN-69', 31),
    ('Thanh Hóa', 'VN-21', 32),
    ('Tuyên Quang', 'VN-07', 33),
    ('Vĩnh Long', 'VN-49', 34)
) AS v(name, code, ord)
WHERE p.standard_code = 'VN' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 赞比亚（ZM）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Central', 'ZM-02', 1),
    ('Copperbelt', 'ZM-08', 2),
    ('Eastern', 'ZM-03', 3),
    ('Luapula', 'ZM-04', 4),
    ('Lusaka', 'ZM-09', 5),
    ('Muchinga', 'ZM-10', 6),
    ('Northern', 'ZM-05', 7),
    ('Northwestern', 'ZM-06', 8),
    ('Southern', 'ZM-07', 9),
    ('Western', 'ZM-01', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'ZM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 泽西岛（JE）：12 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Grouville', 'JE-01', 1),
    ('St Brelade', 'JE-02', 2),
    ('St Clement', 'JE-03', 3),
    ('St Helier', 'JE-04', 4),
    ('St John', 'JE-05', 5),
    ('St Lawrence', 'JE-06', 6),
    ('St Martin', 'JE-07', 7),
    ('St Mary', 'JE-08', 8),
    ('St Ouen', 'JE-09', 9),
    ('St Peter', 'JE-10', 10),
    ('St Saviour', 'JE-11', 11),
    ('Trinity', 'JE-12', 12)
) AS v(name, code, ord)
WHERE p.standard_code = 'JE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 乍得（TD）：23 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bahr el Gazel', 'TD-BG', 1),
    ('Batha', 'TD-BA', 2),
    ('Borkou', 'TD-BO', 3),
    ('Chari-Baguirmi', 'TD-CB', 4),
    ('Ennedi-Est', 'TD-EE', 5),
    ('Ennedi-Ouest', 'TD-EO', 6),
    ('Guéra', 'TD-GR', 7),
    ('Hadjer-Lamis', 'TD-HL', 8),
    ('Kanem', 'TD-KA', 9),
    ('Lac', 'TD-LC', 10),
    ('Logone Occidental', 'TD-LO', 11),
    ('Logone Oriental', 'TD-LR', 12),
    ('Mandoul', 'TD-MA', 13),
    ('Mayo-Kebbi Est', 'TD-ME', 14),
    ('Mayo-Kebbi Ouest', 'TD-MO', 15),
    ('Moyen-Chari', 'TD-MC', 16),
    ('N''Djamena', 'TD-ND', 17),
    ('Ouaddaï', 'TD-OD', 18),
    ('Salamat', 'TD-SA', 19),
    ('Sila', 'TD-SI', 20),
    ('Tandjilé', 'TD-TA', 21),
    ('Tibesti', 'TD-TI', 22),
    ('Wadi Fira', 'TD-WF', 23)
) AS v(name, code, ord)
WHERE p.standard_code = 'TD' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 智利（CL）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Aisén del General Carlos Ibañez del Campo', 'CL-AI', 1),
    ('Antofagasta', 'CL-AN', 2),
    ('Arica y Parinacota', 'CL-AP', 3),
    ('Atacama', 'CL-AT', 4),
    ('Biobío', 'CL-BI', 5),
    ('Coquimbo', 'CL-CO', 6),
    ('La Araucanía', 'CL-AR', 7),
    ('Libertador General Bernardo O''Higgins', 'CL-LI', 8),
    ('Los Lagos', 'CL-LL', 9),
    ('Los Ríos', 'CL-LR', 10),
    ('Magallanes y de la Antártica Chilena', 'CL-MA', 11),
    ('Maule', 'CL-ML', 12),
    ('Ñuble', 'CL-NB', 13),
    ('Región Metropolitana de Santiago', 'CL-RM', 14),
    ('Tarapacá', 'CL-TA', 15),
    ('Valparaíso', 'CL-VS', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'CL' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 中非共和国（CF）：17 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bamingui-Bangoran', 'CF-BB', 1),
    ('Bangui', 'CF-BGF', 2),
    ('Basse-Kotto', 'CF-BK', 3),
    ('Haut-Mbomou', 'CF-HM', 4),
    ('Haute-Kotto', 'CF-HK', 5),
    ('Kémo', 'CF-KG', 6),
    ('Lobaye', 'CF-LB', 7),
    ('Mambéré-Kadéï', 'CF-HS', 8),
    ('Mbomou', 'CF-MB', 9),
    ('Nana-Grébizi', 'CF-KB', 10),
    ('Nana-Mambéré', 'CF-NM', 11),
    ('Ombella-M''Poko', 'CF-MP', 12),
    ('Ouaka', 'CF-UK', 13),
    ('Ouham', 'CF-AC', 14),
    ('Ouham-Pendé', 'CF-OP', 15),
    ('Sangha-Mbaéré', 'CF-SE', 16),
    ('Vakaga', 'CF-VK', 17)
) AS v(name, code, ord)
WHERE p.standard_code = 'CF' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

COMMIT;
