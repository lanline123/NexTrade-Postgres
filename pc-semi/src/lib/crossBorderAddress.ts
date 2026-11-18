/**
 * 跨境收货地址注册表（PC 端）。
 *
 * 与 H5 `uniapp-react/src/utils/crossBorderAddress.ts` 同一口径（字段键、
 * 邮编/手机号规则、`address_data` 组装键名），PC 与 H5 是独立应用无法
 * 共用模块，故在此镜像一份；改国家配置时两端同步修改。
 * 覆盖：CN/MY/SG/TH/KH/VN/HK/TW（一期 8 国）+ US/CA（v5.8.62 二期）。
 */

export type CrossBorderFieldKey =
  | 'province'
  | 'city'
  | 'district'
  | 'subdistrict'
  | 'village'
  | 'address'
  | 'addressLine1'
  | 'addressLineTwo'
  | 'buildingName'
  | 'floorUnit'
  | 'floor'
  | 'unit'
  | 'legacyDistrict'
  | 'postcode';

export interface CrossBorderField {
  key: CrossBorderFieldKey;
  label: string;
  placeholder: string;
  required?: boolean;
}

export interface CrossBorderCountry {
  code: string;
  name: string;
  dialCode: string;
  regionFields: CrossBorderField[];
  fields: CrossBorderField[];
  postcodePattern?: RegExp;
  postcodeMessage?: string;
  mobilePattern: RegExp;
  mobileHint: string;
}

export const CROSS_BORDER_COUNTRIES: CrossBorderCountry[] = [
  {
    code: 'CN',
    name: '中国大陆',
    dialCode: '86',
    regionFields: [
      { key: 'province', label: '省', placeholder: '请选择省', required: true },
      { key: 'city', label: '市', placeholder: '请选择市', required: true },
      { key: 'district', label: '区/县', placeholder: '请选择区/县', required: true },
    ],
    fields: [
      { key: 'address', label: '详细地址', placeholder: '门牌号、建筑物、街道', required: true },
      { key: 'postcode', label: '邮编', placeholder: '6位数字' },
    ],
    postcodePattern: /^\d{6}$/,
    postcodeMessage: '邮编必须为6位数字',
    mobilePattern: /^1[3-9]\d{9}$/,
    mobileHint: '11位且以1开头',
  },
  {
    code: 'MY',
    name: '马来西亚',
    dialCode: '60',
    regionFields: [
      { key: 'province', label: '州', placeholder: '请输入州', required: true },
      { key: 'city', label: '城市', placeholder: '请输入城市', required: true },
    ],
    fields: [
      { key: 'addressLine1', label: '详细地址', placeholder: '请输入详细地址', required: true },
      { key: 'addressLineTwo', label: '补充地址', placeholder: '选填' },
      { key: 'postcode', label: '邮政编码', placeholder: '5位数字', required: true },
    ],
    postcodePattern: /^\d{5}$/,
    postcodeMessage: '邮政编码必须为5位数字',
    mobilePattern: /^1\d{8,9}$/,
    mobileHint: '9-10位且以1开头',
  },
  {
    code: 'SG',
    name: '新加坡',
    dialCode: '65',
    regionFields: [{ key: 'district', label: '地区', placeholder: '请输入地区', required: true }],
    fields: [
      { key: 'addressLine1', label: '门牌号及街道名称', placeholder: '请输入门牌号及街道名称', required: true },
      { key: 'floorUnit', label: '楼层/单元号', placeholder: '选填' },
      { key: 'buildingName', label: '建筑名称', placeholder: '选填' },
      { key: 'postcode', label: '邮政编码', placeholder: '6位数字', required: true },
    ],
    postcodePattern: /^\d{6}$/,
    postcodeMessage: '邮政编码必须为6位数字',
    mobilePattern: /^[89]\d{7}$/,
    mobileHint: '8位且以8或9开头',
  },
  {
    code: 'TH',
    name: '泰国',
    dialCode: '66',
    regionFields: [
      { key: 'province', label: '府/直辖市', placeholder: '请输入府/直辖市', required: true },
      { key: 'district', label: '县/区', placeholder: '请输入县/区', required: true },
      { key: 'subdistrict', label: '乡/街道', placeholder: '请输入乡/街道', required: true },
    ],
    fields: [
      { key: 'address', label: '详细地址', placeholder: '请输入详细地址', required: true },
      { key: 'postcode', label: '邮编', placeholder: '5位数字', required: true },
    ],
    postcodePattern: /^\d{5}$/,
    postcodeMessage: '邮编必须为5位数字',
    mobilePattern: /^[689]\d{8}$/,
    mobileHint: '9位且以6、8或9开头',
  },
  {
    code: 'KH',
    name: '柬埔寨',
    dialCode: '855',
    regionFields: [
      { key: 'province', label: '省/直辖市', placeholder: '请输入省/直辖市', required: true },
      { key: 'district', label: '县/区', placeholder: '请输入县/区', required: true },
      { key: 'subdistrict', label: '乡/分区', placeholder: '请输入乡/分区' },
      { key: 'village', label: '村', placeholder: '请输入村' },
    ],
    fields: [
      { key: 'address', label: '详细地址', placeholder: '请输入详细地址', required: true },
      { key: 'postcode', label: '邮编', placeholder: '选填' },
    ],
    postcodePattern: /^[A-Za-z0-9 -]{1,20}$/,
    postcodeMessage: '邮编须为1至20位字母、数字、空格或短横线',
    mobilePattern: /^[1-9]\d{7,8}$/,
    mobileHint: '8-9位数字',
  },
  {
    code: 'VN',
    name: '越南',
    dialCode: '84',
    regionFields: [
      { key: 'province', label: '省/直辖市', placeholder: '请输入省/直辖市', required: true },
      { key: 'subdistrict', label: '坊/社/特区', placeholder: '请输入坊/社/特区', required: true },
    ],
    fields: [
      { key: 'legacyDistrict', label: '历史郡/县', placeholder: '请输入历史郡/县', required: true },
      { key: 'address', label: '详细地址', placeholder: '请输入详细地址', required: true },
      { key: 'postcode', label: '邮编', placeholder: '5位数字（选填）' },
    ],
    postcodePattern: /^\d{5}$/,
    postcodeMessage: '邮编必须为5位数字',
    mobilePattern: /^[35789]\d{8}$/,
    mobileHint: '9位且以3、5、7、8或9开头',
  },
  {
    code: 'HK',
    name: '中国香港',
    dialCode: '852',
    regionFields: [
      { key: 'province', label: '区域', placeholder: '香港岛、九龙或新界', required: true },
      { key: 'district', label: '地区', placeholder: '香港18区', required: true },
    ],
    fields: [
      { key: 'address', label: '街道/屋苑及门牌', placeholder: '请输入街道、屋苑及门牌', required: true },
      { key: 'buildingName', label: '楼宇/大厦/座', placeholder: '选填' },
      { key: 'floor', label: '楼层', placeholder: '选填' },
      { key: 'unit', label: '单位/室', placeholder: '选填' },
    ],
    mobilePattern: /^[4-9]\d{7}$/,
    mobileHint: '8位且以4-9开头',
  },
  {
    code: 'TW',
    name: '中国台湾',
    dialCode: '886',
    regionFields: [
      { key: 'province', label: '县市', placeholder: '请输入县市', required: true },
      { key: 'district', label: '乡镇市区', placeholder: '请输入乡镇市区', required: true },
    ],
    fields: [
      { key: 'address', label: '详细地址', placeholder: '请输入详细地址', required: true },
      { key: 'postcode', label: '邮政区号', placeholder: '3位、旧5位或3+3格式', required: true },
    ],
    postcodePattern: /^\d{3}(?:-?\d{2,3})?$/,
    postcodeMessage: '邮政区号须为3位、旧5位或3+3格式',
    mobilePattern: /^9\d{8}$/,
    mobileHint: '9位且以9开头',
  },
  {
    code: 'US',
    name: '美国',
    dialCode: '1',
    regionFields: [
      { key: 'province', label: '州', placeholder: '请选择州', required: true },
      { key: 'city', label: '城市', placeholder: '请输入城市', required: true },
    ],
    fields: [
      { key: 'addressLine1', label: '详细地址', placeholder: '街道名称及门牌号', required: true },
      { key: 'addressLineTwo', label: '公寓/套房', placeholder: '选填' },
      { key: 'postcode', label: '邮编 (ZIP)', placeholder: '5位数字或 ZIP+4', required: true },
    ],
    postcodePattern: /^\d{5}(-\d{4})?$/,
    postcodeMessage: '邮编须为5位数字或 ZIP+4 格式',
    mobilePattern: /^[2-9]\d{9}$/,
    mobileHint: '10位数字',
  },
  {
    code: 'CA',
    name: '加拿大',
    dialCode: '1',
    regionFields: [
      { key: 'province', label: '省/地区', placeholder: '请选择省/地区', required: true },
      { key: 'city', label: '城市', placeholder: '请输入城市', required: true },
    ],
    fields: [
      { key: 'addressLine1', label: '详细地址', placeholder: '街道名称及门牌号', required: true },
      { key: 'addressLineTwo', label: '公寓/套房', placeholder: '选填' },
      { key: 'postcode', label: '邮政编码', placeholder: '如 K1A 0B1', required: true },
    ],
    postcodePattern: /^[A-Za-z]\d[A-Za-z][ -]?\d[A-Za-z]\d$/,
    postcodeMessage: '邮政编码须为 A1A 1A1 格式',
    mobilePattern: /^[2-9]\d{9}$/,
    mobileHint: '10位数字',
  },
];

export function getCrossBorderCountry(code?: string): CrossBorderCountry {
  return (
    CROSS_BORDER_COUNTRIES.find((item) => item.code === String(code || '').toUpperCase()) ||
    CROSS_BORDER_COUNTRIES[0]
  );
}

export type CrossBorderValues = Record<CrossBorderFieldKey, string>;

export function emptyCrossBorderValues(): CrossBorderValues {
  return {
    province: '',
    city: '',
    district: '',
    subdistrict: '',
    village: '',
    address: '',
    addressLine1: '',
    addressLineTwo: '',
    buildingName: '',
    floorUnit: '',
    floor: '',
    unit: '',
    legacyDistrict: '',
    postcode: '',
  };
}

/** `address_data` 的键名映射（与 H5 一致：addressLine1 落 addressLineOne）。 */
export function addressDataKey(key: CrossBorderFieldKey): string {
  return key === 'addressLine1' ? 'addressLineOne' : key;
}

/** 组装后端 `address_data`（CN 只收 fields，其余国家 regionFields + fields，与 H5 同口径）。 */
export function buildAddressData(countryCode: string, values: CrossBorderValues): Record<string, string> {
  const country = getCrossBorderCountry(countryCode);
  const fields = country.code === 'CN' ? country.fields : [...country.regionFields, ...country.fields];
  const result: Record<string, string> = {};
  for (const field of fields) {
    const value = String(values[field.key] || '').trim();
    if (value) result[addressDataKey(field.key)] = value;
  }
  return result;
}

/** 主地址行（与 H5 formatCrossBorderAddress 同口径）。 */
export function formatMainAddress(countryCode: string, values: CrossBorderValues): string {
  const country = getCrossBorderCountry(countryCode);
  const keys = country.fields.map((field) => field.key);
  const mainKey: CrossBorderFieldKey =
    keys.includes('addressLine1') && !keys.includes('address') ? 'addressLine1' : 'address';
  return String(values[mainKey] || '').trim();
}

/** 完整展示地址：国家 + 地区名 + 主地址行 + 邮编（存 formatted_address）。 */
export function formatFullAddress(
  countryCode: string,
  regionNames: string[],
  values: CrossBorderValues,
): string {
  const country = getCrossBorderCountry(countryCode);
  const main = formatMainAddress(countryCode, values);
  const line2 = String(values.addressLineTwo || '').trim();
  const postcode = String(values.postcode || '').trim();
  return [country.name, ...regionNames, main, line2, postcode].filter(Boolean).join(' ');
}
