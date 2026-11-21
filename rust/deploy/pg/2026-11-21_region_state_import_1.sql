-- 2026-11-21: 跨境国家/地区全量导入（二）州/省层 第 1 部分 —— Tigshop 论坛帖 407879（我方新增）。
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

-- 阿尔巴尼亚（AL）：12 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Berat', 'AL-01', 1),
    ('Dibër', 'AL-09', 2),
    ('Durrës', 'AL-02', 3),
    ('Elbasan', 'AL-03', 4),
    ('Fier', 'AL-04', 5),
    ('Gjirokastër', 'AL-05', 6),
    ('Korçë', 'AL-06', 7),
    ('Kukës', 'AL-07', 8),
    ('Lezhë', 'AL-08', 9),
    ('Shkodër', 'AL-10', 10),
    ('Tirana', 'AL-11', 11),
    ('Vlorë', 'AL-12', 12)
) AS v(name, code, ord)
WHERE p.standard_code = 'AL' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 阿尔及利亚（DZ）：58 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Adrar', 'DZ-01', 1),
    ('Aïn Defla', 'DZ-44', 2),
    ('Aïn Témouchent', 'DZ-46', 3),
    ('Algiers', 'DZ-16', 4),
    ('Annaba', 'DZ-23', 5),
    ('Batna', 'DZ-05', 6),
    ('Béchar', 'DZ-08', 7),
    ('Béjaïa', 'DZ-06', 8),
    ('Béni Abbès', 'DZ-52', 9),
    ('Biskra', 'DZ-07', 10),
    ('Blida', 'DZ-09', 11),
    ('Bordj Baji Mokhtar', 'DZ-50', 12),
    ('Bordj Bou Arréridj', 'DZ-34', 13),
    ('Bouïra', 'DZ-10', 14),
    ('Boumerdès', 'DZ-35', 15),
    ('Chlef', 'DZ-02', 16),
    ('Constantine', 'DZ-25', 17),
    ('Djanet', 'DZ-56', 18),
    ('Djelfa', 'DZ-17', 19),
    ('El Bayadh', 'DZ-32', 20),
    ('El M''ghair', 'DZ-57', 21),
    ('El Menia', 'DZ-58', 22),
    ('El Oued', 'DZ-39', 23),
    ('El Tarf', 'DZ-36', 24),
    ('Ghardaïa', 'DZ-47', 25),
    ('Guelma', 'DZ-24', 26),
    ('Illizi', 'DZ-33', 27),
    ('In Guezzam', 'DZ-54', 28),
    ('In Salah', 'DZ-53', 29),
    ('Jijel', 'DZ-18', 30),
    ('Khenchela', 'DZ-40', 31),
    ('Laghouat', 'DZ-03', 32),
    ('M''Sila', 'DZ-28', 33),
    ('Mascara', 'DZ-29', 34),
    ('Médéa', 'DZ-26', 35),
    ('Mila', 'DZ-43', 36),
    ('Mostaganem', 'DZ-27', 37),
    ('Naama', 'DZ-45', 38),
    ('Oran', 'DZ-31', 39),
    ('Ouargla', 'DZ-30', 40),
    ('Ouled Djellal', 'DZ-51', 41),
    ('Oum El Bouaghi', 'DZ-04', 42),
    ('Relizane', 'DZ-48', 43),
    ('Saïda', 'DZ-20', 44),
    ('Sétif', 'DZ-19', 45),
    ('Sidi Bel Abbès', 'DZ-22', 46),
    ('Skikda', 'DZ-21', 47),
    ('Souk Ahras', 'DZ-41', 48),
    ('Tamanghasset', 'DZ-11', 49),
    ('Tébessa', 'DZ-12', 50),
    ('Tiaret', 'DZ-14', 51),
    ('Timimoun', 'DZ-49', 52),
    ('Tindouf', 'DZ-37', 53),
    ('Tipasa', 'DZ-42', 54),
    ('Tissemsilt', 'DZ-38', 55),
    ('Tizi Ouzou', 'DZ-15', 56),
    ('Tlemcen', 'DZ-13', 57),
    ('Touggourt', 'DZ-55', 58)
) AS v(name, code, ord)
WHERE p.standard_code = 'DZ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 阿富汗（AF）：34 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Badakhshan', 'AF-BDS', 1),
    ('Badghis', 'AF-BDG', 2),
    ('Baghlan', 'AF-BGL', 3),
    ('Balkh', 'AF-BAL', 4),
    ('Bamyan', 'AF-BAM', 5),
    ('Daykundi', 'AF-DAY', 6),
    ('Farah', 'AF-FRA', 7),
    ('Faryab', 'AF-FYB', 8),
    ('Ghazni', 'AF-GHA', 9),
    ('Ghōr', 'AF-GHO', 10),
    ('Helmand', 'AF-HEL', 11),
    ('Herat', 'AF-HER', 12),
    ('Jowzjan', 'AF-JOW', 13),
    ('Kabul', 'AF-KAB', 14),
    ('Kandahar', 'AF-KAN', 15),
    ('Kapisa', 'AF-KAP', 16),
    ('Khost', 'AF-KHO', 17),
    ('Kunar', 'AF-KNR', 18),
    ('Kunduz Province', 'AF-KDZ', 19),
    ('Laghman', 'AF-LAG', 20),
    ('Logar', 'AF-LOG', 21),
    ('Nangarhar', 'AF-NAN', 22),
    ('Nimruz', 'AF-NIM', 23),
    ('Nuristan', 'AF-NUR', 24),
    ('Paktia', 'AF-PIA', 25),
    ('Paktika', 'AF-PKA', 26),
    ('Panjshir', 'AF-PAN', 27),
    ('Parwan', 'AF-PAR', 28),
    ('Samangan', 'AF-SAM', 29),
    ('Sar-e Pol', 'AF-SAR', 30),
    ('Takhar', 'AF-TAK', 31),
    ('Urozgan', 'AF-URU', 32),
    ('Wardak', 'AF-WAR', 33),
    ('Zabul', 'AF-ZAB', 34)
) AS v(name, code, ord)
WHERE p.standard_code = 'AF' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 阿根廷（AR）：24 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Autonomous City of Buenos Aires', 'AR-C', 1),
    ('Buenos Aires', 'AR-B', 2),
    ('Catamarca', 'AR-K', 3),
    ('Chaco', 'AR-H', 4),
    ('Chubut', 'AR-U', 5),
    ('Córdoba', 'AR-X', 6),
    ('Corrientes', 'AR-W', 7),
    ('Entre Ríos', 'AR-E', 8),
    ('Formosa', 'AR-P', 9),
    ('Jujuy', 'AR-Y', 10),
    ('La Pampa', 'AR-L', 11),
    ('La Rioja', 'AR-F', 12),
    ('Mendoza', 'AR-M', 13),
    ('Misiones', 'AR-N', 14),
    ('Neuquén', 'AR-Q', 15),
    ('Río Negro', 'AR-R', 16),
    ('Salta', 'AR-A', 17),
    ('San Juan', 'AR-J', 18),
    ('San Luis', 'AR-D', 19),
    ('Santa Cruz', 'AR-Z', 20),
    ('Santa Fe', 'AR-S', 21),
    ('Santiago del Estero', 'AR-G', 22),
    ('Tierra del Fuego', 'AR-V', 23),
    ('Tucumán', 'AR-T', 24)
) AS v(name, code, ord)
WHERE p.standard_code = 'AR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 爱尔兰（IE）：30 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Carlow', 'IE-CW', 1),
    ('Cavan', 'IE-CN', 2),
    ('Clare', 'IE-CE', 3),
    ('Connacht', 'IE-C', 4),
    ('Cork', 'IE-CO', 5),
    ('Donegal', 'IE-DL', 6),
    ('Dublin', 'IE-D', 7),
    ('Galway', 'IE-G', 8),
    ('Kerry', 'IE-KY', 9),
    ('Kildare', 'IE-KE', 10),
    ('Kilkenny', 'IE-KK', 11),
    ('Laois', 'IE-LS', 12),
    ('Leinster', 'IE-L', 13),
    ('Leitrim', 'IE-LM', 14),
    ('Limerick', 'IE-LK', 15),
    ('Longford', 'IE-LD', 16),
    ('Louth', 'IE-LH', 17),
    ('Mayo', 'IE-MO', 18),
    ('Meath', 'IE-MH', 19),
    ('Monaghan', 'IE-MN', 20),
    ('Munster', 'IE-M', 21),
    ('Offaly', 'IE-OY', 22),
    ('Roscommon', 'IE-RN', 23),
    ('Sligo', 'IE-SO', 24),
    ('Tipperary', 'IE-TA', 25),
    ('Ulster', 'IE-U', 26),
    ('Waterford', 'IE-WD', 27),
    ('Westmeath', 'IE-WH', 28),
    ('Wexford', 'IE-WX', 29),
    ('Wicklow', 'IE-WW', 30)
) AS v(name, code, ord)
WHERE p.standard_code = 'IE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 埃及（EG）：27 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alexandria', 'EG-ALX', 1),
    ('Aswan', 'EG-ASN', 2),
    ('Asyut', 'EG-AST', 3),
    ('Beheira', 'EG-BH', 4),
    ('Beni Suef', 'EG-BNS', 5),
    ('Cairo', 'EG-C', 6),
    ('Dakahlia', 'EG-DK', 7),
    ('Damietta', 'EG-DT', 8),
    ('Faiyum', 'EG-FYM', 9),
    ('Gharbia', 'EG-GH', 10),
    ('Giza', 'EG-GZ', 11),
    ('Ismailia', 'EG-IS', 12),
    ('Kafr El-Sheikh', 'EG-KFS', 13),
    ('Luxor', 'EG-LX', 14),
    ('Matrouh', 'EG-MT', 15),
    ('Minya', 'EG-MN', 16),
    ('Monufia', 'EG-MNF', 17),
    ('New Valley', 'EG-WAD', 18),
    ('North Sinai', 'EG-SIN', 19),
    ('Port Said', 'EG-PTS', 20),
    ('Qalyubia', 'EG-KB', 21),
    ('Qena', 'EG-KN', 22),
    ('Red Sea', 'EG-BA', 23),
    ('Sharqia', 'EG-SHR', 24),
    ('Sohag', 'EG-SHG', 25),
    ('South Sinai', 'EG-JS', 26),
    ('Suez', 'EG-SUZ', 27)
) AS v(name, code, ord)
WHERE p.standard_code = 'EG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 埃塞俄比亚（ET）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Addis Ababa', 'ET-AA', 1),
    ('Afar', 'ET-AF', 2),
    ('Amhara', 'ET-AM', 3),
    ('Benishangul-Gumuz', 'ET-BE', 4),
    ('Central Ethiopia', 'ET-55', 5),
    ('Dire Dawa', 'ET-DD', 6),
    ('Gambela', 'ET-GA', 7),
    ('Harari', 'ET-HA', 8),
    ('Oromia', 'ET-OR', 9),
    ('Sidama', 'ET-SI', 10),
    ('Somali', 'ET-SO', 11),
    ('South Ethiopia', 'ET-56', 12),
    ('Southwest Ethiopia Peoples', 'ET-SW', 13),
    ('Tigray', 'ET-TI', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'ET' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 爱沙尼亚（EE）：93 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alutaguse', 'EE-130', 1),
    ('Anija', 'EE-141', 2),
    ('Antsla', 'EE-142', 3),
    ('Elva', 'EE-171', 4),
    ('Häädemeeste', 'EE-214', 5),
    ('Haapsalu', 'EE-184', 6),
    ('Haljala', 'EE-191', 7),
    ('Harju', 'EE-37', 8),
    ('Harku', 'EE-198', 9),
    ('Hiiu', 'EE-39', 10),
    ('Hiiumaa', 'EE-205', 11),
    ('Ida-Viru', 'EE-45', 12),
    ('Järva', 'EE-255', 13),
    ('Järva', 'EE-52', 14),
    ('Joelähtme', 'EE-245', 15),
    ('Jõgeva', 'EE-50', 16),
    ('Jõgeva', 'EE-247', 17),
    ('Jõhvi', 'EE-250', 18),
    ('Kadrina', 'EE-272', 19),
    ('Kambja', 'EE-283', 20),
    ('Kanepi', 'EE-284', 21),
    ('Kastre', 'EE-291', 22),
    ('Kehtna', 'EE-293', 23),
    ('Keila', 'EE-296', 24),
    ('Kihnu', 'EE-303', 25),
    ('Kiili', 'EE-305', 26),
    ('Kohila', 'EE-317', 27),
    ('Kohtla-Järve', 'EE-321', 28),
    ('Kose', 'EE-338', 29),
    ('Kuusalu', 'EE-353', 30),
    ('Lääne', 'EE-56', 31),
    ('Lääne-Harju', 'EE-431', 32),
    ('Lääne-Nigula', 'EE-441', 33),
    ('Lääne-Viru', 'EE-60', 34),
    ('Lääneranna', 'EE-430', 35),
    ('Loksa', 'EE-424', 36),
    ('Lüganuse', 'EE-442', 37),
    ('Luunja', 'EE-432', 38),
    ('Maardu', 'EE-446', 39),
    ('Märjamaa', 'EE-503', 40),
    ('Muhu', 'EE-478', 41),
    ('Mulgi', 'EE-480', 42),
    ('Mustvee', 'EE-486', 43),
    ('Narva', 'EE-511', 44),
    ('Narva-Jõesuu', 'EE-514', 45),
    ('Noo', 'EE-528', 46),
    ('Otepää', 'EE-557', 47),
    ('Paide', 'EE-567', 48),
    ('Pärnu', 'EE-624', 49),
    ('Pärnu', 'EE-68', 50),
    ('Peipsiääre', 'EE-586', 51),
    ('Põhja-Pärnumaa', 'EE-638', 52),
    ('Põhja-Sakala', 'EE-615', 53),
    ('Poltsamaa', 'EE-618', 54),
    ('Põlva', 'EE-622', 55),
    ('Põlva', 'EE-64', 56),
    ('Raasiku', 'EE-651', 57),
    ('Rae', 'EE-653', 58),
    ('Rakvere', 'EE-663', 59),
    ('Rakvere', 'EE-661', 60),
    ('Räpina', 'EE-708', 61),
    ('Rapla', 'EE-71', 62),
    ('Rapla', 'EE-668', 63),
    ('Rõuge', 'EE-698', 64),
    ('Ruhnu', 'EE-689', 65),
    ('Saarde', 'EE-712', 66),
    ('Saare', 'EE-74', 67),
    ('Saaremaa', 'EE-714', 68),
    ('Saku', 'EE-719', 69),
    ('Saue', 'EE-726', 70),
    ('Setomaa', 'EE-732', 71),
    ('Sillamäe', 'EE-735', 72),
    ('Tallinn', 'EE-784', 73),
    ('Tapa', 'EE-792', 74),
    ('Tartu', 'EE-79', 75),
    ('Tartu', 'EE-793', 76),
    ('Tartu', 'EE-796', 77),
    ('Tori', 'EE-809', 78),
    ('Tõrva', 'EE-824', 79),
    ('Türi', 'EE-834', 80),
    ('Väike-Maarja', 'EE-928', 81),
    ('Valga', 'EE-855', 82),
    ('Valga', 'EE-81', 83),
    ('Viimsi', 'EE-890', 84),
    ('Viljandi', 'EE-899', 85),
    ('Viljandi', 'EE-897', 86),
    ('Viljandi', 'EE-84', 87),
    ('Vinni', 'EE-901', 88),
    ('Viru-Nigula', 'EE-903', 89),
    ('Vormsi', 'EE-907', 90),
    ('Võru', 'EE-87', 91),
    ('Võru', 'EE-917', 92),
    ('Võru', 'EE-919', 93)
) AS v(name, code, ord)
WHERE p.standard_code = 'EE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 阿拉伯联合酋长国（AE）：7 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Abu Dhabi', 'AE-AZ', 1),
    ('Ajman', 'AE-AJ', 2),
    ('Dubai', 'AE-DU', 3),
    ('Fujairah', 'AE-FU', 4),
    ('Ras Al Khaimah', 'AE-RK', 5),
    ('Sharjah', 'AE-SH', 6),
    ('Umm Al Quwain', 'AE-UQ', 7)
) AS v(name, code, ord)
WHERE p.standard_code = 'AE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 阿鲁巴（AW）：9 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Noord', 'AW-01', 1),
    ('Oranjestad', 'AW-09', 2),
    ('Oranjestad East', 'AW-03', 3),
    ('Oranjestad West', 'AW-02', 4),
    ('Paradera', 'AW-04', 5),
    ('San Nicolaas Noord', 'AW-05', 6),
    ('San Nicolaas Zuid', 'AW-06', 7),
    ('Santa Cruz', 'AW-07', 8),
    ('Savaneta', 'AW-08', 9)
) AS v(name, code, ord)
WHERE p.standard_code = 'AW' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 阿曼（OM）：11 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ad Dakhiliyah', 'OM-DA', 1),
    ('Ad Dhahirah', 'OM-ZA', 2),
    ('Al Batinah North', 'OM-BS', 3),
    ('Al Batinah South', 'OM-BJ', 4),
    ('Al Buraimi', 'OM-BU', 5),
    ('Al Wusta', 'OM-WU', 6),
    ('Ash Sharqiyah North', 'OM-SS', 7),
    ('Ash Sharqiyah South', 'OM-SJ', 8),
    ('Dhofar', 'OM-ZU', 9),
    ('Musandam', 'OM-MU', 10),
    ('Muscat', 'OM-MA', 11)
) AS v(name, code, ord)
WHERE p.standard_code = 'OM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 安道尔（AD）：7 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Andorra la Vella', 'AD-07', 1),
    ('Canillo', 'AD-02', 2),
    ('Encamp', 'AD-03', 3),
    ('Escaldes-Engordany', 'AD-08', 4),
    ('La Massana', 'AD-04', 5),
    ('Ordino', 'AD-05', 6),
    ('Sant Julià de Lòria', 'AD-06', 7)
) AS v(name, code, ord)
WHERE p.standard_code = 'AD' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 安哥拉（AO）：19 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bengo', 'AO-BGO', 1),
    ('Benguela', 'AO-BGU', 2),
    ('Bié', 'AO-BIE', 3),
    ('Cabinda', 'AO-CAB', 4),
    ('Cuando', 'AO-CUB', 5),
    ('Cuanza Norte', 'AO-CNO', 6),
    ('Cuanza Sul', 'AO-CUS', 7),
    ('Cubango', 'AO-CBG', 8),
    ('Cunene', 'AO-CNN', 9),
    ('Huambo', 'AO-HUA', 10),
    ('Huíla', 'AO-HUI', 11),
    ('Luanda', 'AO-LUA', 12),
    ('Lunda Norte', 'AO-LNO', 13),
    ('Lunda Sul', 'AO-LSU', 14),
    ('Malanje', 'AO-MAL', 15),
    ('Moxico', 'AO-MOX', 16),
    ('Namibe', 'AO-NAM', 17),
    ('Uíge', 'AO-UIG', 18),
    ('Zaire', 'AO-ZAI', 19)
) AS v(name, code, ord)
WHERE p.standard_code = 'AO' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 安圭拉（AI）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Blowing Point', 'AI-01', 1),
    ('East End', 'AI-02', 2),
    ('George Hill', 'AI-03', 3),
    ('Island Harbour', 'AI-04', 4),
    ('North Hill', 'AI-05', 5),
    ('North Side', 'AI-06', 6),
    ('Sandy Ground', 'AI-07', 7),
    ('Sandy Hill', 'AI-08', 8),
    ('South Hill', 'AI-09', 9),
    ('Stoney Ground', 'AI-10', 10),
    ('The Farrington', 'AI-11', 11),
    ('The Quarter', 'AI-12', 12),
    ('The Valley', 'AI-13', 13),
    ('West End', 'AI-14', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'AI' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 安提瓜和巴布达（AG）：8 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Barbuda', 'AG-10', 1),
    ('Redonda', 'AG-11', 2),
    ('Saint George', 'AG-03', 3),
    ('Saint John', 'AG-04', 4),
    ('Saint Mary', 'AG-05', 5),
    ('Saint Paul', 'AG-06', 6),
    ('Saint Peter', 'AG-07', 7),
    ('Saint Philip', 'AG-08', 8)
) AS v(name, code, ord)
WHERE p.standard_code = 'AG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 澳大利亚（AU）：8 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Australian Capital Territory', 'AU-ACT', 1),
    ('New South Wales', 'AU-NSW', 2),
    ('Northern Territory', 'AU-NT', 3),
    ('Queensland', 'AU-QLD', 4),
    ('South Australia', 'AU-SA', 5),
    ('Tasmania', 'AU-TAS', 6),
    ('Victoria', 'AU-VIC', 7),
    ('Western Australia', 'AU-WA', 8)
) AS v(name, code, ord)
WHERE p.standard_code = 'AU' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 奥地利（AT）：9 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Burgenland', 'AT-1', 1),
    ('Carinthia', 'AT-2', 2),
    ('Lower Austria', 'AT-3', 3),
    ('Salzburg', 'AT-5', 4),
    ('Styria', 'AT-6', 5),
    ('Tyrol', 'AT-7', 6),
    ('Upper Austria', 'AT-4', 7),
    ('Vienna', 'AT-9', 8),
    ('Vorarlberg', 'AT-8', 9)
) AS v(name, code, ord)
WHERE p.standard_code = 'AT' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 奥兰群岛（AX）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Brändö', 'AX-12', 1),
    ('Eckerö', 'AX-08', 2),
    ('Finström', 'AX-03', 3),
    ('Föglö', 'AX-09', 4),
    ('Geta', 'AX-10', 5),
    ('Hammarland', 'AX-06', 6),
    ('Jomala', 'AX-02', 7),
    ('Kökar', 'AX-15', 8),
    ('Kumlinge', 'AX-14', 9),
    ('Lemland', 'AX-04', 10),
    ('Lumparland', 'AX-13', 11),
    ('Mariehamn', 'AX-01', 12),
    ('Saltvik', 'AX-05', 13),
    ('Sottunga', 'AX-16', 14),
    ('Sund', 'AX-07', 15),
    ('Vårdö', 'AX-11', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'AX' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 阿塞拜疆（AZ）：78 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Absheron', 'AZ-ABS', 1),
    ('Agdam', 'AZ-AGM', 2),
    ('Agdash', 'AZ-AGS', 3),
    ('Aghjabadi', 'AZ-AGC', 4),
    ('Agstafa', 'AZ-AGA', 5),
    ('Agsu', 'AZ-AGU', 6),
    ('Astara', 'AZ-AST', 7),
    ('Babek', 'AZ-BAB', 8),
    ('Baku', 'AZ-BA', 9),
    ('Balakan', 'AZ-BAL', 10),
    ('Barda', 'AZ-BAR', 11),
    ('Beylagan', 'AZ-BEY', 12),
    ('Bilasuvar', 'AZ-BIL', 13),
    ('Dashkasan', 'AZ-DAS', 14),
    ('Fizuli', 'AZ-FUZ', 15),
    ('Ganja', 'AZ-GA', 16),
    ('Gədəbəy', 'AZ-GAD', 17),
    ('Gobustan', 'AZ-QOB', 18),
    ('Goranboy', 'AZ-GOR', 19),
    ('Goychay', 'AZ-GOY', 20),
    ('Goygol', 'AZ-GYG', 21),
    ('Hajigabul', 'AZ-HAC', 22),
    ('Imishli', 'AZ-IMI', 23),
    ('Ismailli', 'AZ-ISM', 24),
    ('Jabrayil', 'AZ-CAB', 25),
    ('Jalilabad', 'AZ-CAL', 26),
    ('Julfa', 'AZ-CUL', 27),
    ('Kalbajar', 'AZ-KAL', 28),
    ('Kangarli', 'AZ-KAN', 29),
    ('Khachmaz', 'AZ-XAC', 30),
    ('Khankendi', 'AZ-XA', 31),
    ('Khizi', 'AZ-XIZ', 32),
    ('Khojali', 'AZ-XCI', 33),
    ('Kurdamir', 'AZ-KUR', 34),
    ('Lachin', 'AZ-LAC', 35),
    ('Lankaran', 'AZ-LAN', 36),
    ('Lankaran', 'AZ-LA', 37),
    ('Lerik', 'AZ-LER', 38),
    ('Martuni', 'AZ-XVD', 39),
    ('Masally', 'AZ-MAS', 40),
    ('Mingachevir', 'AZ-MI', 41),
    ('Naftalan', 'AZ-NA', 42),
    ('Nakhchivan', 'AZ-NV', 43),
    ('Nakhchivan', 'AZ-NX', 44),
    ('Neftchala', 'AZ-NEF', 45),
    ('Oghuz', 'AZ-OGU', 46),
    ('Ordubad', 'AZ-ORD', 47),
    ('Qabala', 'AZ-QAB', 48),
    ('Qakh', 'AZ-QAX', 49),
    ('Qazakh', 'AZ-QAZ', 50),
    ('Quba', 'AZ-QBA', 51),
    ('Qubadli', 'AZ-QBI', 52),
    ('Qusar', 'AZ-QUS', 53),
    ('Saatly', 'AZ-SAT', 54),
    ('Sabirabad', 'AZ-SAB', 55),
    ('Sadarak', 'AZ-SAD', 56),
    ('Salyan', 'AZ-SAL', 57),
    ('Samukh', 'AZ-SMX', 58),
    ('Shabran', 'AZ-SBN', 59),
    ('Shahbuz', 'AZ-SAH', 60),
    ('Shaki', 'AZ-SA', 61),
    ('Shaki', 'AZ-SAK', 62),
    ('Shamakhi', 'AZ-SMI', 63),
    ('Shamkir', 'AZ-SKR', 64),
    ('Sharur', 'AZ-SAR', 65),
    ('Shirvan', 'AZ-SR', 66),
    ('Shusha', 'AZ-SUS', 67),
    ('Siazan', 'AZ-SIY', 68),
    ('Sumqayit', 'AZ-SM', 69),
    ('Tartar', 'AZ-TAR', 70),
    ('Tovuz', 'AZ-TOV', 71),
    ('Ujar', 'AZ-UCA', 72),
    ('Yardymli', 'AZ-YAR', 73),
    ('Yevlakh', 'AZ-YEV', 74),
    ('Yevlakh', 'AZ-YE', 75),
    ('Zangilan', 'AZ-ZAN', 76),
    ('Zaqatala', 'AZ-ZAQ', 77),
    ('Zardab', 'AZ-ZAR', 78)
) AS v(name, code, ord)
WHERE p.standard_code = 'AZ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 巴巴多斯（BB）：11 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Christ Church', 'BB-01', 1),
    ('Saint Andrew', 'BB-02', 2),
    ('Saint George', 'BB-03', 3),
    ('Saint James', 'BB-04', 4),
    ('Saint John', 'BB-05', 5),
    ('Saint Joseph', 'BB-06', 6),
    ('Saint Lucy', 'BB-07', 7),
    ('Saint Michael', 'BB-08', 8),
    ('Saint Peter', 'BB-09', 9),
    ('Saint Philip', 'BB-10', 10),
    ('Saint Thomas', 'BB-11', 11)
) AS v(name, code, ord)
WHERE p.standard_code = 'BB' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 巴布亚新几内亚（PG）：22 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bougainville', 'PG-NSB', 1),
    ('Central', 'PG-CPM', 2),
    ('Chimbu', 'PG-CPK', 3),
    ('East New Britain', 'PG-EBR', 4),
    ('East Sepik', 'PG-ESW', 5),
    ('Eastern Highlands', 'PG-EHG', 6),
    ('Enga', 'PG-EPW', 7),
    ('Gulf', 'PG-GPK', 8),
    ('Hela', 'PG-HLA', 9),
    ('Jiwaka', 'PG-JWK', 10),
    ('Madang', 'PG-MPM', 11),
    ('Manus', 'PG-MRL', 12),
    ('Milne Bay', 'PG-MBA', 13),
    ('Morobe', 'PG-MPL', 14),
    ('New Ireland', 'PG-NIK', 15),
    ('Oro', 'PG-NPP', 16),
    ('Port Moresby', 'PG-NCD', 17),
    ('Sandaun', 'PG-SAN', 18),
    ('Southern Highlands', 'PG-SHM', 19),
    ('West New Britain', 'PG-WBK', 20),
    ('Western', 'PG-WPD', 21),
    ('Western Highlands', 'PG-WHM', 22)
) AS v(name, code, ord)
WHERE p.standard_code = 'PG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 巴哈马（BS）：32 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Acklins', 'BS-AK', 1),
    ('Berry Islands', 'BS-BY', 2),
    ('Bimini', 'BS-BI', 3),
    ('Black Point', 'BS-BP', 4),
    ('Cat Island', 'BS-CI', 5),
    ('Central Abaco', 'BS-CO', 6),
    ('Central Andros', 'BS-CS', 7),
    ('Central Eleuthera', 'BS-CE', 8),
    ('Crooked Island', 'BS-CK', 9),
    ('East Grand Bahama', 'BS-EG', 10),
    ('Exuma', 'BS-EX', 11),
    ('Freeport', 'BS-FP', 12),
    ('Grand Cay', 'BS-GC', 13),
    ('Harbour Island', 'BS-HI', 14),
    ('Hope Town', 'BS-HT', 15),
    ('Inagua', 'BS-IN', 16),
    ('Long Island', 'BS-LI', 17),
    ('Mangrove Cay', 'BS-MC', 18),
    ('Mayaguana', 'BS-MG', 19),
    ('Moore''s Island', 'BS-MI', 20),
    ('New Providence', 'BS-NP', 21),
    ('North Abaco', 'BS-NO', 22),
    ('North Andros', 'BS-NS', 23),
    ('North Eleuthera', 'BS-NE', 24),
    ('Ragged Island', 'BS-RI', 25),
    ('Rum Cay', 'BS-RC', 26),
    ('San Salvador Island', 'BS-SS', 27),
    ('South Abaco', 'BS-SO', 28),
    ('South Andros', 'BS-SA', 29),
    ('South Eleuthera', 'BS-SE', 30),
    ('Spanish Wells', 'BS-SW', 31),
    ('West Grand Bahama', 'BS-WG', 32)
) AS v(name, code, ord)
WHERE p.standard_code = 'BS' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 白俄罗斯（BY）：7 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Brest', 'BY-BR', 1),
    ('Gomel', 'BY-HO', 2),
    ('Grodno', 'BY-HR', 3),
    ('Minsk', 'BY-MI', 4),
    ('Minsk', 'BY-HM', 5),
    ('Mogilev', 'BY-MA', 6),
    ('Vitebsk', 'BY-VI', 7)
) AS v(name, code, ord)
WHERE p.standard_code = 'BY' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 百慕大（BM）：9 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Devonshire', 'BM-DEV', 1),
    ('Hamilton', 'BM-HA', 2),
    ('Paget', 'BM-PAG', 3),
    ('Pembroke', 'BM-PEM', 4),
    ('Saint George''s', 'BM-SGE', 5),
    ('Sandys', 'BM-SAN', 6),
    ('Smith''s', 'BM-SMI', 7),
    ('Southampton', 'BM-SOU', 8),
    ('Warwick', 'BM-WAR', 9)
) AS v(name, code, ord)
WHERE p.standard_code = 'BM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 巴基斯坦（PK）：7 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Azad Kashmir', 'PK-JK', 1),
    ('Balochistan', 'PK-BA', 2),
    ('Gilgit-Baltistan', 'PK-GB', 3),
    ('Islamabad', 'PK-IS', 4),
    ('Khyber Pakhtunkhwa', 'PK-KP', 5),
    ('Punjab', 'PK-PB', 6),
    ('Sindh', 'PK-SD', 7)
) AS v(name, code, ord)
WHERE p.standard_code = 'PK' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 巴拉圭（PY）：18 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alto Paraguay', 'PY-16', 1),
    ('Alto Paraná', 'PY-10', 2),
    ('Amambay', 'PY-13', 3),
    ('Asuncion', 'PY-ASU', 4),
    ('Boquerón', 'PY-19', 5),
    ('Caaguazú', 'PY-5', 6),
    ('Caazapá', 'PY-6', 7),
    ('Canindeyú', 'PY-14', 8),
    ('Central', 'PY-11', 9),
    ('Concepción', 'PY-1', 10),
    ('Cordillera', 'PY-3', 11),
    ('Guairá', 'PY-4', 12),
    ('Itapúa', 'PY-7', 13),
    ('Misiones', 'PY-8', 14),
    ('Ñeembucú', 'PY-12', 15),
    ('Paraguarí', 'PY-9', 16),
    ('Presidente Hayes', 'PY-15', 17),
    ('San Pedro', 'PY-2', 18)
) AS v(name, code, ord)
WHERE p.standard_code = 'PY' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 巴勒斯坦（PS）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bethlehem', 'PS-BTH', 1),
    ('Deir El Balah', 'PS-DEB', 2),
    ('Gaza', 'PS-GZA', 3),
    ('Hebron', 'PS-HBN', 4),
    ('Jenin', 'PS-JEN', 5),
    ('Jericho', 'PS-JRH', 6),
    ('Jerusalem (Quds)', 'PS-JEM', 7),
    ('Khan Yunis', 'PS-KYS', 8),
    ('Nablus', 'PS-NBS', 9),
    ('North Gaza', 'PS-NGZ', 10),
    ('Qalqilya', 'PS-QQA', 11),
    ('Rafah', 'PS-RFH', 12),
    ('Ramallah', 'PS-RBH', 13),
    ('Salfit', 'PS-SLT', 14),
    ('Tubas', 'PS-TBS', 15),
    ('Tulkarm', 'PS-TKM', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'PS' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 巴林（BH）：4 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Capital', 'BH-13', 1),
    ('Muharraq', 'BH-15', 2),
    ('Northern', 'BH-17', 3),
    ('Southern', 'BH-14', 4)
) AS v(name, code, ord)
WHERE p.standard_code = 'BH' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 巴拿马（PA）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bocas del Toro', 'PA-1', 1),
    ('Chiriquí Province', 'PA-4', 2),
    ('Coclé', 'PA-2', 3),
    ('Colón', 'PA-3', 4),
    ('Darién', 'PA-5', 5),
    ('Emberá-Wounaan Comarca', 'PA-EM', 6),
    ('Guna', 'PA-KY', 7),
    ('Herrera', 'PA-6', 8),
    ('Los Santos', 'PA-7', 9),
    ('Naso Tjër Di', 'PA-NT', 10),
    ('Ngöbe-Buglé Comarca', 'PA-NB', 11),
    ('Panamá', 'PA-8', 12),
    ('Panamá Oeste', 'PA-10', 13),
    ('Veraguas', 'PA-9', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'PA' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 保加利亚（BG）：28 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Blagoevgrad', 'BG-01', 1),
    ('Burgas', 'BG-02', 2),
    ('Dobrich', 'BG-08', 3),
    ('Gabrovo', 'BG-07', 4),
    ('Haskovo', 'BG-26', 5),
    ('Kardzhali', 'BG-09', 6),
    ('Kyustendil', 'BG-10', 7),
    ('Lovech', 'BG-11', 8),
    ('Montana', 'BG-12', 9),
    ('Pazardzhik', 'BG-13', 10),
    ('Pernik', 'BG-14', 11),
    ('Pleven', 'BG-15', 12),
    ('Plovdiv', 'BG-16', 13),
    ('Razgrad', 'BG-17', 14),
    ('Ruse', 'BG-18', 15),
    ('Shumen', 'BG-27', 16),
    ('Silistra', 'BG-19', 17),
    ('Sliven', 'BG-20', 18),
    ('Smolyan', 'BG-21', 19),
    ('Sofia', 'BG-23', 20),
    ('Sofia City', 'BG-22', 21),
    ('Stara Zagora', 'BG-24', 22),
    ('Targovishte', 'BG-25', 23),
    ('Varna', 'BG-03', 24),
    ('Veliko Tarnovo', 'BG-04', 25),
    ('Vidin', 'BG-05', 26),
    ('Vratsa', 'BG-06', 27),
    ('Yambol', 'BG-28', 28)
) AS v(name, code, ord)
WHERE p.standard_code = 'BG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 巴西（BR）：27 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Acre', 'BR-AC', 1),
    ('Alagoas', 'BR-AL', 2),
    ('Amapá', 'BR-AP', 3),
    ('Amazonas', 'BR-AM', 4),
    ('Bahia', 'BR-BA', 5),
    ('Ceará', 'BR-CE', 6),
    ('Distrito Federal', 'BR-DF', 7),
    ('Espírito Santo', 'BR-ES', 8),
    ('Goiás', 'BR-GO', 9),
    ('Maranhão', 'BR-MA', 10),
    ('Mato Grosso', 'BR-MT', 11),
    ('Mato Grosso do Sul', 'BR-MS', 12),
    ('Minas Gerais', 'BR-MG', 13),
    ('Pará', 'BR-PA', 14),
    ('Paraíba', 'BR-PB', 15),
    ('Paraná', 'BR-PR', 16),
    ('Pernambuco', 'BR-PE', 17),
    ('Piauí', 'BR-PI', 18),
    ('Rio de Janeiro', 'BR-RJ', 19),
    ('Rio Grande do Norte', 'BR-RN', 20),
    ('Rio Grande do Sul', 'BR-RS', 21),
    ('Rondônia', 'BR-RO', 22),
    ('Roraima', 'BR-RR', 23),
    ('Santa Catarina', 'BR-SC', 24),
    ('São Paulo', 'BR-SP', 25),
    ('Sergipe', 'BR-SE', 26),
    ('Tocantins', 'BR-TO', 27)
) AS v(name, code, ord)
WHERE p.standard_code = 'BR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 北马其顿（MK）：80 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Aerodrom', 'MK-801', 1),
    ('Aračinovo', 'MK-802', 2),
    ('Berovo', 'MK-201', 3),
    ('Bitola', 'MK-501', 4),
    ('Bogdanci', 'MK-401', 5),
    ('Bogovinje', 'MK-601', 6),
    ('Bosilovo', 'MK-402', 7),
    ('Brvenica', 'MK-602', 8),
    ('Butel', 'MK-803', 9),
    ('Čair', 'MK-815', 10),
    ('Čaška', 'MK-109', 11),
    ('Centar', 'MK-814', 12),
    ('Centar Župa', 'MK-313', 13),
    ('Češinovo-Obleševo', 'MK-210', 14),
    ('Čučer-Sandevo', 'MK-816', 15),
    ('Debar', 'MK-303', 16),
    ('Debarca', 'MK-304', 17),
    ('Delčevo', 'MK-203', 18),
    ('Demir Hisar', 'MK-502', 19),
    ('Demir Kapija', 'MK-103', 20),
    ('Dojran', 'MK-406', 21),
    ('Dolneni', 'MK-503', 22),
    ('Gazi Baba', 'MK-804', 23),
    ('Gevgelija', 'MK-405', 24),
    ('Gjorče Petrov', 'MK-805', 25),
    ('Gostivar', 'MK-604', 26),
    ('Gradsko', 'MK-102', 27),
    ('Ilinden', 'MK-807', 28),
    ('Jegunovce', 'MK-606', 29),
    ('Karbinci', 'MK-205', 30),
    ('Karpoš', 'MK-808', 31),
    ('Kavadarci', 'MK-104', 32),
    ('Kičevo', 'MK-307', 33),
    ('Kisela Voda', 'MK-809', 34),
    ('Kočani', 'MK-206', 35),
    ('Konče', 'MK-407', 36),
    ('Kratovo', 'MK-701', 37),
    ('Kriva Palanka', 'MK-702', 38),
    ('Krivogaštani', 'MK-504', 39),
    ('Kruševo', 'MK-505', 40),
    ('Kumanovo', 'MK-703', 41),
    ('Lipkovo', 'MK-704', 42),
    ('Lozovo', 'MK-105', 43),
    ('Makedonska Kamenica', 'MK-207', 44),
    ('Makedonski Brod', 'MK-308', 45),
    ('Mavrovo and Rostuša', 'MK-607', 46),
    ('Mogila', 'MK-506', 47),
    ('Negotino', 'MK-106', 48),
    ('Novaci', 'MK-507', 49),
    ('Novo Selo', 'MK-408', 50),
    ('Ohrid', 'MK-310', 51),
    ('Pehčevo', 'MK-208', 52),
    ('Petrovec', 'MK-810', 53),
    ('Plasnica', 'MK-311', 54),
    ('Prilep', 'MK-508', 55),
    ('Probištip', 'MK-209', 56),
    ('Radoviš', 'MK-409', 57),
    ('Rankovce', 'MK-705', 58),
    ('Resen', 'MK-509', 59),
    ('Rosoman', 'MK-107', 60),
    ('Saraj', 'MK-811', 61),
    ('Sopište', 'MK-812', 62),
    ('Staro Nagoričane', 'MK-706', 63),
    ('Štip', 'MK-211', 64),
    ('Struga', 'MK-312', 65),
    ('Strumica', 'MK-410', 66),
    ('Studeničani', 'MK-813', 67),
    ('Šuto Orizari', 'MK-817', 68),
    ('Sveti Nikole', 'MK-108', 69),
    ('Tearce', 'MK-608', 70),
    ('Tetovo', 'MK-609', 71),
    ('Valandovo', 'MK-403', 72),
    ('Vasilevo', 'MK-404', 73),
    ('Veles', 'MK-101', 74),
    ('Vevčani', 'MK-301', 75),
    ('Vinica', 'MK-202', 76),
    ('Vrapčište', 'MK-603', 77),
    ('Zelenikovo', 'MK-806', 78),
    ('Želino', 'MK-605', 79),
    ('Zrnovci', 'MK-204', 80)
) AS v(name, code, ord)
WHERE p.standard_code = 'MK' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 贝宁（BJ）：12 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alibori', 'BJ-AL', 1),
    ('Atakora', 'BJ-AK', 2),
    ('Atlantique', 'BJ-AQ', 3),
    ('Borgou', 'BJ-BO', 4),
    ('Collines', 'BJ-CO', 5),
    ('Donga', 'BJ-DO', 6),
    ('Kouffo', 'BJ-KO', 7),
    ('Littoral', 'BJ-LI', 8),
    ('Mono', 'BJ-MO', 9),
    ('Ouémé', 'BJ-OU', 10),
    ('Plateau', 'BJ-PL', 11),
    ('Zou', 'BJ-ZO', 12)
) AS v(name, code, ord)
WHERE p.standard_code = 'BJ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 比利时（BE）：13 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Antwerp', 'BE-VAN', 1),
    ('Brussels-Capital', 'BE-BRU', 2),
    ('East Flanders', 'BE-VOV', 3),
    ('Flanders', 'BE-VLG', 4),
    ('Flemish Brabant', 'BE-VBR', 5),
    ('Hainaut', 'BE-WHT', 6),
    ('Liège', 'BE-WLG', 7),
    ('Limburg', 'BE-VLI', 8),
    ('Luxembourg', 'BE-WLX', 9),
    ('Namur', 'BE-WNA', 10),
    ('Wallonia', 'BE-WAL', 11),
    ('Walloon Brabant', 'BE-WBR', 12),
    ('West Flanders', 'BE-VWV', 13)
) AS v(name, code, ord)
WHERE p.standard_code = 'BE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 秘鲁（PE）：26 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Amazonas', 'PE-AMA', 1),
    ('Áncash', 'PE-ANC', 2),
    ('Apurímac', 'PE-APU', 3),
    ('Arequipa', 'PE-ARE', 4),
    ('Ayacucho', 'PE-AYA', 5),
    ('Cajamarca', 'PE-CAJ', 6),
    ('Callao', 'PE-CAL', 7),
    ('Cusco', 'PE-CUS', 8),
    ('Huancavelica', 'PE-HUV', 9),
    ('Huanuco', 'PE-HUC', 10),
    ('Ica', 'PE-ICA', 11),
    ('Junín', 'PE-JUN', 12),
    ('La Libertad', 'PE-LAL', 13),
    ('Lambayeque', 'PE-LAM', 14),
    ('Lima', 'PE-LIM', 15),
    ('Loreto', 'PE-LOR', 16),
    ('Madre de Dios', 'PE-MDD', 17),
    ('Moquegua', 'PE-MOQ', 18),
    ('Municipalidad Metropolitana de Lima', 'PE-LMA', 19),
    ('Pasco', 'PE-PAS', 20),
    ('Piura', 'PE-PIU', 21),
    ('Puno', 'PE-PUN', 22),
    ('San Martín', 'PE-SAM', 23),
    ('Tacna', 'PE-TAC', 24),
    ('Tumbes', 'PE-TUM', 25),
    ('Ucayali', 'PE-UCA', 26)
) AS v(name, code, ord)
WHERE p.standard_code = 'PE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 冰岛（IS）：72 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Akranes', 'IS-AKN', 1),
    ('Akureyri', 'IS-AKU', 2),
    ('Árborg', 'IS-SFA', 3),
    ('Árneshreppur', 'IS-ARN', 4),
    ('Ásahreppur', 'IS-ASA', 5),
    ('Bláskógabyggð', 'IS-BLA', 6),
    ('Bolungarvík', 'IS-BOL', 7),
    ('Borgarbyggð', 'IS-BOG', 8),
    ('Capital', 'IS-1', 9),
    ('Dalabyggð', 'IS-DAB', 10),
    ('Dalvíkurbyggð', 'IS-DAV', 11),
    ('Eastern', 'IS-7', 12),
    ('Eyja- og Miklaholtshreppur', 'IS-EOM', 13),
    ('Eyjafjarðarsveit', 'IS-EYF', 14),
    ('Fjallabyggð', 'IS-FJL', 15),
    ('Fjarðabyggð', 'IS-FJD', 16),
    ('Fljótsdalshreppur', 'IS-FLR', 17),
    ('Flóahreppur', 'IS-FLA', 18),
    ('Garðabær', 'IS-GAR', 19),
    ('Grímsnes- og Grafningshreppur', 'IS-GOG', 20),
    ('Grindavík', 'IS-GRN', 21),
    ('Grundarfjörður', 'IS-GRU', 22),
    ('Grýtubakkahreppur', 'IS-GRY', 23),
    ('Hafnarfjörður', 'IS-HAF', 24),
    ('Hörgársveit', 'IS-HRG', 25),
    ('Hornafjörður', 'IS-SHF', 26),
    ('Hrunamannahreppur', 'IS-HRU', 27),
    ('Húnabyggð', 'IS-HUG', 28),
    ('Húnaþing vestra', 'IS-HUV', 29),
    ('Hvalfjarðarsveit', 'IS-HVA', 30),
    ('Hveragerði', 'IS-HVE', 31),
    ('Ísafjörður', 'IS-ISA', 32),
    ('Kaldrananeshreppur', 'IS-KAL', 33),
    ('Kjósarhreppur', 'IS-KJO', 34),
    ('Kópavogur', 'IS-KOP', 35),
    ('Langanesbyggð', 'IS-LAN', 36),
    ('Mosfellsbær', 'IS-MOS', 37),
    ('Múlaþing', 'IS-MUL', 38),
    ('Mýrdalshreppur', 'IS-MYR', 39),
    ('Norðurþing', 'IS-NOR', 40),
    ('Northeastern', 'IS-6', 41),
    ('Northwestern', 'IS-5', 42),
    ('Ölfus', 'IS-SOL', 43),
    ('Rangárþing eystra', 'IS-RGE', 44),
    ('Rangárþing ytra', 'IS-RGY', 45),
    ('Reykhólahreppur', 'IS-RHH', 46),
    ('Reykjanesbær', 'IS-RKN', 47),
    ('Reykjavík', 'IS-RKV', 48),
    ('Seltjarnarnes', 'IS-SEL', 49),
    ('Skaftárhreppur', 'IS-SKF', 50),
    ('Skagabyggð', 'IS-SKG', 51),
    ('Skagafjörður', 'IS-SKR', 52),
    ('Skagaströnd', 'IS-SSS', 53),
    ('Skeiða- og Gnúpverjahreppur', 'IS-SOG', 54),
    ('Skorradalshreppur', 'IS-SKO', 55),
    ('Snæfellsbær', 'IS-SNF', 56),
    ('Southern', 'IS-8', 57),
    ('Southern Peninsula', 'IS-2', 58),
    ('Strandabyggð', 'IS-STR', 59),
    ('Stykkishólmur', 'IS-STY', 60),
    ('Súðavík', 'IS-SDV', 61),
    ('Suðurnesjabær', 'IS-SDN', 62),
    ('Svalbardsstrandarhreppur', 'IS-SBT', 63),
    ('Tálknafjarðarhreppur', 'IS-TAL', 64),
    ('Tjörneshreppur', 'IS-TJO', 65),
    ('Vestmannaeyjar', 'IS-VEM', 66),
    ('Vesturbyggð', 'IS-VER', 67),
    ('Vogar', 'IS-SVG', 68),
    ('Vopnafjarðarhreppur', 'IS-VOP', 69),
    ('Western', 'IS-3', 70),
    ('Westfjords', 'IS-4', 71),
    ('Þingeyjarsveit', 'IS-THG', 72)
) AS v(name, code, ord)
WHERE p.standard_code = 'IS' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 博茨瓦纳（BW）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Central', 'BW-CE', 1),
    ('Chobe', 'BW-CH', 2),
    ('Francistown', 'BW-FR', 3),
    ('Gaborone', 'BW-GA', 4),
    ('Ghanzi', 'BW-GH', 5),
    ('Jwaneng', 'BW-JW', 6),
    ('Kgalagadi', 'BW-KG', 7),
    ('Kgatleng', 'BW-KL', 8),
    ('Kweneng', 'BW-KW', 9),
    ('Lobatse', 'BW-LO', 10),
    ('North-East', 'BW-NE', 11),
    ('North-West', 'BW-NW', 12),
    ('Selibe Phikwe', 'BW-SP', 13),
    ('South-East', 'BW-SE', 14),
    ('Southern', 'BW-SO', 15),
    ('Sowa Town', 'BW-ST', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'BW' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 波多黎各（PR）：78 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Adjuntas', 'PR-001', 1),
    ('Aguada', 'PR-003', 2),
    ('Aguadilla', 'PR-005', 3),
    ('Aguas Buenas', 'PR-007', 4),
    ('Aibonito', 'PR-009', 5),
    ('Añasco', 'PR-011', 6),
    ('Arecibo', 'PR-AR', 7),
    ('Arroyo', 'PR-015', 8),
    ('Barceloneta', 'PR-017', 9),
    ('Barranquitas', 'PR-019', 10),
    ('Bayamon', 'PR-BY', 11),
    ('Cabo Rojo', 'PR-023', 12),
    ('Caguas', 'PR-CG', 13),
    ('Camuy', 'PR-027', 14),
    ('Canóvanas', 'PR-029', 15),
    ('Carolina', 'PR-CL', 16),
    ('Cataño', 'PR-033', 17),
    ('Cayey', 'PR-035', 18),
    ('Ceiba', 'PR-037', 19),
    ('Ciales', 'PR-039', 20),
    ('Cidra', 'PR-041', 21),
    ('Coamo', 'PR-043', 22),
    ('Comerío', 'PR-045', 23),
    ('Corozal', 'PR-047', 24),
    ('Culebra', 'PR-049', 25),
    ('Dorado', 'PR-051', 26),
    ('Fajardo', 'PR-053', 27),
    ('Florida', 'PR-054', 28),
    ('Guánica', 'PR-055', 29),
    ('Guayama', 'PR-057', 30),
    ('Guayanilla', 'PR-059', 31),
    ('Guaynabo', 'PR-GN', 32),
    ('Gurabo', 'PR-063', 33),
    ('Hatillo', 'PR-065', 34),
    ('Hormigueros', 'PR-067', 35),
    ('Humacao', 'PR-069', 36),
    ('Isabela', 'PR-071', 37),
    ('Jayuya', 'PR-073', 38),
    ('Juana Díaz', 'PR-075', 39),
    ('Juncos', 'PR-077', 40),
    ('Lajas', 'PR-079', 41),
    ('Lares', 'PR-081', 42),
    ('Las Marías', 'PR-083', 43),
    ('Las Piedras', 'PR-085', 44),
    ('Loíza', 'PR-087', 45),
    ('Luquillo', 'PR-089', 46),
    ('Manatí', 'PR-091', 47),
    ('Maricao', 'PR-093', 48),
    ('Maunabo', 'PR-095', 49),
    ('Mayagüez', 'PR-MG', 50),
    ('Moca', 'PR-099', 51),
    ('Morovis', 'PR-101', 52),
    ('Naguabo', 'PR-103', 53),
    ('Naranjito', 'PR-105', 54),
    ('Orocovis', 'PR-107', 55),
    ('Patillas', 'PR-109', 56),
    ('Peñuelas', 'PR-111', 57),
    ('Ponce', 'PR-PO', 58),
    ('Quebradillas', 'PR-115', 59),
    ('Rincón', 'PR-117', 60),
    ('Río Grande', 'PR-119', 61),
    ('Sabana Grande', 'PR-121', 62),
    ('Salinas', 'PR-123', 63),
    ('San Germán', 'PR-125', 64),
    ('San Juan', 'PR-SJ', 65),
    ('San Lorenzo', 'PR-129', 66),
    ('San Sebastián', 'PR-131', 67),
    ('Santa Isabel', 'PR-133', 68),
    ('Toa Alta', 'PR-135', 69),
    ('Toa Baja', 'PR-TB', 70),
    ('Trujillo Alto', 'PR-TA', 71),
    ('Utuado', 'PR-141', 72),
    ('Vega Alta', 'PR-143', 73),
    ('Vega Baja', 'PR-145', 74),
    ('Vieques', 'PR-147', 75),
    ('Villalba', 'PR-149', 76),
    ('Yabucoa', 'PR-151', 77),
    ('Yauco', 'PR-153', 78)
) AS v(name, code, ord)
WHERE p.standard_code = 'PR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 波兰（PL）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Greater Poland', 'PL-30', 1),
    ('Holy Cross', 'PL-26', 2),
    ('Kuyavia-Pomerania', 'PL-04', 3),
    ('Lesser Poland', 'PL-12', 4),
    ('Lower Silesia', 'PL-02', 5),
    ('Lublin', 'PL-06', 6),
    ('Lubusz', 'PL-08', 7),
    ('Łódź', 'PL-10', 8),
    ('Mazovia', 'PL-14', 9),
    ('Podlaskie', 'PL-20', 10),
    ('Pomerania', 'PL-22', 11),
    ('Silesia', 'PL-24', 12),
    ('Subcarpathia', 'PL-18', 13),
    ('Upper Silesia', 'PL-16', 14),
    ('Warmia-Masuria', 'PL-28', 15),
    ('West Pomerania', 'PL-32', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'PL' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 玻利维亚（BO）：9 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Beni', 'BO-B', 1),
    ('Chuquisaca', 'BO-H', 2),
    ('Cochabamba', 'BO-C', 3),
    ('La Paz', 'BO-L', 4),
    ('Oruro', 'BO-O', 5),
    ('Pando', 'BO-N', 6),
    ('Potosí', 'BO-P', 7),
    ('Santa Cruz', 'BO-S', 8),
    ('Tarija', 'BO-T', 9)
) AS v(name, code, ord)
WHERE p.standard_code = 'BO' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 伯利兹（BZ）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Belize', 'BZ-BZ', 1),
    ('Cayo', 'BZ-CY', 2),
    ('Corozal', 'BZ-CZL', 3),
    ('Orange Walk', 'BZ-OW', 4),
    ('Stann Creek', 'BZ-SC', 5),
    ('Toledo', 'BZ-TOL', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'BZ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 波斯尼亚和黑塞哥维那（BA）：3 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Brčko', 'BA-BRC', 1),
    ('Federation of Bosnia and Herzegovina', 'BA-BIH', 2),
    ('Republika Srpska', 'BA-SRP', 3)
) AS v(name, code, ord)
WHERE p.standard_code = 'BA' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 不丹（BT）：20 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bumthang', 'BT-33', 1),
    ('Chukha', 'BT-12', 2),
    ('Dagana', 'BT-22', 3),
    ('Gasa', 'BT-GA', 4),
    ('Haa', 'BT-13', 5),
    ('Lhuntse', 'BT-44', 6),
    ('Mongar', 'BT-42', 7),
    ('Paro', 'BT-11', 8),
    ('Pemagatshel', 'BT-43', 9),
    ('Punakha', 'BT-23', 10),
    ('Samdrup Jongkhar', 'BT-45', 11),
    ('Samtse', 'BT-14', 12),
    ('Sarpang', 'BT-31', 13),
    ('Thimphu', 'BT-15', 14),
    ('Trashi Yangtse', 'BT-TY', 15),
    ('Trashigang', 'BT-41', 16),
    ('Trongsa', 'BT-32', 17),
    ('Tsirang', 'BT-21', 18),
    ('Wangdue Phodrang', 'BT-24', 19),
    ('Zhemgang', 'BT-34', 20)
) AS v(name, code, ord)
WHERE p.standard_code = 'BT' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 布基纳法索（BF）：58 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Balé', 'BF-BAL', 1),
    ('Bam', 'BF-BAM', 2),
    ('Banwa', 'BF-BAN', 3),
    ('Bazèga', 'BF-BAZ', 4),
    ('Boucle du Mouhoun', 'BF-01', 5),
    ('Bougouriba', 'BF-BGR', 6),
    ('Boulgou', 'BF-BLG', 7),
    ('Boulkiemde', 'BF-BLK', 8),
    ('Cascades', 'BF-02', 9),
    ('Centre', 'BF-03', 10),
    ('Centre-Est', 'BF-04', 11),
    ('Centre-Nord', 'BF-05', 12),
    ('Centre-Ouest', 'BF-06', 13),
    ('Centre-Sud', 'BF-07', 14),
    ('Comoé', 'BF-COM', 15),
    ('Est', 'BF-08', 16),
    ('Ganzourgou', 'BF-GAN', 17),
    ('Gnagna', 'BF-GNA', 18),
    ('Gourma', 'BF-GOU', 19),
    ('Hauts-Bassins', 'BF-09', 20),
    ('Houet', 'BF-HOU', 21),
    ('Ioba', 'BF-IOB', 22),
    ('Kadiogo', 'BF-KAD', 23),
    ('Kénédougou', 'BF-KEN', 24),
    ('Komondjari', 'BF-KMD', 25),
    ('Kompienga', 'BF-KMP', 26),
    ('Kossi', 'BF-KOS', 27),
    ('Koulpélogo', 'BF-KOP', 28),
    ('Kouritenga', 'BF-KOT', 29),
    ('Kourwéogo', 'BF-KOW', 30),
    ('Léraba', 'BF-LER', 31),
    ('Loroum', 'BF-LOR', 32),
    ('Mouhoun', 'BF-MOU', 33),
    ('Nahouri', 'BF-NAO', 34),
    ('Namentenga', 'BF-NAM', 35),
    ('Nayala', 'BF-NAY', 36),
    ('Nord', 'BF-10', 37),
    ('Noumbiel', 'BF-NOU', 38),
    ('Oubritenga', 'BF-OUB', 39),
    ('Oudalan', 'BF-OUD', 40),
    ('Passoré', 'BF-PAS', 41),
    ('Plateau-Central', 'BF-11', 42),
    ('Poni', 'BF-PON', 43),
    ('Sahel', 'BF-12', 44),
    ('Sanguié', 'BF-SNG', 45),
    ('Sanmatenga', 'BF-SMT', 46),
    ('Séno', 'BF-SEN', 47),
    ('Sissili', 'BF-SIS', 48),
    ('Soum', 'BF-SOM', 49),
    ('Sourou', 'BF-SOR', 50),
    ('Sud-Ouest', 'BF-13', 51),
    ('Tapoa', 'BF-TAP', 52),
    ('Tuy', 'BF-TUI', 53),
    ('Yagha', 'BF-YAG', 54),
    ('Yatenga', 'BF-YAT', 55),
    ('Ziro', 'BF-ZIR', 56),
    ('Zondoma', 'BF-ZON', 57),
    ('Zoundwéogo', 'BF-ZOU', 58)
) AS v(name, code, ord)
WHERE p.standard_code = 'BF' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 布隆迪（BI）：18 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bubanza', 'BI-BB', 1),
    ('Bujumbura Mairie', 'BI-BM', 2),
    ('Bujumbura Rural', 'BI-BL', 3),
    ('Bururi', 'BI-BR', 4),
    ('Cankuzo', 'BI-CA', 5),
    ('Cibitoke', 'BI-CI', 6),
    ('Gitega', 'BI-GI', 7),
    ('Karuzi', 'BI-KR', 8),
    ('Kayanza', 'BI-KY', 9),
    ('Kirundo', 'BI-KI', 10),
    ('Makamba', 'BI-MA', 11),
    ('Muramvya', 'BI-MU', 12),
    ('Muyinga', 'BI-MY', 13),
    ('Mwaro', 'BI-MW', 14),
    ('Ngozi', 'BI-NG', 15),
    ('Rumonge', 'BI-RM', 16),
    ('Rutana', 'BI-RT', 17),
    ('Ruyigi', 'BI-RY', 18)
) AS v(name, code, ord)
WHERE p.standard_code = 'BI' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 朝鲜（KP）：13 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Chagang', 'KP-04', 1),
    ('Kaesong', 'KP-15', 2),
    ('Kangwon', 'KP-07', 3),
    ('Nampho', 'KP-14', 4),
    ('North Hamgyong', 'KP-09', 5),
    ('North Hwanghae', 'KP-06', 6),
    ('North Pyongan', 'KP-03', 7),
    ('Pyongyang', 'KP-01', 8),
    ('Rason', 'KP-13', 9),
    ('Ryanggang', 'KP-10', 10),
    ('South Hamgyong', 'KP-08', 11),
    ('South Hwanghae', 'KP-05', 12),
    ('South Pyongan', 'KP-02', 13)
) AS v(name, code, ord)
WHERE p.standard_code = 'KP' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 赤道几内亚（GQ）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Annobón', 'GQ-AN', 1),
    ('Bioko Norte', 'GQ-BN', 2),
    ('Bioko Sur', 'GQ-BS', 3),
    ('Centro Sur', 'GQ-CS', 4),
    ('Djibloho', 'GQ-DJ', 5),
    ('Insular', 'GQ-I', 6),
    ('Kié-Ntem', 'GQ-KN', 7),
    ('Litoral', 'GQ-LI', 8),
    ('Río Muni', 'GQ-C', 9),
    ('Wele-Nzas', 'GQ-WN', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'GQ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 丹麦（DK）：5 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Capital Region of Denmark', 'DK-84', 1),
    ('Central Denmark', 'DK-82', 2),
    ('North Denmark', 'DK-81', 3),
    ('Southern Denmark', 'DK-83', 4),
    ('Zealand', 'DK-85', 5)
) AS v(name, code, ord)
WHERE p.standard_code = 'DK' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 德国（DE）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Baden-Württemberg', 'DE-BW', 1),
    ('Bavaria', 'DE-BY', 2),
    ('Berlin', 'DE-BE', 3),
    ('Brandenburg', 'DE-BB', 4),
    ('Bremen', 'DE-HB', 5),
    ('Hamburg', 'DE-HH', 6),
    ('Hessen', 'DE-HE', 7),
    ('Lower Saxony', 'DE-NI', 8),
    ('Mecklenburg-Vorpommern', 'DE-MV', 9),
    ('North Rhine-Westphalia', 'DE-NW', 10),
    ('Rhineland-Palatinate', 'DE-RP', 11),
    ('Saarland', 'DE-SL', 12),
    ('Saxony', 'DE-SN', 13),
    ('Saxony-Anhalt', 'DE-ST', 14),
    ('Schleswig-Holstein', 'DE-SH', 15),
    ('Thuringia', 'DE-TH', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'DE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 东帝汶（TL）：13 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Aileu', 'TL-AL', 1),
    ('Ainaro', 'TL-AN', 2),
    ('Baucau', 'TL-BA', 3),
    ('Bobonaro', 'TL-BO', 4),
    ('Cova Lima', 'TL-CO', 5),
    ('Dili', 'TL-DI', 6),
    ('Ermera', 'TL-ER', 7),
    ('Lautém', 'TL-LA', 8),
    ('Liquiçá', 'TL-LI', 9),
    ('Manatuto', 'TL-MT', 10),
    ('Manufahi', 'TL-MF', 11),
    ('Oecusse', 'TL-OE', 12),
    ('Viqueque', 'TL-VI', 13)
) AS v(name, code, ord)
WHERE p.standard_code = 'TL' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 多哥（TG）：5 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Centrale', 'TG-C', 1),
    ('Kara', 'TG-K', 2),
    ('Maritime', 'TG-M', 3),
    ('Plateaux', 'TG-P', 4),
    ('Savanes', 'TG-S', 5)
) AS v(name, code, ord)
WHERE p.standard_code = 'TG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 多米尼加共和国（DO）：42 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Azua', 'DO-02', 1),
    ('Baoruco', 'DO-03', 2),
    ('Barahona', 'DO-04', 3),
    ('Cibao Nordeste', 'DO-33', 4),
    ('Cibao Noroeste', 'DO-34', 5),
    ('Cibao Norte', 'DO-35', 6),
    ('Cibao Sur', 'DO-36', 7),
    ('Dajabón', 'DO-05', 8),
    ('Distrito Nacional', 'DO-01', 9),
    ('Duarte', 'DO-06', 10),
    ('El Seibo', 'DO-08', 11),
    ('El Valle', 'DO-37', 12),
    ('Elías Piña', 'DO-07', 13),
    ('Enriquillo', 'DO-38', 14),
    ('Espaillat', 'DO-09', 15),
    ('Hato Mayor', 'DO-30', 16),
    ('Hermanas Mirabal', 'DO-19', 17),
    ('Higuamo', 'DO-39', 18),
    ('Independencia', 'DO-10', 19),
    ('La Altagracia', 'DO-11', 20),
    ('La Romana', 'DO-12', 21),
    ('La Vega', 'DO-13', 22),
    ('María Trinidad Sánchez', 'DO-14', 23),
    ('Monseñor Nouel', 'DO-28', 24),
    ('Monte Cristi', 'DO-15', 25),
    ('Monte Plata', 'DO-29', 26),
    ('Ozama', 'DO-40', 27),
    ('Pedernales', 'DO-16', 28),
    ('Peravia', 'DO-17', 29),
    ('Puerto Plata', 'DO-18', 30),
    ('Samaná', 'DO-20', 31),
    ('San Cristóbal', 'DO-21', 32),
    ('San José de Ocoa', 'DO-31', 33),
    ('San Juan', 'DO-22', 34),
    ('San Pedro de Macorís', 'DO-23', 35),
    ('Sánchez Ramírez', 'DO-24', 36),
    ('Santiago', 'DO-25', 37),
    ('Santiago Rodríguez', 'DO-26', 38),
    ('Santo Domingo', 'DO-32', 39),
    ('Valdesia', 'DO-41', 40),
    ('Valverde', 'DO-27', 41),
    ('Yuma', 'DO-42', 42)
) AS v(name, code, ord)
WHERE p.standard_code = 'DO' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 多米尼克（DM）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Saint Andrew', 'DM-02', 1),
    ('Saint David', 'DM-03', 2),
    ('Saint George', 'DM-04', 3),
    ('Saint John', 'DM-05', 4),
    ('Saint Joseph', 'DM-06', 5),
    ('Saint Luke', 'DM-07', 6),
    ('Saint Mark', 'DM-08', 7),
    ('Saint Patrick', 'DM-09', 8),
    ('Saint Paul', 'DM-10', 9),
    ('Saint Peter', 'DM-11', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'DM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 厄瓜多尔（EC）：24 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Azuay', 'EC-A', 1),
    ('Bolívar', 'EC-B', 2),
    ('Cañar', 'EC-F', 3),
    ('Carchi', 'EC-C', 4),
    ('Chimborazo', 'EC-H', 5),
    ('Cotopaxi', 'EC-X', 6),
    ('El Oro', 'EC-O', 7),
    ('Esmeraldas', 'EC-E', 8),
    ('Galápagos', 'EC-W', 9),
    ('Guayas', 'EC-G', 10),
    ('Imbabura', 'EC-I', 11),
    ('Loja', 'EC-L', 12),
    ('Los Ríos', 'EC-R', 13),
    ('Manabí', 'EC-M', 14),
    ('Morona-Santiago', 'EC-S', 15),
    ('Napo', 'EC-N', 16),
    ('Orellana', 'EC-D', 17),
    ('Pastaza', 'EC-Y', 18),
    ('Pichincha', 'EC-P', 19),
    ('Santa Elena', 'EC-SE', 20),
    ('Santo Domingo de los Tsáchilas', 'EC-SD', 21),
    ('Sucumbíos', 'EC-U', 22),
    ('Tungurahua', 'EC-T', 23),
    ('Zamora Chinchipe', 'EC-Z', 24)
) AS v(name, code, ord)
WHERE p.standard_code = 'EC' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 厄立特里亚（ER）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Anseba', 'ER-AN', 1),
    ('Debub', 'ER-DU', 2),
    ('Gash-Barka', 'ER-GB', 3),
    ('Maekel', 'ER-MA', 4),
    ('Northern Red Sea', 'ER-SK', 5),
    ('Southern Red Sea', 'ER-DK', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'ER' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 俄罗斯（RU）：83 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Adygea', 'RU-AD', 1),
    ('Altai', 'RU-ALT', 2),
    ('Altai', 'RU-AL', 3),
    ('Amur', 'RU-AMU', 4),
    ('Arkhangelsk', 'RU-ARK', 5),
    ('Astrakhan', 'RU-AST', 6),
    ('Bashkortostan', 'RU-BA', 7),
    ('Belgorod', 'RU-BEL', 8),
    ('Bryansk', 'RU-BRY', 9),
    ('Buryatia', 'RU-BU', 10),
    ('Chechen', 'RU-CE', 11),
    ('Chelyabinsk', 'RU-CHE', 12),
    ('Chukotka', 'RU-CHU', 13),
    ('Chuvash', 'RU-CU', 14),
    ('Dagestan', 'RU-DA', 15),
    ('Ingushetia', 'RU-IN', 16),
    ('Irkutsk', 'RU-IRK', 17),
    ('Ivanovo', 'RU-IVA', 18),
    ('Jewish', 'RU-YEV', 19),
    ('Kabardino-Balkar', 'RU-KB', 20),
    ('Kaliningrad', 'RU-KGD', 21),
    ('Kalmykia', 'RU-KL', 22),
    ('Kaluga', 'RU-KLU', 23),
    ('Kamchatka', 'RU-KAM', 24),
    ('Karachay-Cherkess', 'RU-KC', 25),
    ('Karelia', 'RU-KR', 26),
    ('Kemerovo', 'RU-KEM', 27),
    ('Khabarovsk', 'RU-KHA', 28),
    ('Khakassia', 'RU-KK', 29),
    ('Khanty-Mansi', 'RU-KHM', 30),
    ('Kirov', 'RU-KIR', 31),
    ('Komi', 'RU-KO', 32),
    ('Kostroma', 'RU-KOS', 33),
    ('Krasnodar', 'RU-KDA', 34),
    ('Krasnoyarsk', 'RU-KYA', 35),
    ('Kurgan', 'RU-KGN', 36),
    ('Kursk', 'RU-KRS', 37),
    ('Leningrad', 'RU-LEN', 38),
    ('Lipetsk', 'RU-LIP', 39),
    ('Magadan', 'RU-MAG', 40),
    ('Mari El', 'RU-ME', 41),
    ('Mordovia', 'RU-MO', 42),
    ('Moscow', 'RU-MOS', 43),
    ('Moscow', 'RU-MOW', 44),
    ('Murmansk', 'RU-MUR', 45),
    ('Nenets', 'RU-NEN', 46),
    ('Nizhny Novgorod', 'RU-NIZ', 47),
    ('North Ossetia-Alania', 'RU-SE', 48),
    ('Novgorod', 'RU-NGR', 49),
    ('Novosibirsk', 'RU-NVS', 50),
    ('Omsk', 'RU-OMS', 51),
    ('Orenburg', 'RU-ORE', 52),
    ('Oryol', 'RU-ORL', 53),
    ('Penza', 'RU-PNZ', 54),
    ('Perm', 'RU-PER', 55),
    ('Primorsky', 'RU-PRI', 56),
    ('Pskov', 'RU-PSK', 57),
    ('Rostov', 'RU-ROS', 58),
    ('Ryazan', 'RU-RYA', 59),
    ('Saint Petersburg', 'RU-SPE', 60),
    ('Sakha', 'RU-SA', 61),
    ('Sakhalin', 'RU-SAK', 62),
    ('Samara', 'RU-SAM', 63),
    ('Saratov', 'RU-SAR', 64),
    ('Smolensk', 'RU-SMO', 65),
    ('Stavropol', 'RU-STA', 66),
    ('Sverdlovsk', 'RU-SVE', 67),
    ('Tambov', 'RU-TAM', 68),
    ('Tatarstan', 'RU-TA', 69),
    ('Tomsk', 'RU-TOM', 70),
    ('Tula', 'RU-TUL', 71),
    ('Tuva', 'RU-TY', 72),
    ('Tver', 'RU-TVE', 73),
    ('Tyumen', 'RU-TYU', 74),
    ('Udmurt', 'RU-UD', 75),
    ('Ulyanovsk', 'RU-ULY', 76),
    ('Vladimir', 'RU-VLA', 77),
    ('Volgograd Oblast', 'RU-VGG', 78),
    ('Vologda', 'RU-VLG', 79),
    ('Voronezh', 'RU-VOR', 80),
    ('Yamalo-Nenets', 'RU-YAN', 81),
    ('Yaroslavl', 'RU-YAR', 82),
    ('Zabaykalsky', 'RU-ZAB', 83)
) AS v(name, code, ord)
WHERE p.standard_code = 'RU' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 法国（FR）：124 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ain', 'FR-01', 1),
    ('Aisne', 'FR-02', 2),
    ('Allier', 'FR-03', 3),
    ('Alpes-de-Haute-Provence', 'FR-04', 4),
    ('Alpes-Maritimes', 'FR-06', 5),
    ('Alsace', 'FR-6AE', 6),
    ('Ardèche', 'FR-07', 7),
    ('Ardennes', 'FR-08', 8),
    ('Ariège', 'FR-09', 9),
    ('Aube', 'FR-10', 10),
    ('Aude', 'FR-11', 11),
    ('Auvergne-Rhône-Alpes', 'FR-ARA', 12),
    ('Aveyron', 'FR-12', 13),
    ('Bas-Rhin', 'FR-67', 14),
    ('Bouches-du-Rhône', 'FR-13', 15),
    ('Bourgogne-Franche-Comté', 'FR-BFC', 16),
    ('Bretagne', 'FR-BRE', 17),
    ('Calvados', 'FR-14', 18),
    ('Cantal', 'FR-15', 19),
    ('Centre-Val de Loire', 'FR-CVL', 20),
    ('Charente', 'FR-16', 21),
    ('Charente-Maritime', 'FR-17', 22),
    ('Cher', 'FR-18', 23),
    ('Clipperton', 'FR-CP', 24),
    ('Corrèze', 'FR-19', 25),
    ('Corse', 'FR-20R', 26),
    ('Corse-du-Sud', 'FR-2A', 27),
    ('Côte-d''Or', 'FR-21', 28),
    ('Côtes-d''Armor', 'FR-22', 29),
    ('Creuse', 'FR-23', 30),
    ('Deux-Sèvres', 'FR-79', 31),
    ('Dordogne', 'FR-24', 32),
    ('Doubs', 'FR-25', 33),
    ('Drôme', 'FR-26', 34),
    ('Essonne', 'FR-91', 35),
    ('Eure', 'FR-27', 36),
    ('Eure-et-Loir', 'FR-28', 37),
    ('Finistère', 'FR-29', 38),
    ('French Guiana', 'FR-973', 39),
    ('French Polynesia', 'FR-PF', 40),
    ('French Southern and Antarctic Lands', 'FR-TF', 41),
    ('Gard', 'FR-30', 42),
    ('Gers', 'FR-32', 43),
    ('Gironde', 'FR-33', 44),
    ('Grand-Est', 'FR-GES', 45),
    ('Guadeloupe', 'FR-971', 46),
    ('Haut-Rhin', 'FR-68', 47),
    ('Haute-Corse', 'FR-2B', 48),
    ('Haute-Garonne', 'FR-31', 49),
    ('Haute-Loire', 'FR-43', 50),
    ('Haute-Marne', 'FR-52', 51),
    ('Haute-Saône', 'FR-70', 52),
    ('Haute-Savoie', 'FR-74', 53),
    ('Haute-Vienne', 'FR-87', 54),
    ('Hautes-Alpes', 'FR-05', 55),
    ('Hautes-Pyrénées', 'FR-65', 56),
    ('Hauts-de-France', 'FR-HDF', 57),
    ('Hauts-de-Seine', 'FR-92', 58),
    ('Hérault', 'FR-34', 59),
    ('Île-de-France', 'FR-IDF', 60),
    ('Ille-et-Vilaine', 'FR-35', 61),
    ('Indre', 'FR-36', 62),
    ('Indre-et-Loire', 'FR-37', 63),
    ('Isère', 'FR-38', 64),
    ('Jura', 'FR-39', 65),
    ('La Réunion', 'FR-974', 66),
    ('Landes', 'FR-40', 67),
    ('Loir-et-Cher', 'FR-41', 68),
    ('Loire', 'FR-42', 69),
    ('Loire-Atlantique', 'FR-44', 70),
    ('Loiret', 'FR-45', 71),
    ('Lot', 'FR-46', 72),
    ('Lot-et-Garonne', 'FR-47', 73),
    ('Lozère', 'FR-48', 74),
    ('Maine-et-Loire', 'FR-49', 75),
    ('Manche', 'FR-50', 76),
    ('Marne', 'FR-51', 77),
    ('Martinique', 'FR-972', 78),
    ('Mayenne', 'FR-53', 79),
    ('Mayotte', 'FR-976', 80),
    ('Métropole de Lyon', 'FR-69M', 81),
    ('Meurthe-et-Moselle', 'FR-54', 82),
    ('Meuse', 'FR-55', 83),
    ('Morbihan', 'FR-56', 84),
    ('Moselle', 'FR-57', 85),
    ('Nièvre', 'FR-58', 86),
    ('Nord', 'FR-59', 87),
    ('Normandie', 'FR-NOR', 88),
    ('Nouvelle-Aquitaine', 'FR-NAQ', 89),
    ('Nouvelle-Calédonie', 'FR-NC', 90),
    ('Occitanie', 'FR-OCC', 91),
    ('Oise', 'FR-60', 92),
    ('Orne', 'FR-61', 93),
    ('Paris', 'FR-75C', 94),
    ('Pas-de-Calais', 'FR-62', 95),
    ('Pays-de-la-Loire', 'FR-PDL', 96),
    ('Provence-Alpes-Côte-d’Azur', 'FR-PAC', 97),
    ('Puy-de-Dôme', 'FR-63', 98),
    ('Pyrénées-Atlantiques', 'FR-64', 99),
    ('Pyrénées-Orientales', 'FR-66', 100),
    ('Rhône', 'FR-69', 101),
    ('Saint Pierre and Miquelon', 'FR-PM', 102),
    ('Saint-Barthélemy', 'FR-BL', 103),
    ('Saint-Martin', 'FR-MF', 104),
    ('Saône-et-Loire', 'FR-71', 105),
    ('Sarthe', 'FR-72', 106),
    ('Savoie', 'FR-73', 107),
    ('Seine-et-Marne', 'FR-77', 108),
    ('Seine-Maritime', 'FR-76', 109),
    ('Seine-Saint-Denis', 'FR-93', 110),
    ('Somme', 'FR-80', 111),
    ('Tarn', 'FR-81', 112),
    ('Tarn-et-Garonne', 'FR-82', 113),
    ('Territoire de Belfort', 'FR-90', 114),
    ('Val-d''Oise', 'FR-95', 115),
    ('Val-de-Marne', 'FR-94', 116),
    ('Var', 'FR-83', 117),
    ('Vaucluse', 'FR-84', 118),
    ('Vendée', 'FR-85', 119),
    ('Vienne', 'FR-86', 120),
    ('Vosges', 'FR-88', 121),
    ('Wallis and Futuna', 'FR-WF', 122),
    ('Yonne', 'FR-89', 123),
    ('Yvelines', 'FR-78', 124)
) AS v(name, code, ord)
WHERE p.standard_code = 'FR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 法罗群岛（FO）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Eysturoy', 'FO-EY', 1),
    ('Northern Isles', 'FO-NO', 2),
    ('Sandoy', 'FO-SA', 3),
    ('Streymoy', 'FO-ST', 4),
    ('Suðuroy', 'FO-SU', 5),
    ('Vágar', 'FO-VA', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'FO' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 法属波利尼西亚（PF）：5 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Austral Islands', 'PF-01', 1),
    ('Leeward Islands', 'PF-02', 2),
    ('Marquesas Islands', 'PF-03', 3),
    ('Tuamotu-Gambier', 'PF-04', 4),
    ('Windward Islands', 'PF-05', 5)
) AS v(name, code, ord)
WHERE p.standard_code = 'PF' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 法属圭亚那（GF）：1 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Guyane', 'FR-973', 1)
) AS v(name, code, ord)
WHERE p.standard_code = 'GF' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 法属南部领地（TF）：5 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Adélie Land', 'TF-01', 1),
    ('Crozet Islands', 'TF-02', 2),
    ('Kerguelen Islands', 'TF-03', 3),
    ('Saint-Paul and Amsterdam Islands', 'TF-04', 4),
    ('Scattered Islands', 'TF-05', 5)
) AS v(name, code, ord)
WHERE p.standard_code = 'TF' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 法属圣马丁（MF）：1 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Saint-Martin', 'FR-MF', 1)
) AS v(name, code, ord)
WHERE p.standard_code = 'MF' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 斐济（FJ）：19 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ba', 'FJ-01', 1),
    ('Bua', 'FJ-02', 2),
    ('Cakaudrove', 'FJ-03', 3),
    ('Central', 'FJ-C', 4),
    ('Eastern', 'FJ-E', 5),
    ('Kadavu', 'FJ-04', 6),
    ('Lau', 'FJ-05', 7),
    ('Lomaiviti', 'FJ-06', 8),
    ('Macuata', 'FJ-07', 9),
    ('Nadroga-Navosa', 'FJ-08', 10),
    ('Naitasiri', 'FJ-09', 11),
    ('Namosi', 'FJ-10', 12),
    ('Northern', 'FJ-N', 13),
    ('Ra', 'FJ-11', 14),
    ('Rewa', 'FJ-12', 15),
    ('Rotuma', 'FJ-R', 16),
    ('Serua', 'FJ-13', 17),
    ('Tailevu', 'FJ-14', 18),
    ('Western', 'FJ-W', 19)
) AS v(name, code, ord)
WHERE p.standard_code = 'FJ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 菲律宾（PH）：101 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Abra', 'PH-ABR', 1),
    ('Agusan del Norte', 'PH-AGN', 2),
    ('Agusan del Sur', 'PH-AGS', 3),
    ('Aklan', 'PH-AKL', 4),
    ('Albay', 'PH-ALB', 5),
    ('Antique', 'PH-ANT', 6),
    ('Apayao', 'PH-APA', 7),
    ('Aurora', 'PH-AUR', 8),
    ('Autonomous Region in Muslim Mindanao', 'PH-14', 9),
    ('Basilan', 'PH-BAS', 10),
    ('Bataan', 'PH-BAN', 11),
    ('Batanes', 'PH-BTN', 12),
    ('Batangas', 'PH-BTG', 13),
    ('Benguet', 'PH-BEN', 14),
    ('Bicol', 'PH-05', 15),
    ('Biliran', 'PH-BIL', 16),
    ('Bohol', 'PH-BOH', 17),
    ('Bukidnon', 'PH-BUK', 18),
    ('Bulacan', 'PH-BUL', 19),
    ('Cagayan', 'PH-CAG', 20),
    ('Cagayan Valley', 'PH-02', 21),
    ('Calabarzon', 'PH-40', 22),
    ('Camarines Norte', 'PH-CAN', 23),
    ('Camarines Sur', 'PH-CAS', 24),
    ('Camiguin', 'PH-CAM', 25),
    ('Capiz', 'PH-CAP', 26),
    ('Caraga', 'PH-13', 27),
    ('Catanduanes', 'PH-CAT', 28),
    ('Cavite', 'PH-CAV', 29),
    ('Cebu', 'PH-CEB', 30),
    ('Central Luzon', 'PH-03', 31),
    ('Central Visayas', 'PH-07', 32),
    ('Cordillera Administrative', 'PH-15', 33),
    ('Cotabato', 'PH-NCO', 34),
    ('Davao', 'PH-11', 35),
    ('Davao de Oro', 'PH-COM', 36),
    ('Davao del Norte', 'PH-DAV', 37),
    ('Davao del Sur', 'PH-DAS', 38),
    ('Davao Occidental', 'PH-DVO', 39),
    ('Davao Oriental', 'PH-DAO', 40),
    ('Dinagat Islands', 'PH-DIN', 41),
    ('Eastern Samar', 'PH-EAS', 42),
    ('Eastern Visayas', 'PH-08', 43),
    ('Guimaras', 'PH-GUI', 44),
    ('Ifugao', 'PH-IFU', 45),
    ('Ilocos', 'PH-01', 46),
    ('Ilocos Norte', 'PH-ILN', 47),
    ('Ilocos Sur', 'PH-ILS', 48),
    ('Iloilo', 'PH-ILI', 49),
    ('Isabela', 'PH-ISA', 50),
    ('Kalinga', 'PH-KAL', 51),
    ('La Union', 'PH-LUN', 52),
    ('Laguna', 'PH-LAG', 53),
    ('Lanao del Norte', 'PH-LAN', 54),
    ('Lanao del Sur', 'PH-LAS', 55),
    ('Leyte', 'PH-LEY', 56),
    ('Maguindanao del Norte', 'PH-MGN', 57),
    ('Maguindanao del Sur', 'PH-MGS', 58),
    ('Marinduque', 'PH-MAD', 59),
    ('Masbate', 'PH-MAS', 60),
    ('Mimaropa', 'PH-41', 61),
    ('Misamis Occidental', 'PH-MSC', 62),
    ('Misamis Oriental', 'PH-MSR', 63),
    ('Mountain Province', 'PH-MOU', 64),
    ('National Capital Region (Metro Manila)', 'PH-00', 65),
    ('Negros Island Region', 'PH-18', 66),
    ('Negros Occidental', 'PH-NEC', 67),
    ('Negros Oriental', 'PH-NER', 68),
    ('Northern Mindanao', 'PH-10', 69),
    ('Northern Samar', 'PH-NSA', 70),
    ('Nueva Ecija', 'PH-NUE', 71),
    ('Nueva Vizcaya', 'PH-NUV', 72),
    ('Occidental Mindoro', 'PH-MDC', 73),
    ('Oriental Mindoro', 'PH-MDR', 74),
    ('Palawan', 'PH-PLW', 75),
    ('Pampanga', 'PH-PAM', 76),
    ('Pangasinan', 'PH-PAN', 77),
    ('Quezon', 'PH-QUE', 78),
    ('Quirino', 'PH-QUI', 79),
    ('Rizal', 'PH-RIZ', 80),
    ('Romblon', 'PH-ROM', 81),
    ('Sarangani', 'PH-SAR', 82),
    ('Siquijor', 'PH-SIG', 83),
    ('Soccsksargen', 'PH-12', 84),
    ('Sorsogon', 'PH-SOR', 85),
    ('South Cotabato', 'PH-SCO', 86),
    ('Southern Leyte', 'PH-SLE', 87),
    ('Special Geographic Area', 'PH-19999', 88),
    ('Sultan Kudarat', 'PH-SUK', 89),
    ('Sulu', 'PH-SLU', 90),
    ('Surigao del Norte', 'PH-SUN', 91),
    ('Surigao del Sur', 'PH-SUR', 92),
    ('Tarlac', 'PH-TAR', 93),
    ('Tawi-Tawi', 'PH-TAW', 94),
    ('Western Samar', 'PH-WSA', 95),
    ('Western Visayas', 'PH-06', 96),
    ('Zambales', 'PH-ZMB', 97),
    ('Zamboanga del Norte', 'PH-ZAN', 98),
    ('Zamboanga del Sur', 'PH-ZAS', 99),
    ('Zamboanga Peninsula', 'PH-09', 100),
    ('Zamboanga Sibugay', 'PH-ZSI', 101)
) AS v(name, code, ord)
WHERE p.standard_code = 'PH' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 芬兰（FI）：19 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Åland Islands', 'FI-01', 1),
    ('Central Finland', 'FI-08', 2),
    ('Central Ostrobothnia', 'FI-07', 3),
    ('Finland Proper', 'FI-19', 4),
    ('Kainuu', 'FI-05', 5),
    ('Kymenlaakso', 'FI-09', 6),
    ('Lapland', 'FI-10', 7),
    ('North Karelia', 'FI-13', 8),
    ('Northern Ostrobothnia', 'FI-14', 9),
    ('Northern Savonia', 'FI-15', 10),
    ('Ostrobothnia', 'FI-12', 11),
    ('Päijänne Tavastia', 'FI-16', 12),
    ('Pirkanmaa', 'FI-11', 13),
    ('Satakunta', 'FI-17', 14),
    ('South Karelia', 'FI-02', 15),
    ('Southern Ostrobothnia', 'FI-03', 16),
    ('Southern Savonia', 'FI-04', 17),
    ('Tavastia Proper', 'FI-06', 18),
    ('Uusimaa', 'FI-18', 19)
) AS v(name, code, ord)
WHERE p.standard_code = 'FI' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 佛得角（CV）：24 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Barlavento Islands', 'CV-B', 1),
    ('Boa Vista', 'CV-BV', 2),
    ('Brava', 'CV-BR', 3),
    ('Maio', 'CV-MA', 4),
    ('Mosteiros', 'CV-MO', 5),
    ('Paul', 'CV-PA', 6),
    ('Porto Novo', 'CV-PN', 7),
    ('Praia', 'CV-PR', 8),
    ('Ribeira Brava', 'CV-RB', 9),
    ('Ribeira Grande', 'CV-RG', 10),
    ('Ribeira Grande de Santiago', 'CV-RS', 11),
    ('Sal', 'CV-SL', 12),
    ('Santa Catarina', 'CV-CA', 13),
    ('Santa Catarina do Fogo', 'CV-CF', 14),
    ('Santa Cruz', 'CV-CR', 15),
    ('São Domingos', 'CV-SD', 16),
    ('São Filipe', 'CV-SF', 17),
    ('São Lourenço dos Órgãos', 'CV-SO', 18),
    ('São Miguel', 'CV-SM', 19),
    ('São Salvador do Mundo', 'CV-SS', 20),
    ('São Vicente', 'CV-SV', 21),
    ('Sotavento Islands', 'CV-S', 22),
    ('Tarrafal', 'CV-TA', 23),
    ('Tarrafal de São Nicolau', 'CV-TS', 24)
) AS v(name, code, ord)
WHERE p.standard_code = 'CV' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 冈比亚（GM）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Banjul', 'GM-B', 1),
    ('Central River', 'GM-M', 2),
    ('Lower River', 'GM-L', 3),
    ('North Bank', 'GM-N', 4),
    ('Upper River', 'GM-U', 5),
    ('West Coast', 'GM-W', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'GM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 刚果（布）（CG）：12 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bouenza', 'CG-11', 1),
    ('Brazzaville', 'CG-BZV', 2),
    ('Cuvette', 'CG-8', 3),
    ('Cuvette-Ouest', 'CG-15', 4),
    ('Kouilou', 'CG-5', 5),
    ('Lékoumou', 'CG-2', 6),
    ('Likouala', 'CG-7', 7),
    ('Niari', 'CG-9', 8),
    ('Plateaux', 'CG-14', 9),
    ('Pointe-Noire', 'CG-16', 10),
    ('Pool', 'CG-12', 11),
    ('Sangha', 'CG-13', 12)
) AS v(name, code, ord)
WHERE p.standard_code = 'CG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 刚果（金）（CD）：26 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bas-Uélé', 'CD-BU', 1),
    ('Équateur', 'CD-EQ', 2),
    ('Haut-Katanga', 'CD-HK', 3),
    ('Haut-Lomami', 'CD-HL', 4),
    ('Haut-Uélé', 'CD-HU', 5),
    ('Ituri', 'CD-IT', 6),
    ('Kasaï', 'CD-KS', 7),
    ('Kasaï Central', 'CD-KC', 8),
    ('Kasaï Oriental', 'CD-KE', 9),
    ('Kinshasa', 'CD-KN', 10),
    ('Kongo Central', 'CD-BC', 11),
    ('Kwango', 'CD-KG', 12),
    ('Kwilu', 'CD-KL', 13),
    ('Lomami', 'CD-LO', 14),
    ('Lualaba', 'CD-LU', 15),
    ('Mai-Ndombe', 'CD-MN', 16),
    ('Maniema', 'CD-MA', 17),
    ('Mongala', 'CD-MO', 18),
    ('Nord-Kivu', 'CD-NK', 19),
    ('Nord-Ubangi', 'CD-NU', 20),
    ('Sankuru', 'CD-SA', 21),
    ('Sud-Kivu', 'CD-SK', 22),
    ('Sud-Ubangi', 'CD-SU', 23),
    ('Tanganyika', 'CD-TA', 24),
    ('Tshopo', 'CD-TO', 25),
    ('Tshuapa', 'CD-TU', 26)
) AS v(name, code, ord)
WHERE p.standard_code = 'CD' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 格陵兰（GL）：5 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Avannaata', 'GL-AV', 1),
    ('Kujalleq', 'GL-KU', 2),
    ('Qeqertalik', 'GL-QT', 3),
    ('Qeqqata', 'GL-QE', 4),
    ('Sermersooq', 'GL-SM', 5)
) AS v(name, code, ord)
WHERE p.standard_code = 'GL' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 格林纳达（GD）：7 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Carriacou', 'GD-10', 1),
    ('Saint Andrew', 'GD-01', 2),
    ('Saint David', 'GD-02', 3),
    ('Saint George', 'GD-03', 4),
    ('Saint John', 'GD-04', 5),
    ('Saint Mark', 'GD-05', 6),
    ('Saint Patrick', 'GD-06', 7)
) AS v(name, code, ord)
WHERE p.standard_code = 'GD' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 格鲁吉亚（GE）：12 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Abkhazia', 'GE-AB', 1),
    ('Adjara', 'GE-AJ', 2),
    ('Guria', 'GE-GU', 3),
    ('Imereti', 'GE-IM', 4),
    ('Kakheti', 'GE-KA', 5),
    ('Kvemo Kartli', 'GE-KK', 6),
    ('Mtskheta-Mtianeti', 'GE-MM', 7),
    ('Racha-Lechkhumi and Kvemo Svaneti', 'GE-RL', 8),
    ('Samegrelo-Zemo Svaneti', 'GE-SZ', 9),
    ('Samtskhe-Javakheti', 'GE-SJ', 10),
    ('Shida Kartli', 'GE-SK', 11),
    ('Tbilisi', 'GE-TB', 12)
) AS v(name, code, ord)
WHERE p.standard_code = 'GE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 哥伦比亚（CO）：33 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Amazonas', 'CO-AMA', 1),
    ('Antioquia', 'CO-ANT', 2),
    ('Arauca', 'CO-ARA', 3),
    ('Atlántico', 'CO-ATL', 4),
    ('Bogotá D.C.', 'CO-DC', 5),
    ('Bolívar', 'CO-BOL', 6),
    ('Boyacá', 'CO-BOY', 7),
    ('Caldas', 'CO-CAL', 8),
    ('Caquetá', 'CO-CAQ', 9),
    ('Casanare', 'CO-CAS', 10),
    ('Cauca', 'CO-CAU', 11),
    ('Cesar', 'CO-CES', 12),
    ('Chocó', 'CO-CHO', 13),
    ('Córdoba', 'CO-COR', 14),
    ('Cundinamarca', 'CO-CUN', 15),
    ('Guainía', 'CO-GUA', 16),
    ('Guaviare', 'CO-GUV', 17),
    ('Huila', 'CO-HUI', 18),
    ('La Guajira', 'CO-LAG', 19),
    ('Magdalena', 'CO-MAG', 20),
    ('Meta', 'CO-MET', 21),
    ('Nariño', 'CO-NAR', 22),
    ('Norte de Santander', 'CO-NSA', 23),
    ('Putumayo', 'CO-PUT', 24),
    ('Quindío', 'CO-QUI', 25),
    ('Risaralda', 'CO-RIS', 26),
    ('San Andrés, Providencia y Santa Catalina', 'CO-SAP', 27),
    ('Santander', 'CO-SAN', 28),
    ('Sucre', 'CO-SUC', 29),
    ('Tolima', 'CO-TOL', 30),
    ('Valle del Cauca', 'CO-VAC', 31),
    ('Vaupés', 'CO-VAU', 32),
    ('Vichada', 'CO-VID', 33)
) AS v(name, code, ord)
WHERE p.standard_code = 'CO' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 根西岛（GG）：12 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alderney', 'GG-04', 1),
    ('Castel', 'GG-01', 2),
    ('Forest', 'GG-02', 3),
    ('Sark', 'GG-07', 4),
    ('St Andrew', 'GG-03', 5),
    ('St Martin', 'GG-05', 6),
    ('St Peter Port', 'GG-06', 7),
    ('St Pierre du Bois', 'GG-08', 8),
    ('St Sampson', 'GG-09', 9),
    ('St Saviour', 'GG-10', 10),
    ('Torteval', 'GG-11', 11),
    ('Vale', 'GG-12', 12)
) AS v(name, code, ord)
WHERE p.standard_code = 'GG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 哥斯达黎加（CR）：7 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alajuela', 'CR-A', 1),
    ('Cartago', 'CR-C', 2),
    ('Guanacaste', 'CR-G', 3),
    ('Heredia', 'CR-H', 4),
    ('Limón', 'CR-L', 5),
    ('Puntarenas', 'CR-P', 6),
    ('San José', 'CR-SJ', 7)
) AS v(name, code, ord)
WHERE p.standard_code = 'CR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 瓜德罗普（GP）：2 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Basse-Terre', 'GP-01', 1),
    ('Pointe-à-Pitre', 'GP-02', 2)
) AS v(name, code, ord)
WHERE p.standard_code = 'GP' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 关岛（GU）：19 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Agana Heights', 'GU-01', 1),
    ('Asan-Maina', 'GU-02', 2),
    ('Barrigada', 'GU-03', 3),
    ('Chalan Pago-Ordot', 'GU-04', 4),
    ('Dededo', 'GU-05', 5),
    ('Hågat', 'GU-06', 6),
    ('Hagåtña', 'GU-12', 7),
    ('Inarajan (Inalåhan)', 'GU-08', 8),
    ('Mangilao', 'GU-10', 9),
    ('Merizo (Malesso)', 'GU-09', 10),
    ('Mongmong-Toto-Maite', 'GU-11', 11),
    ('Piti', 'GU-13', 12),
    ('Santa Rita (Sånta Rita-Sumai)', 'GU-14', 13),
    ('Sinajana', 'GU-15', 14),
    ('Talofofo (Talo''fo''fo)', 'GU-16', 15),
    ('Tamuning', 'GU-17', 16),
    ('Umatac (Humåtak)', 'GU-07', 17),
    ('Yigo', 'GU-18', 18),
    ('Yona', 'GU-19', 19)
) AS v(name, code, ord)
WHERE p.standard_code = 'GU' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 古巴（CU）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Artemisa', 'CU-15', 1),
    ('Camagüey', 'CU-09', 2),
    ('Ciego de Ávila', 'CU-08', 3),
    ('Cienfuegos', 'CU-06', 4),
    ('Granma', 'CU-12', 5),
    ('Guantánamo', 'CU-14', 6),
    ('Havana', 'CU-03', 7),
    ('Holguín', 'CU-11', 8),
    ('Isla de la Juventud', 'CU-99', 9),
    ('Las Tunas', 'CU-10', 10),
    ('Matanzas', 'CU-04', 11),
    ('Mayabeque', 'CU-16', 12),
    ('Pinar del Río', 'CU-01', 13),
    ('Sancti Spíritus', 'CU-07', 14),
    ('Santiago de Cuba', 'CU-13', 15),
    ('Villa Clara', 'CU-05', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'CU' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 圭亚那（GY）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Barima-Waini', 'GY-BA', 1),
    ('Cuyuni-Mazaruni', 'GY-CU', 2),
    ('Demerara-Mahaica', 'GY-DE', 3),
    ('East Berbice-Corentyne', 'GY-EB', 4),
    ('Essequibo Islands-West Demerara', 'GY-ES', 5),
    ('Mahaica-Berbice', 'GY-MA', 6),
    ('Pomeroon-Supenaam', 'GY-PM', 7),
    ('Potaro-Siparuni', 'GY-PT', 8),
    ('Upper Demerara-Berbice', 'GY-UD', 9),
    ('Upper Takutu-Upper Essequibo', 'GY-UT', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'GY' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 海地（HT）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Artibonite', 'HT-AR', 1),
    ('Centre', 'HT-CE', 2),
    ('Grand''Anse', 'HT-GA', 3),
    ('Nippes', 'HT-NI', 4),
    ('Nord', 'HT-ND', 5),
    ('Nord-Est', 'HT-NE', 6),
    ('Nord-Ouest', 'HT-NO', 7),
    ('Ouest', 'HT-OU', 8),
    ('Sud', 'HT-SD', 9),
    ('Sud-Est', 'HT-SE', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'HT' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 韩国（KR）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Busan', 'KR-26', 1),
    ('Daegu', 'KR-27', 2),
    ('Daejeon', 'KR-30', 3),
    ('Gangwon', 'KR-42', 4),
    ('Gyeonggi', 'KR-41', 5),
    ('Incheon', 'KR-28', 6),
    ('Jeju', 'KR-49', 7),
    ('Jeonnam-Gwangju Integrated Special City', 'KR-12', 8),
    ('North Chungcheong', 'KR-43', 9),
    ('North Gyeongsang', 'KR-47', 10),
    ('North Jeolla', 'KR-45', 11),
    ('Sejong City', 'KR-50', 12),
    ('Seoul', 'KR-11', 13),
    ('South Chungcheong', 'KR-44', 14),
    ('South Gyeongsang', 'KR-48', 15),
    ('Ulsan', 'KR-31', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'KR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 哈萨克斯坦（KZ）：20 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Abai', 'KZ-10', 1),
    ('Akmola', 'KZ-11', 2),
    ('Aktobe', 'KZ-15', 3),
    ('Almaty', 'KZ-19', 4),
    ('Almaty', 'KZ-75', 5),
    ('Astana', 'KZ-71', 6),
    ('Atyrau', 'KZ-23', 7),
    ('East Kazakhstan', 'KZ-63', 8),
    ('Jambyl', 'KZ-31', 9),
    ('Jetisu', 'KZ-33', 10),
    ('Karaganda', 'KZ-35', 11),
    ('Kostanay', 'KZ-39', 12),
    ('Kyzylorda', 'KZ-43', 13),
    ('Mangystau', 'KZ-47', 14),
    ('North Kazakhstan', 'KZ-59', 15),
    ('Pavlodar', 'KZ-55', 16),
    ('Shymkent', 'KZ-79', 17),
    ('Turkistan', 'KZ-61', 18),
    ('Ulytau', 'KZ-62', 19),
    ('West Kazakhstan', 'KZ-27', 20)
) AS v(name, code, ord)
WHERE p.standard_code = 'KZ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 黑山（ME）：25 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Andrijevica', 'ME-01', 1),
    ('Bar', 'ME-02', 2),
    ('Berane', 'ME-03', 3),
    ('Bijelo Polje', 'ME-04', 4),
    ('Budva', 'ME-05', 5),
    ('Danilovgrad', 'ME-07', 6),
    ('Gusinje', 'ME-22', 7),
    ('Herceg-Novi', 'ME-08', 8),
    ('Kolašin', 'ME-09', 9),
    ('Kotor', 'ME-10', 10),
    ('Mojkovac', 'ME-11', 11),
    ('Nikšić', 'ME-12', 12),
    ('Old Royal Capital Cetinje', 'ME-06', 13),
    ('Petnjica', 'ME-23', 14),
    ('Plav', 'ME-13', 15),
    ('Pljevlja', 'ME-14', 16),
    ('Plužine', 'ME-15', 17),
    ('Podgorica', 'ME-16', 18),
    ('Rožaje', 'ME-17', 19),
    ('Šavnik', 'ME-18', 20),
    ('Tivat', 'ME-19', 21),
    ('Tuzi', 'ME-24', 22),
    ('Ulcinj', 'ME-20', 23),
    ('Žabljak', 'ME-21', 24),
    ('Zeta', 'ME-25', 25)
) AS v(name, code, ord)
WHERE p.standard_code = 'ME' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 荷兰（NL）：12 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Drenthe', 'NL-DR', 1),
    ('Flevoland', 'NL-FL', 2),
    ('Friesland', 'NL-FR', 3),
    ('Gelderland', 'NL-GE', 4),
    ('Groningen', 'NL-GR', 5),
    ('Limburg', 'NL-LI', 6),
    ('North Brabant', 'NL-NB', 7),
    ('North Holland', 'NL-NH', 8),
    ('Overijssel', 'NL-OV', 9),
    ('South Holland', 'NL-ZH', 10),
    ('Utrecht', 'NL-UT', 11),
    ('Zeeland', 'NL-ZE', 12)
) AS v(name, code, ord)
WHERE p.standard_code = 'NL' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 荷属加勒比区（BQ）：3 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bonaire', 'BQ-BO', 1),
    ('Saba', 'BQ-SA', 2),
    ('Sint Eustatius', 'BQ-SE', 3)
) AS v(name, code, ord)
WHERE p.standard_code = 'BQ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 洪都拉斯（HN）：18 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Atlántida', 'HN-AT', 1),
    ('Bay Islands', 'HN-IB', 2),
    ('Choluteca', 'HN-CH', 3),
    ('Colón', 'HN-CL', 4),
    ('Comayagua', 'HN-CM', 5),
    ('Copán', 'HN-CP', 6),
    ('Cortés', 'HN-CR', 7),
    ('El Paraíso', 'HN-EP', 8),
    ('Francisco Morazán', 'HN-FM', 9),
    ('Gracias a Dios', 'HN-GD', 10),
    ('Intibucá', 'HN-IN', 11),
    ('La Paz', 'HN-LP', 12),
    ('Lempira', 'HN-LE', 13),
    ('Ocotepeque', 'HN-OC', 14),
    ('Olancho', 'HN-OL', 15),
    ('Santa Bárbara', 'HN-SB', 16),
    ('Valle', 'HN-VA', 17),
    ('Yoro', 'HN-YO', 18)
) AS v(name, code, ord)
WHERE p.standard_code = 'HN' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 加纳（GH）：16 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ahafo', 'GH-AF', 1),
    ('Ashanti', 'GH-AH', 2),
    ('Bono', 'GH-BO', 3),
    ('Bono East', 'GH-BE', 4),
    ('Central', 'GH-CP', 5),
    ('Eastern', 'GH-EP', 6),
    ('Greater Accra', 'GH-AA', 7),
    ('North East', 'GH-NE', 8),
    ('Northern', 'GH-NP', 9),
    ('Oti', 'GH-OT', 10),
    ('Savannah', 'GH-SV', 11),
    ('Upper East', 'GH-UE', 12),
    ('Upper West', 'GH-UW', 13),
    ('Volta', 'GH-TV', 14),
    ('Western', 'GH-WP', 15),
    ('Western North', 'GH-WN', 16)
) AS v(name, code, ord)
WHERE p.standard_code = 'GH' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 柬埔寨（KH）：25 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Banteay Meanchey', 'KH-1', 1),
    ('Battambang', 'KH-2', 2),
    ('Kampong Cham', 'KH-3', 3),
    ('Kampong Chhnang', 'KH-4', 4),
    ('Kampong Speu', 'KH-5', 5),
    ('Kampong Thom', 'KH-6', 6),
    ('Kampot', 'KH-7', 7),
    ('Kandal', 'KH-8', 8),
    ('Kep', 'KH-23', 9),
    ('Koh Kong', 'KH-9', 10),
    ('Kratie', 'KH-10', 11),
    ('Mondulkiri', 'KH-11', 12),
    ('Oddar Meanchey', 'KH-22', 13),
    ('Pailin', 'KH-24', 14),
    ('Phnom Penh', 'KH-12', 15),
    ('Preah Vihear', 'KH-13', 16),
    ('Prey Veng', 'KH-14', 17),
    ('Pursat', 'KH-15', 18),
    ('Ratanakiri', 'KH-16', 19),
    ('Siem Reap', 'KH-17', 20),
    ('Sihanoukville', 'KH-18', 21),
    ('Stung Treng', 'KH-19', 22),
    ('Svay Rieng', 'KH-20', 23),
    ('Takeo', 'KH-21', 24),
    ('Tboung Khmum', 'KH-25', 25)
) AS v(name, code, ord)
WHERE p.standard_code = 'KH' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 加蓬（GA）：9 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Estuaire', 'GA-1', 1),
    ('Haut-Ogooué', 'GA-2', 2),
    ('Moyen-Ogooué', 'GA-3', 3),
    ('Ngounié', 'GA-4', 4),
    ('Nyanga', 'GA-5', 5),
    ('Ogooué-Ivindo', 'GA-6', 6),
    ('Ogooué-Lolo', 'GA-7', 7),
    ('Ogooué-Maritime', 'GA-8', 8),
    ('Woleu-Ntem', 'GA-9', 9)
) AS v(name, code, ord)
WHERE p.standard_code = 'GA' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 吉布提（DJ）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ali Sabieh', 'DJ-AS', 1),
    ('Arta', 'DJ-AR', 2),
    ('Dikhil', 'DJ-DI', 3),
    ('Djibouti', 'DJ-DJ', 4),
    ('Obock', 'DJ-OB', 5),
    ('Tadjourah', 'DJ-TA', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'DJ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 捷克（CZ）：90 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Benešov', 'CZ-201', 1),
    ('Beroun', 'CZ-202', 2),
    ('Blansko', 'CZ-641', 3),
    ('Břeclav', 'CZ-644', 4),
    ('Brno-město', 'CZ-642', 5),
    ('Brno-venkov', 'CZ-643', 6),
    ('Bruntál', 'CZ-801', 7),
    ('Česká Lípa', 'CZ-511', 8),
    ('České Budějovice', 'CZ-311', 9),
    ('Český Krumlov', 'CZ-312', 10),
    ('Cheb', 'CZ-411', 11),
    ('Chomutov', 'CZ-422', 12),
    ('Chrudim', 'CZ-531', 13),
    ('Děčín', 'CZ-421', 14),
    ('Domažlice', 'CZ-321', 15),
    ('Frýdek-Místek', 'CZ-802', 16),
    ('Havlíčkův Brod', 'CZ-631', 17),
    ('Hodonín', 'CZ-645', 18),
    ('Hradec Králové', 'CZ-521', 19),
    ('Jablonec nad Nisou', 'CZ-512', 20),
    ('Jeseník', 'CZ-711', 21),
    ('Jičín', 'CZ-522', 22),
    ('Jihlava', 'CZ-632', 23),
    ('Jihočeský kraj', 'CZ-31', 24),
    ('Jihomoravský kraj', 'CZ-64', 25),
    ('Jindřichův Hradec', 'CZ-313', 26),
    ('Karlovarský kraj', 'CZ-41', 27),
    ('Karlovy Vary', 'CZ-412', 28),
    ('Karviná', 'CZ-803', 29),
    ('Kladno', 'CZ-203', 30),
    ('Klatovy', 'CZ-322', 31),
    ('Kolín', 'CZ-204', 32),
    ('Kraj Vysočina', 'CZ-63', 33),
    ('Královéhradecký kraj', 'CZ-52', 34),
    ('Kroměříž', 'CZ-721', 35),
    ('Kutná Hora', 'CZ-205', 36),
    ('Liberec', 'CZ-513', 37),
    ('Liberecký kraj', 'CZ-51', 38),
    ('Litoměřice', 'CZ-423', 39),
    ('Louny', 'CZ-424', 40),
    ('Mělník', 'CZ-206', 41),
    ('Mladá Boleslav', 'CZ-207', 42),
    ('Moravskoslezský kraj', 'CZ-80', 43),
    ('Most', 'CZ-425', 44),
    ('Náchod', 'CZ-523', 45),
    ('Nový Jičín', 'CZ-804', 46),
    ('Nymburk', 'CZ-208', 47),
    ('Olomouc', 'CZ-712', 48),
    ('Olomoucký kraj', 'CZ-71', 49),
    ('Opava', 'CZ-805', 50),
    ('Ostrava-město', 'CZ-806', 51),
    ('Pardubice', 'CZ-532', 52),
    ('Pardubický kraj', 'CZ-53', 53),
    ('Pelhřimov', 'CZ-633', 54),
    ('Písek', 'CZ-314', 55),
    ('Plzeň-jih', 'CZ-324', 56),
    ('Plzeň-město', 'CZ-323', 57),
    ('Plzeň-sever', 'CZ-325', 58),
    ('Plzeňský kraj', 'CZ-32', 59),
    ('Prachatice', 'CZ-315', 60),
    ('Praha-východ', 'CZ-209', 61),
    ('Praha-západ', 'CZ-20A', 62),
    ('Praha, Hlavní město', 'CZ-10', 63),
    ('Přerov', 'CZ-714', 64),
    ('Příbram', 'CZ-20B', 65),
    ('Prostějov', 'CZ-713', 66),
    ('Rakovník', 'CZ-20C', 67),
    ('Rokycany', 'CZ-326', 68),
    ('Rychnov nad Kněžnou', 'CZ-524', 69),
    ('Semily', 'CZ-514', 70),
    ('Sokolov', 'CZ-413', 71),
    ('Strakonice', 'CZ-316', 72),
    ('Středočeský kraj', 'CZ-20', 73),
    ('Šumperk', 'CZ-715', 74),
    ('Svitavy', 'CZ-533', 75),
    ('Tábor', 'CZ-317', 76),
    ('Tachov', 'CZ-327', 77),
    ('Teplice', 'CZ-426', 78),
    ('Třebíč', 'CZ-634', 79),
    ('Trutnov', 'CZ-525', 80),
    ('Uherské Hradiště', 'CZ-722', 81),
    ('Ústecký kraj', 'CZ-42', 82),
    ('Ústí nad Labem', 'CZ-427', 83),
    ('Ústí nad Orlicí', 'CZ-534', 84),
    ('Vsetín', 'CZ-723', 85),
    ('Vyškov', 'CZ-646', 86),
    ('Žďár nad Sázavou', 'CZ-635', 87),
    ('Zlín', 'CZ-724', 88),
    ('Zlínský kraj', 'CZ-72', 89),
    ('Znojmo', 'CZ-647', 90)
) AS v(name, code, ord)
WHERE p.standard_code = 'CZ' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 吉尔吉斯斯坦（KG）：9 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Batken', 'KG-B', 1),
    ('Bishkek', 'KG-GB', 2),
    ('Chuy', 'KG-C', 3),
    ('Issyk-Kul', 'KG-Y', 4),
    ('Jalal-Abad', 'KG-J', 5),
    ('Naryn', 'KG-N', 6),
    ('Osh', 'KG-GO', 7),
    ('Osh', 'KG-O', 8),
    ('Talas', 'KG-T', 9)
) AS v(name, code, ord)
WHERE p.standard_code = 'KG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 基里巴斯（KI）：3 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Gilbert', 'KI-G', 1),
    ('Line', 'KI-L', 2),
    ('Phoenix', 'KI-P', 3)
) AS v(name, code, ord)
WHERE p.standard_code = 'KI' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 津巴布韦（ZW）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bulawayo', 'ZW-BU', 1),
    ('Harare', 'ZW-HA', 2),
    ('Manicaland', 'ZW-MA', 3),
    ('Mashonaland Central', 'ZW-MC', 4),
    ('Mashonaland East', 'ZW-ME', 5),
    ('Mashonaland West', 'ZW-MW', 6),
    ('Masvingo', 'ZW-MV', 7),
    ('Matabeleland North', 'ZW-MN', 8),
    ('Matabeleland South', 'ZW-MS', 9),
    ('Midlands', 'ZW-MI', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'ZW' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 几内亚（GN）：41 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Beyla', 'GN-BE', 1),
    ('Boffa', 'GN-BF', 2),
    ('Boké', 'GN-B', 3),
    ('Boké', 'GN-BK', 4),
    ('Conakry', 'GN-C', 5),
    ('Coyah', 'GN-CO', 6),
    ('Dabola', 'GN-DB', 7),
    ('Dalaba', 'GN-DL', 8),
    ('Dinguiraye', 'GN-DI', 9),
    ('Dubréka', 'GN-DU', 10),
    ('Faranah', 'GN-F', 11),
    ('Faranah', 'GN-FA', 12),
    ('Forécariah', 'GN-FO', 13),
    ('Fria', 'GN-FR', 14),
    ('Gaoual', 'GN-GA', 15),
    ('Guéckédou', 'GN-GU', 16),
    ('Kankan', 'GN-KA', 17),
    ('Kankan', 'GN-K', 18),
    ('Kérouané', 'GN-KE', 19),
    ('Kindia', 'GN-D', 20),
    ('Kindia', 'GN-KD', 21),
    ('Kissidougou', 'GN-KS', 22),
    ('Koubia', 'GN-KB', 23),
    ('Koundara', 'GN-KN', 24),
    ('Kouroussa', 'GN-KO', 25),
    ('Labé', 'GN-LA', 26),
    ('Labé', 'GN-L', 27),
    ('Lélouma', 'GN-LE', 28),
    ('Lola', 'GN-LO', 29),
    ('Macenta', 'GN-MC', 30),
    ('Mali', 'GN-ML', 31),
    ('Mamou', 'GN-M', 32),
    ('Mamou', 'GN-MM', 33),
    ('Mandiana', 'GN-MD', 34),
    ('Nzérékoré', 'GN-N', 35),
    ('Nzérékoré', 'GN-NZ', 36),
    ('Pita', 'GN-PI', 37),
    ('Siguiri', 'GN-SI', 38),
    ('Télimélé', 'GN-TE', 39),
    ('Tougué', 'GN-TO', 40),
    ('Yomou', 'GN-YO', 41)
) AS v(name, code, ord)
WHERE p.standard_code = 'GN' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 几内亚比绍（GW）：12 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bafatá', 'GW-BA', 1),
    ('Biombo', 'GW-BM', 2),
    ('Bissau', 'GW-BS', 3),
    ('Bolama', 'GW-BL', 4),
    ('Cacheu', 'GW-CA', 5),
    ('Gabú', 'GW-GA', 6),
    ('Leste', 'GW-L', 7),
    ('Norte', 'GW-N', 8),
    ('Oio', 'GW-OI', 9),
    ('Quinara', 'GW-QU', 10),
    ('Sul', 'GW-S', 11),
    ('Tombali', 'GW-TO', 12)
) AS v(name, code, ord)
WHERE p.standard_code = 'GW' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 开曼群岛（KY）：3 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Cayman Brac', 'KY-02', 1),
    ('Grand Cayman', 'KY-01', 2),
    ('Little Cayman', 'KY-03', 3)
) AS v(name, code, ord)
WHERE p.standard_code = 'KY' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 喀麦隆（CM）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Adamawa', 'CM-AD', 1),
    ('Centre', 'CM-CE', 2),
    ('East', 'CM-ES', 3),
    ('Far North', 'CM-EN', 4),
    ('Littoral', 'CM-LT', 5),
    ('North', 'CM-NO', 6),
    ('Northwest', 'CM-NW', 7),
    ('South', 'CM-SU', 8),
    ('Southwest', 'CM-SW', 9),
    ('West', 'CM-OU', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'CM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 卡塔尔（QA）：8 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Al Daayen', 'QA-ZA', 1),
    ('Al Khor', 'QA-KH', 2),
    ('Al Rayyan', 'QA-RA', 3),
    ('Al Wakrah', 'QA-WA', 4),
    ('Al-Shahaniya', 'QA-SH', 5),
    ('Doha', 'QA-DA', 6),
    ('Madinat ash Shamal', 'QA-MS', 7),
    ('Umm Salal', 'QA-US', 8)
) AS v(name, code, ord)
WHERE p.standard_code = 'QA' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 克罗地亚（HR）：21 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bjelovar-Bilogora', 'HR-07', 1),
    ('Brod-Posavina', 'HR-12', 2),
    ('City of Zagreb', 'HR-21', 3),
    ('Dubrovnik-Neretva', 'HR-19', 4),
    ('Istria', 'HR-18', 5),
    ('Karlovac', 'HR-04', 6),
    ('Koprivnica-Križevci', 'HR-06', 7),
    ('Krapina-Zagorje', 'HR-02', 8),
    ('Lika-Senj', 'HR-09', 9),
    ('Međimurje', 'HR-20', 10),
    ('Osijek-Baranja', 'HR-14', 11),
    ('Požega-Slavonia', 'HR-11', 12),
    ('Primorje-Gorski Kotar', 'HR-08', 13),
    ('Šibenik-Knin', 'HR-15', 14),
    ('Sisak-Moslavina', 'HR-03', 15),
    ('Split-Dalmatia', 'HR-17', 16),
    ('Varaždin', 'HR-05', 17),
    ('Virovitica-Podravina', 'HR-10', 18),
    ('Vukovar-Syrmia', 'HR-16', 19),
    ('Zadar', 'HR-13', 20),
    ('Zagreb', 'HR-01', 21)
) AS v(name, code, ord)
WHERE p.standard_code = 'HR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 科摩罗（KM）：3 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Anjouan', 'KM-A', 1),
    ('Grande Comore', 'KM-G', 2),
    ('Mohéli', 'KM-M', 3)
) AS v(name, code, ord)
WHERE p.standard_code = 'KM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 肯尼亚（KE）：47 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Baringo', 'KE-01', 1),
    ('Bomet', 'KE-02', 2),
    ('Bungoma', 'KE-03', 3),
    ('Busia', 'KE-04', 4),
    ('Elgeyo-Marakwet', 'KE-05', 5),
    ('Embu', 'KE-06', 6),
    ('Garissa', 'KE-07', 7),
    ('Homa Bay', 'KE-08', 8),
    ('Isiolo', 'KE-09', 9),
    ('Kajiado', 'KE-10', 10),
    ('Kakamega', 'KE-11', 11),
    ('Kericho', 'KE-12', 12),
    ('Kiambu', 'KE-13', 13),
    ('Kilifi', 'KE-14', 14),
    ('Kirinyaga', 'KE-15', 15),
    ('Kisii', 'KE-16', 16),
    ('Kisumu', 'KE-17', 17),
    ('Kitui', 'KE-18', 18),
    ('Kwale', 'KE-19', 19),
    ('Laikipia', 'KE-20', 20),
    ('Lamu', 'KE-21', 21),
    ('Machakos', 'KE-22', 22),
    ('Makueni', 'KE-23', 23),
    ('Mandera', 'KE-24', 24),
    ('Marsabit', 'KE-25', 25),
    ('Meru', 'KE-26', 26),
    ('Migori', 'KE-27', 27),
    ('Mombasa', 'KE-28', 28),
    ('Murang''a', 'KE-29', 29),
    ('Nairobi City', 'KE-30', 30),
    ('Nakuru', 'KE-31', 31),
    ('Nandi', 'KE-32', 32),
    ('Narok', 'KE-33', 33),
    ('Nyamira', 'KE-34', 34),
    ('Nyandarua', 'KE-35', 35),
    ('Nyeri', 'KE-36', 36),
    ('Samburu', 'KE-37', 37),
    ('Siaya', 'KE-38', 38),
    ('Taita–Taveta', 'KE-39', 39),
    ('Tana River', 'KE-40', 40),
    ('Tharaka-Nithi', 'KE-41', 41),
    ('Trans Nzoia', 'KE-42', 42),
    ('Turkana', 'KE-43', 43),
    ('Uasin Gishu', 'KE-44', 44),
    ('Vihiga', 'KE-45', 45),
    ('Wajir', 'KE-46', 46),
    ('West Pokot', 'KE-47', 47)
) AS v(name, code, ord)
WHERE p.standard_code = 'KE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 科特迪瓦（CI）：14 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Abidjan', 'CI-AB', 1),
    ('Bas-Sassandra', 'CI-BS', 2),
    ('Comoé', 'CI-CM', 3),
    ('Denguélé', 'CI-DN', 4),
    ('Gôh-Djiboua', 'CI-GD', 5),
    ('Lacs', 'CI-LC', 6),
    ('Lagunes', 'CI-LG', 7),
    ('Montagnes', 'CI-MG', 8),
    ('Sassandra-Marahoué', 'CI-SM', 9),
    ('Savanes', 'CI-SV', 10),
    ('Vallée du Bandama', 'CI-VB', 11),
    ('Woroba', 'CI-WR', 12),
    ('Yamoussoukro', 'CI-YM', 13),
    ('Zanzan', 'CI-ZZ', 14)
) AS v(name, code, ord)
WHERE p.standard_code = 'CI' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 科威特（KW）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Al Ahmadi', 'KW-AH', 1),
    ('Al Asimah', 'KW-KU', 2),
    ('Al Farwaniyah', 'KW-FA', 3),
    ('Al Jahra', 'KW-JA', 4),
    ('Hawalli', 'KW-HA', 5),
    ('Mubarak Al-Kabeer', 'KW-MU', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'KW' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 莱索托（LS）：10 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Berea', 'LS-D', 1),
    ('Butha-Buthe', 'LS-B', 2),
    ('Leribe', 'LS-C', 3),
    ('Mafeteng', 'LS-E', 4),
    ('Maseru', 'LS-A', 5),
    ('Mohale''s Hoek', 'LS-F', 6),
    ('Mokhotlong', 'LS-J', 7),
    ('Qacha''s Nek', 'LS-H', 8),
    ('Quthing', 'LS-G', 9),
    ('Thaba-Tseka', 'LS-K', 10)
) AS v(name, code, ord)
WHERE p.standard_code = 'LS' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 老挝（LA）：18 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Attapeu', 'LA-AT', 1),
    ('Bokeo', 'LA-BK', 2),
    ('Bolikhamsai', 'LA-BL', 3),
    ('Champasak', 'LA-CH', 4),
    ('Houaphanh', 'LA-HO', 5),
    ('Khammouane', 'LA-KH', 6),
    ('Luang Namtha', 'LA-LM', 7),
    ('Luang Prabang', 'LA-LP', 8),
    ('Oudomxay', 'LA-OU', 9),
    ('Phongsaly', 'LA-PH', 10),
    ('Sainyabuli', 'LA-XA', 11),
    ('Salavan', 'LA-SL', 12),
    ('Savannakhet', 'LA-SV', 13),
    ('Sekong', 'LA-XE', 14),
    ('Vientiane', 'LA-VI', 15),
    ('Vientiane', 'LA-VT', 16),
    ('Xaisomboun', 'LA-XS', 17),
    ('Xiangkhouang', 'LA-XI', 18)
) AS v(name, code, ord)
WHERE p.standard_code = 'LA' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 拉脱维亚（LV）：42 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ādaži', 'LV-011', 1),
    ('Aizkraukle', 'LV-002', 2),
    ('Alūksne', 'LV-007', 3),
    ('Augšdaugava', 'LV-111', 4),
    ('Balvi', 'LV-015', 5),
    ('Bauska', 'LV-016', 6),
    ('Cēsis', 'LV-022', 7),
    ('Daugavpils', 'LV-DGV', 8),
    ('Dienvidkurzemes', 'LV-112', 9),
    ('Dobele', 'LV-026', 10),
    ('Gulbene', 'LV-033', 11),
    ('Jēkabpils', 'LV-042', 12),
    ('Jelgava', 'LV-041', 13),
    ('Jelgava', 'LV-JEL', 14),
    ('Jūrmala', 'LV-JUR', 15),
    ('Ķekava', 'LV-052', 16),
    ('Krāslava', 'LV-047', 17),
    ('Kuldīga', 'LV-050', 18),
    ('Liepāja', 'LV-LPX', 19),
    ('Limbaži', 'LV-054', 20),
    ('Līvāni', 'LV-056', 21),
    ('Ludza', 'LV-058', 22),
    ('Madona', 'LV-059', 23),
    ('Mārupe', 'LV-062', 24),
    ('Ogre', 'LV-067', 25),
    ('Olaine', 'LV-068', 26),
    ('Preiļi', 'LV-073', 27),
    ('Rēzekne', 'LV-077', 28),
    ('Rēzekne', 'LV-REZ', 29),
    ('Riga', 'LV-RIX', 30),
    ('Ropaži', 'LV-080', 31),
    ('Salaspils', 'LV-087', 32),
    ('Saldus', 'LV-088', 33),
    ('Saulkrasti', 'LV-089', 34),
    ('Sigulda', 'LV-091', 35),
    ('Smiltene', 'LV-094', 36),
    ('Talsi', 'LV-097', 37),
    ('Tukums', 'LV-099', 38),
    ('Valka', 'LV-101', 39),
    ('Valmiera', 'LV-113', 40),
    ('Ventspils', 'LV-VEN', 41),
    ('Ventspils', 'LV-106', 42)
) AS v(name, code, ord)
WHERE p.standard_code = 'LV' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 黎巴嫩（LB）：8 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Akkar', 'LB-AK', 1),
    ('Baalbek-Hermel', 'LB-BH', 2),
    ('Beirut', 'LB-BA', 3),
    ('Beqaa', 'LB-BI', 4),
    ('Mount Lebanon', 'LB-JL', 5),
    ('Nabatieh', 'LB-NA', 6),
    ('North', 'LB-AS', 7),
    ('South', 'LB-JA', 8)
) AS v(name, code, ord)
WHERE p.standard_code = 'LB' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 利比里亚（LR）：15 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Bomi', 'LR-BM', 1),
    ('Bong', 'LR-BG', 2),
    ('Gbarpolu', 'LR-GP', 3),
    ('Grand Bassa', 'LR-GB', 4),
    ('Grand Cape Mount', 'LR-CM', 5),
    ('Grand Gedeh', 'LR-GG', 6),
    ('Grand Kru', 'LR-GK', 7),
    ('Lofa', 'LR-LO', 8),
    ('Margibi', 'LR-MG', 9),
    ('Maryland', 'LR-MY', 10),
    ('Montserrado', 'LR-MO', 11),
    ('Nimba', 'LR-NI', 12),
    ('River Cess', 'LR-RI', 13),
    ('River Gee', 'LR-RG', 14),
    ('Sinoe', 'LR-SI', 15)
) AS v(name, code, ord)
WHERE p.standard_code = 'LR' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 利比亚（LY）：22 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Al Butnan', 'LY-BU', 1),
    ('Al Wahat', 'LY-WA', 2),
    ('Benghazi', 'LY-BA', 3),
    ('Derna', 'LY-DR', 4),
    ('Ghat', 'LY-GT', 5),
    ('Jabal al Akhdar', 'LY-JA', 6),
    ('Jabal al Gharbi', 'LY-JG', 7),
    ('Jafara', 'LY-JI', 8),
    ('Jufra', 'LY-JU', 9),
    ('Kufra', 'LY-KF', 10),
    ('Marj', 'LY-MJ', 11),
    ('Misrata', 'LY-MI', 12),
    ('Murqub', 'LY-MB', 13),
    ('Murzuq', 'LY-MQ', 14),
    ('Nalut', 'LY-NL', 15),
    ('Nuqat al Khams', 'LY-NQ', 16),
    ('Sabha', 'LY-SB', 17),
    ('Sirte', 'LY-SR', 18),
    ('Tripoli', 'LY-TB', 19),
    ('Wadi al Hayaa', 'LY-WD', 20),
    ('Wadi al Shatii', 'LY-WS', 21),
    ('Zawiya', 'LY-ZA', 22)
) AS v(name, code, ord)
WHERE p.standard_code = 'LY' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 列支敦士登（LI）：11 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Balzers', 'LI-01', 1),
    ('Eschen', 'LI-02', 2),
    ('Gamprin', 'LI-03', 3),
    ('Mauren', 'LI-04', 4),
    ('Planken', 'LI-05', 5),
    ('Ruggell', 'LI-06', 6),
    ('Schaan', 'LI-07', 7),
    ('Schellenberg', 'LI-08', 8),
    ('Triesen', 'LI-09', 9),
    ('Triesenberg', 'LI-10', 10),
    ('Vaduz', 'LI-11', 11)
) AS v(name, code, ord)
WHERE p.standard_code = 'LI' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 立陶宛（LT）：70 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Akmenė', 'LT-01', 1),
    ('Alytus', 'LT-03', 2),
    ('Alytus', 'LT-02', 3),
    ('Alytus', 'LT-AL', 4),
    ('Anykščiai', 'LT-04', 5),
    ('Birštonas', 'LT-05', 6),
    ('Biržai', 'LT-06', 7),
    ('Druskininkai', 'LT-07', 8),
    ('Elektrėnai', 'LT-08', 9),
    ('Ignalina', 'LT-09', 10),
    ('Jonava', 'LT-10', 11),
    ('Joniškis', 'LT-11', 12),
    ('Jurbarkas', 'LT-12', 13),
    ('Kaišiadorys', 'LT-13', 14),
    ('Kalvarija', 'LT-14', 15),
    ('Kaunas', 'LT-KU', 16),
    ('Kaunas', 'LT-16', 17),
    ('Kaunas', 'LT-15', 18),
    ('Kazlų Rūda', 'LT-17', 19),
    ('Kėdainiai', 'LT-18', 20),
    ('Kelmė', 'LT-19', 21),
    ('Klaipėda', 'LT-KL', 22),
    ('Klaipėda', 'LT-21', 23),
    ('Klaipėdos miestas', 'LT-20', 24),
    ('Kretinga', 'LT-22', 25),
    ('Kupiškis', 'LT-23', 26),
    ('Lazdijai', 'LT-24', 27),
    ('Marijampolė', 'LT-25', 28),
    ('Marijampolė', 'LT-MR', 29),
    ('Mažeikiai', 'LT-26', 30),
    ('Molėtai', 'LT-27', 31),
    ('Neringa', 'LT-28', 32),
    ('Pagėgiai', 'LT-29', 33),
    ('Pakruojis', 'LT-30', 34),
    ('Palanga', 'LT-31', 35),
    ('Panevėžio miestas', 'LT-32', 36),
    ('Panevėžys', 'LT-33', 37),
    ('Panevėžys', 'LT-PN', 38),
    ('Pasvalys', 'LT-34', 39),
    ('Plungė', 'LT-35', 40),
    ('Prienai', 'LT-36', 41),
    ('Radviliškis', 'LT-37', 42),
    ('Raseiniai', 'LT-38', 43),
    ('Rietavas', 'LT-39', 44),
    ('Rokiškis', 'LT-40', 45),
    ('Šakiai', 'LT-41', 46),
    ('Šalčininkai', 'LT-42', 47),
    ('Šiauliai', 'LT-SA', 48),
    ('Šiauliai', 'LT-43', 49),
    ('Šiauliai', 'LT-44', 50),
    ('Šilalė', 'LT-45', 51),
    ('Šilutė', 'LT-46', 52),
    ('Širvintos', 'LT-47', 53),
    ('Skuodas', 'LT-48', 54),
    ('Švenčionys', 'LT-49', 55),
    ('Tauragė', 'LT-TA', 56),
    ('Tauragė', 'LT-50', 57),
    ('Telšiai', 'LT-51', 58),
    ('Telšiai', 'LT-TE', 59),
    ('Trakai', 'LT-52', 60),
    ('Ukmergė', 'LT-53', 61),
    ('Utena', 'LT-54', 62),
    ('Utena', 'LT-UT', 63),
    ('Varėna', 'LT-55', 64),
    ('Vilkaviškis', 'LT-56', 65),
    ('Vilnius', 'LT-58', 66),
    ('Vilnius', 'LT-57', 67),
    ('Vilnius', 'LT-VL', 68),
    ('Visaginas', 'LT-59', 69),
    ('Zarasai', 'LT-60', 70)
) AS v(name, code, ord)
WHERE p.standard_code = 'LT' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 留尼汪（RE）：4 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Saint-Benoît', 'RE-01', 1),
    ('Saint-Denis', 'RE-02', 2),
    ('Saint-Paul', 'RE-03', 3),
    ('Saint-Pierre', 'RE-04', 4)
) AS v(name, code, ord)
WHERE p.standard_code = 'RE' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 罗马尼亚（RO）：42 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Alba', 'RO-AB', 1),
    ('Arad', 'RO-AR', 2),
    ('Argeș', 'RO-AG', 3),
    ('Bacău', 'RO-BC', 4),
    ('Bihor', 'RO-BH', 5),
    ('Bistrița-Năsăud', 'RO-BN', 6),
    ('Botoșani', 'RO-BT', 7),
    ('Brăila', 'RO-BR', 8),
    ('Brașov', 'RO-BV', 9),
    ('Bucharest', 'RO-B', 10),
    ('Buzău', 'RO-BZ', 11),
    ('Călărași', 'RO-CL', 12),
    ('Caraș-Severin', 'RO-CS', 13),
    ('Cluj', 'RO-CJ', 14),
    ('Constanța', 'RO-CT', 15),
    ('Covasna', 'RO-CV', 16),
    ('Dâmbovița', 'RO-DB', 17),
    ('Dolj', 'RO-DJ', 18),
    ('Galați', 'RO-GL', 19),
    ('Giurgiu', 'RO-GR', 20),
    ('Gorj', 'RO-GJ', 21),
    ('Harghita', 'RO-HR', 22),
    ('Hunedoara', 'RO-HD', 23),
    ('Ialomița', 'RO-IL', 24),
    ('Iași', 'RO-IS', 25),
    ('Ilfov', 'RO-IF', 26),
    ('Maramureș', 'RO-MM', 27),
    ('Mehedinți', 'RO-MH', 28),
    ('Mureș', 'RO-MS', 29),
    ('Neamț', 'RO-NT', 30),
    ('Olt', 'RO-OT', 31),
    ('Prahova', 'RO-PH', 32),
    ('Sălaj', 'RO-SJ', 33),
    ('Satu Mare', 'RO-SM', 34),
    ('Sibiu', 'RO-SB', 35),
    ('Suceava', 'RO-SV', 36),
    ('Teleorman', 'RO-TR', 37),
    ('Timiș', 'RO-TM', 38),
    ('Tulcea', 'RO-TL', 39),
    ('Vâlcea', 'RO-VL', 40),
    ('Vaslui', 'RO-VS', 41),
    ('Vrancea', 'RO-VN', 42)
) AS v(name, code, ord)
WHERE p.standard_code = 'RO' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 卢森堡（LU）：12 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Capellen', 'LU-CA', 1),
    ('Clervaux', 'LU-CL', 2),
    ('Diekirch', 'LU-DI', 3),
    ('Echternach', 'LU-EC', 4),
    ('Esch-sur-Alzette', 'LU-ES', 5),
    ('Grevenmacher', 'LU-GR', 6),
    ('Luxembourg', 'LU-LU', 7),
    ('Mersch', 'LU-ME', 8),
    ('Redange', 'LU-RD', 9),
    ('Remich', 'LU-RM', 10),
    ('Vianden', 'LU-VD', 11),
    ('Wiltz', 'LU-WI', 12)
) AS v(name, code, ord)
WHERE p.standard_code = 'LU' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 卢旺达（RW）：5 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Eastern', 'RW-02', 1),
    ('Kigali', 'RW-01', 2),
    ('Northern', 'RW-03', 3),
    ('Southern', 'RW-05', 4),
    ('Western', 'RW-04', 5)
) AS v(name, code, ord)
WHERE p.standard_code = 'RW' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 马达加斯加（MG）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Antananarivo', 'MG-T', 1),
    ('Antsiranana', 'MG-D', 2),
    ('Fianarantsoa', 'MG-F', 3),
    ('Mahajanga', 'MG-M', 4),
    ('Toamasina', 'MG-A', 5),
    ('Toliara', 'MG-U', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'MG' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 马恩岛（IM）：6 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Ayre', 'IM-01', 1),
    ('Garff', 'IM-02', 2),
    ('Glenfaba', 'IM-03', 3),
    ('Michael', 'IM-04', 4),
    ('Middle', 'IM-05', 5),
    ('Rushen', 'IM-06', 6)
) AS v(name, code, ord)
WHERE p.standard_code = 'IM' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

-- 马尔代夫（MV）：21 个一级行政区
INSERT INTO region (level, region_name, parent_id, is_hot, sort_order, first_word, standard_code)
SELECT 2, v.name, p.region_id, 0, v.ord, left(v.name, 1), v.code
FROM region p,
(VALUES
    ('Addu', 'MV-01', 1),
    ('Alif Alif', 'MV-02', 2),
    ('Alif Dhaal', 'MV-00', 3),
    ('Baa', 'MV-20', 4),
    ('Dhaalu', 'MV-17', 5),
    ('Faafu', 'MV-14', 6),
    ('Gaafu Alif', 'MV-27', 7),
    ('Gaafu Dhaalu', 'MV-28', 8),
    ('Gnaviyani', 'MV-29', 9),
    ('Haa Alif', 'MV-07', 10),
    ('Haa Dhaalu', 'MV-23', 11),
    ('Kaafu', 'MV-26', 12),
    ('Laamu', 'MV-05', 13),
    ('Lhaviyani', 'MV-03', 14),
    ('Malé', 'MV-MLE', 15),
    ('Meemu', 'MV-12', 16),
    ('Noonu', 'MV-25', 17),
    ('Raa', 'MV-13', 18),
    ('Shaviyani', 'MV-24', 19),
    ('Thaa', 'MV-08', 20),
    ('Vaavu', 'MV-04', 21)
) AS v(name, code, ord)
WHERE p.standard_code = 'MV' AND p.parent_id = 0
  AND NOT EXISTS (
      SELECT 1 FROM region ch WHERE ch.parent_id = p.region_id AND ch.standard_code = v.code
  );

COMMIT;
