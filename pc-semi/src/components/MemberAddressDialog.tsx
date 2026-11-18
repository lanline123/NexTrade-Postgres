
/**
 * 收货地址编辑弹窗（地区级联）
 * 源码对照：java-b2b2c-overseas-v5.8.61/view/pc/pages/member/address/EditRegion.vue（200 行）
 *          + java-b2b2c-overseas-v5.8.61/view/pc/components/member/address/DialogEditRegion.vue
 *
 * 旧版 DialogEditRegion.vue 是「把 EditRegion.vue 表单包进 el-dialog」的懒加载壳：
 * - 点击 → 首次挂载 EditRegion 表单 → el-dialog 打开；
 * - 表单提交成功 → emit('okCallback', result, data)，壳层关闭弹窗。
 * 表单字段：收货人 / 所在地区（SelectRegion 三级级联）/ 详细地址 / 手机号码 / 固定电话 / 邮箱地址。
 *
 * 标记（本文件已按旧版模板逐条改名，不再用自造类名）：
 *   旧版 EditRegion.vue:2-41
 *     <div> > <div class="modal_region_box">
 *         a-form :label-col={span:3} :wrapper-col={span:16}     → .ant-form.ant-form-horizontal
 *           a-form-item                                          → .ant-form-item
 *             .ant-form-item-row > .ant-form-item-label(12.5%) + .ant-form-item-control(66.67%)
 *               .ant-form-item-control-input > .ant-form-item-control-input-content > .ant-input
 *           a-input style="width:300px"                          → width 300 内联
 *         <div class="address_from"> > <div class="from_item">
 *             .tit（空占位） + .btn_box > .btn（确认修改地址） + .btn.info（取消，:showClose 时）
 * `.mp-address-edit` 包裹层承载 EditRegion.vue 的 <style scoped>
 * （见 app/member-pages.scss:197，由 scripts/gen-member-pages-scss.mjs 生成）。
 *
 * 校验（旧版 EditRegion.vue:5-28 的 rules）：收货人必填 maxlength=20；地区必选；
 * 详细地址必填、最少 3 字、maxlength=100；手机必填 maxlength=20；邮箱 maxlength=50。
 *
 * 后端契约：
 * - GET  /sys/region/getRegionListByRegionId?regionId=  省/市/区三级（routes/system/region_client.rs）
 * - GET  /user/address/detail?id=                        编辑回显
 * - POST /user/address/create | /user/address/update     新增 / 修改
 *
 * 跨境扩展（2026-11-18，v5.8.62 二期）：国家选择器 = getCountryList ∩
 * `@/lib/crossBorderAddress` 注册表（与 H5 同一口径，含 US/CA）；CN 保持
 * 原三级联动，非 CN 按注册表渲染州/省（有地区树子级时下拉、否则文本）
 * + 城市 + 地址行 + 邮编，保存时下发 countryCode/addressData/formattedAddress。
 */
import { useCallback, useEffect, useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { Modal, Select, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '@/lib/client-api';
import {
  buildAddressData,
  CROSS_BORDER_COUNTRIES,
  emptyCrossBorderValues,
  formatFullAddress,
  formatMainAddress,
  getCrossBorderCountry,
  type CrossBorderValues,
} from '@/lib/crossBorderAddress';

import { useTranslation } from 'react-i18next';

export interface AddressFormValue {
  consignee: string;
  mobile: string;
  mobileAreaCode: string;
  telephone: string;
  email: string;
  address: string;
  regionIds: number[];
}

interface Region {
  regionId: number;
  regionName: string;
  parentId?: number;
  level?: number;
  standardCode?: string;
}

interface AddressDetail {
  addressId?: number;
  consignee?: string;
  mobile?: string;
  mobileAreaCode?: string;
  telephone?: string;
  email?: string;
  address?: string;
  regionIds?: number[];
  regionNames?: string[];
  countryCode?: string;
  countryName?: string;
  addressData?: Record<string, string> | null;
  postcode?: string;
}

const EMPTY_FORM: AddressFormValue = {
  consignee: '',
  mobile: '',
  mobileAreaCode: '',
  telephone: '',
  email: '',
  address: '',
  regionIds: [],
};

/** 旧版 EditRegion.vue:16 `{ min: 3, message: '收货人地址过短' }` */
const ADDRESS_MIN_LENGTH = 3;

/** 旧版 a-form :label-col={span:3}（ant-design-vue 24 栅格 → 标签列 flex 0 0 12.5%）。
 *  控件列 :wrapper-col={span:16} / {span:8} 不写内联宽度：本页 <style scoped> 里
 *  `.ant-form-item-control { width: 100% }` 会把它撑满（见 app/member-pages.scss:197），
 *  照抄源码就应当由那条规则决定宽度，不能再用内联样式压回去。 */
const LABEL_COL: CSSProperties = { flex: '0 0 12.5%' };

export default function MemberAddressDialog({
  visible,
  addressId,
  onClose,
  onSuccess,
}: {
  visible: boolean;
  /** 0 表示新增；>0 表示编辑（走 detail 回显） */
  addressId?: number;
  onClose: () => void;
  onSuccess: () => void;
}) {
  const { t } = useTranslation();
  const [form, setForm] = useState<AddressFormValue>(EMPTY_FORM);
  const [provinces, setProvinces] = useState<Region[]>([]);
  const [cities, setCities] = useState<Region[]>([]);
  const [districts, setDistricts] = useState<Region[]>([]);
  const [, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);

  // —— 跨境地址（与 H5 同一注册表；CN 仍走下方原三级联动） ——
  const [countries, setCountries] = useState<Region[]>([]);
  const [countryCode, setCountryCode] = useState('CN');
  const [cbValues, setCbValues] = useState<CrossBorderValues>(emptyCrossBorderValues());
  /** 当前国家的一级地区（州/省）选项；为空时一级地区退化为文本输入（同 H5 动态表单） */
  const [stateOptions, setStateOptions] = useState<Region[]>([]);
  const [stateId, setStateId] = useState<number | undefined>(undefined);
  const country = getCrossBorderCountry(countryCode);
  const isOverseas = countryCode !== 'CN';

  const loadChildren = useCallback(async (parentId: number): Promise<Region[]> => {
    const list = await apiGet<Region[]>('sys/region/getRegionListByRegionId', { regionId: parentId }).catch(() => []);
    return Array.isArray(list) ? list : [];
  }, []);

  const loadProvinces = useCallback(async () => {
    const list = await apiGet<Region[]>('sys/region/getRegionListByRegionId', { regionId: 1 }).catch(() => []);
    setProvinces(Array.isArray(list) ? list : []);
  }, []);

  /** 国家列表：后端 level=1 地区行 ∩ 前端注册表（standardCode 匹配）。 */
  const loadCountries = useCallback(async () => {
    const list = await apiGet<Region[]>('sys/region/getCountryList').catch(() => []);
    const known = (Array.isArray(list) ? list : []).filter((item) =>
      CROSS_BORDER_COUNTRIES.some((c) => c.code === String(item.standardCode || '').toUpperCase()),
    );
    setCountries(known);
    return known;
  }, []);

  const reset = useCallback(() => {
    setForm(EMPTY_FORM);
    setCities([]);
    setDistricts([]);
    setCountryCode('CN');
    setCbValues(emptyCrossBorderValues());
    setStateOptions([]);
    setStateId(undefined);
  }, []);

  useEffect(() => {
    if (!visible) return;
    reset();
    void loadProvinces();
    void loadCountries();
  }, [visible, reset, loadProvinces, loadCountries]);

  /** 切换国家：重置跨境字段并拉取该国一级地区（州/省）。 */
  const switchCountry = async (code: string, keep?: { values?: CrossBorderValues; stateRegionId?: number }) => {
    setCountryCode(code);
    setCbValues(keep?.values || emptyCrossBorderValues());
    setStateId(undefined);
    setStateOptions([]);
    if (code === 'CN') return;
    const list = await loadCountries();
    const target = list.find((item) => String(item.standardCode || '').toUpperCase() === code);
    if (target) {
      const children = await loadChildren(target.regionId);
      setStateOptions(children);
      if (keep?.stateRegionId && children.some((item) => item.regionId === keep.stateRegionId)) {
        setStateId(keep.stateRegionId);
      }
    }
  };

  // 编辑模式：拉详情回显并按级联补齐市 / 区
  useEffect(() => {
    if (!visible || !addressId) return;
    let cancelled = false;
    void (async () => {
      setLoading(true);
      try {
        const detail = await apiGet<AddressDetail>('user/address/detail', { id: addressId }).catch(() => null);
        if (cancelled) return;
        const current = detail || {};
        const ids = current.regionIds || [];
        setForm({
          consignee: current.consignee || '',
          mobile: current.mobile || '',
          mobileAreaCode: current.mobileAreaCode || '',
          telephone: current.telephone || '',
          email: current.email || '',
          address: current.address || '',
          regionIds: ids,
        });
        // 跨境地址回显：按 addressData 还原注册表字段（addressLineOne → addressLine1）
        const detailCountry = String(current.countryCode || '').toUpperCase();
        if (detailCountry && detailCountry !== 'CN') {
          const data = current.addressData || {};
          const values = emptyCrossBorderValues();
          (Object.keys(values) as Array<keyof CrossBorderValues>).forEach((key) => {
            const dataKey = key === 'addressLine1' ? 'addressLineOne' : key;
            if (data[dataKey]) values[key] = String(data[dataKey]);
          });
          if (!values.postcode && current.postcode) values.postcode = current.postcode;
          await switchCountry(detailCountry, { values, stateRegionId: ids[1] });
          if (cancelled) return;
        }
        if (ids[0] && (!detailCountry || detailCountry === 'CN')) {
          const cityList = await loadChildren(ids[0]);
          if (cancelled) return;
          setCities(cityList);
          if (ids[1]) {
            const districtList = await loadChildren(ids[1]);
            if (cancelled) return;
            setDistricts(districtList);
          }
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [visible, addressId, loadChildren]);

  const onProvince = async (value: number) => {
    setForm((current) => ({ ...current, regionIds: [value] }));
    setDistricts([]);
    setCities(await loadChildren(value));
  };

  const onCity = async (value: number) => {
    setForm((current) => ({ ...current, regionIds: [current.regionIds[0], value] }));
    setDistricts(await loadChildren(value));
  };

  const onDistrict = (value: number) => {
    setForm((current) => ({ ...current, regionIds: [current.regionIds[0], current.regionIds[1], value] }));
  };

  const saveOverseas = async () => {
    const { consignee, mobile, telephone, email } = form;
    if (!consignee.trim()) return Toast.warning({ content: t('请您填写收货人姓名') });
    for (const fieldDef of [...country.regionFields, ...country.fields]) {
      if (fieldDef.required && !String(cbValues[fieldDef.key] || '').trim()) {
        return Toast.warning({ content: t(fieldDef.placeholder) });
      }
    }
    const postcode = String(cbValues.postcode || '').trim();
    if (postcode && country.postcodePattern && !country.postcodePattern.test(postcode)) {
      return Toast.warning({ content: t(country.postcodeMessage || '邮编格式不正确') });
    }
    // 手机号：去掉空格/短横与可选的 +区号 前缀后按本国规则校验本地号
    let localMobile = mobile.replace(/[\s-]/g, '');
    if (localMobile.startsWith(`+${country.dialCode}`)) localMobile = localMobile.slice(country.dialCode.length + 1);
    if (!localMobile) return Toast.warning({ content: t('请您填写收货人手机号码') });
    if (!country.mobilePattern.test(localMobile)) {
      return Toast.warning({ content: `${t('请输入有效的手机号码')}（${country.mobileHint}）` });
    }
    const countryRow = countries.find((item) => String(item.standardCode || '').toUpperCase() === countryCode);
    const regionIds: number[] = [];
    if (countryRow) regionIds.push(countryRow.regionId);
    if (stateId) regionIds.push(stateId);
    const regionNames = country.regionFields
      .map((fieldDef) => String(cbValues[fieldDef.key] || '').trim())
      .filter(Boolean);
    setSaving(true);
    try {
      const body = {
        consignee,
        mobile: localMobile,
        mobileAreaCode: country.dialCode,
        telephone,
        email,
        countryCode,
        countryName: country.name,
        regionIds,
        regionNames,
        address: formatMainAddress(countryCode, cbValues),
        addressData: buildAddressData(countryCode, cbValues),
        formattedAddress: formatFullAddress(countryCode, regionNames, cbValues),
        postcode,
        internationalMobile: `+${country.dialCode}${localMobile}`,
        addressId: addressId || undefined,
      };
      await apiPost(addressId ? 'user/address/update' : 'user/address/create', body);
      Toast.success({ content: t('操作成功') });
      onClose();
      onSuccess();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  const save = async () => {
    if (isOverseas) return saveOverseas();
    const { consignee, mobile, address, regionIds } = form;
    if (!consignee.trim()) return Toast.warning({ content: t('请您填写收货人姓名') });
    if (!regionIds.length) return Toast.warning({ content: t('请选择收货人所在地区') });
    if (!address.trim()) return Toast.warning({ content: t('请您填写详细收货地址') });
    if (address.trim().length < ADDRESS_MIN_LENGTH) return Toast.warning({ content: t('收货人地址过短') });
    if (!mobile.trim()) return Toast.warning({ content: t('请您填写收货人手机号码') });

    setSaving(true);
    try {
      const body = { ...form, regionIds, addressId: addressId || undefined };
      await apiPost(addressId ? 'user/address/update' : 'user/address/create', body);
      Toast.success({ content: t('操作成功') });
      onClose();
      onSuccess();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  /* 旧版每个 a-form-item 的 label 与控件，按模板顺序 1:1 展开 */
  const field = (
    label: string,
    control: ReactNode,
    key: string,
  ) => (
    <div className="ant-form-item" key={key}>
      <div className="ant-form-item-row">
        <div className="ant-form-item-label" style={LABEL_COL}>
          <label>{label}</label>
        </div>
        <div className="ant-form-item-control">
          <div className="ant-form-item-control-input">
            <div className="ant-form-item-control-input-content">{control}</div>
          </div>
        </div>
      </div>
    </div>
  );

  return (
    <Modal
      title={addressId ? t('编辑收货地址') : t('添加收货地址')}
      visible={visible}
      width={880}
      maskClosable
      onCancel={onClose}
      footer={null}
    >
      {/* 旧版 EditRegion.vue:3 <div class="modal_region_box">，`.mp-address-edit` 承载本组件 <style scoped> */}
      <div className="modal_region_box mp-address-edit">
        {/* 旧版 EditRegion.vue:4  a-form :label-col={span:3} :wrapper-col={span:16} autocomplete="off" */}
        <form className="ant-form ant-form-horizontal" autoComplete="off" onSubmit={(event) => event.preventDefault()}>
          {field(
            t('收货人：'),
            <input
              className="ant-input"
              style={{ width: 300 }}
              maxLength={20}
              value={form.consignee}
              placeholder={t('请您填写收货人姓名')}
              onChange={(event) => setForm((current) => ({ ...current, consignee: event.target.value }))}
            />,
            'consignee',
          )}
          {countries.length > 0 &&
            field(
              t('国家/地区：'),
              <Select
                value={countryCode}
                style={{ width: 300 }}
                onChange={(value) => void switchCountry(String(value))}
                showClear={false}
              >
                {countries.map((item) => (
                  <Select.Option key={item.regionId} value={String(item.standardCode || '').toUpperCase()}>
                    {t(getCrossBorderCountry(String(item.standardCode || '')).name)}
                  </Select.Option>
                ))}
              </Select>,
              'countryCode',
            )}
          {!isOverseas &&
            field(
              t('所在地区：'),
              <div className="select_region" style={{ display: 'flex', gap: 10 }}>
                <Select value={form.regionIds[0]} placeholder={t('省份')} onChange={(value) => void onProvince(Number(value))} showClear={false}>
                  {provinces.map((item) => <Select.Option key={item.regionId} value={item.regionId}>{item.regionName}</Select.Option>)}
                </Select>
                <Select value={form.regionIds[1]} placeholder={t('城市')} onChange={(value) => void onCity(Number(value))} showClear={false} disabled={!cities.length}>
                  {cities.map((item) => <Select.Option key={item.regionId} value={item.regionId}>{item.regionName}</Select.Option>)}
                </Select>
                <Select value={form.regionIds[2]} placeholder={t('区县')} onChange={(value) => onDistrict(Number(value))} showClear={false} disabled={!districts.length}>
                  {districts.map((item) => <Select.Option key={item.regionId} value={item.regionId}>{item.regionName}</Select.Option>)}
                </Select>
              </div>,
              'regionIds',
            )}
          {isOverseas &&
            country.regionFields.map((fieldDef, index) =>
              field(
                `${t(fieldDef.label)}：`,
                index === 0 && stateOptions.length > 0 ? (
                  <Select
                    value={stateId}
                    style={{ width: 300 }}
                    placeholder={t(fieldDef.placeholder)}
                    onChange={(value) => {
                      const id = Number(value);
                      const hit = stateOptions.find((item) => item.regionId === id);
                      setStateId(id);
                      setCbValues((current) => ({ ...current, [fieldDef.key]: hit?.regionName || '' }));
                    }}
                    showClear={false}
                  >
                    {stateOptions.map((item) => <Select.Option key={item.regionId} value={item.regionId}>{item.regionName}</Select.Option>)}
                  </Select>
                ) : (
                  <input
                    className="ant-input"
                    style={{ width: 300 }}
                    maxLength={100}
                    value={cbValues[fieldDef.key]}
                    placeholder={t(fieldDef.placeholder)}
                    onChange={(event) => setCbValues((current) => ({ ...current, [fieldDef.key]: event.target.value }))}
                  />
                ),
                `cb-region-${fieldDef.key}`,
              ),
            )}
          {isOverseas &&
            country.fields.map((fieldDef) =>
              field(
                `${t(fieldDef.label)}：`,
                <input
                  className="ant-input"
                  style={{ width: 300 }}
                  maxLength={100}
                  value={cbValues[fieldDef.key]}
                  placeholder={t(fieldDef.placeholder)}
                  onChange={(event) => setCbValues((current) => ({ ...current, [fieldDef.key]: event.target.value }))}
                />,
                `cb-field-${fieldDef.key}`,
              ),
            )}
          {!isOverseas &&
            field(
              t('详细地址：'),
              <input
                className="ant-input"
                style={{ width: 300 }}
                maxLength={100}
                value={form.address}
                placeholder={t('请您填写详细收货地址')}
                onChange={(event) => setForm((current) => ({ ...current, address: event.target.value }))}
              />,
              'address',
            )}
          {field(
            t('手机号码：'),
            isOverseas ? (
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
                <span className="ant-input" style={{ width: 64, textAlign: 'center' }}>+{country.dialCode}</span>
                <input
                  className="ant-input"
                  style={{ width: 228 }}
                  maxLength={20}
                  value={form.mobile}
                  placeholder={t('请您填写收货人手机号码')}
                  onChange={(event) => setForm((current) => ({ ...current, mobile: event.target.value }))}
                />
              </span>
            ) : (
              <input
                className="ant-input"
                style={{ width: 300 }}
                maxLength={20}
                value={form.mobile}
                placeholder={t('请您填写收货人手机号码')}
                onChange={(event) => setForm((current) => ({ ...current, mobile: event.target.value }))}
              />
            ),
            'mobile',
          )}
          {/* 旧版这两项 :rules="[]" 且 :wrapper-col={span:8} */}
          {field(
            t('固定电话：'),
            <input
              className="ant-input"
              style={{ width: 300 }}
              maxLength={20}
              value={form.telephone}
              onChange={(event) => setForm((current) => ({ ...current, telephone: event.target.value }))}
            />,
            'telephone',
          )}
          {field(
            t('邮箱地址：'),
            <input
              className="ant-input"
              style={{ width: 300 }}
              maxLength={50}
              value={form.email}
              onChange={(event) => setForm((current) => ({ ...current, email: event.target.value }))}
            />,
            'email',
          )}
        </form>

        {/* 旧版 EditRegion.vue:31-39 <div class="address_from">（旧版 :showFooter="false"，按钮在这里不在弹窗 footer） */}
        <div className="address_from">
          <div className="from_item">
            <div className="tit"></div>
            <div className="btn_box">
              <div className="btn" onClick={() => { if (!saving) void save(); }}>{t('确认修改地址')}</div>
              <div className="btn info" onClick={onClose}>{t('取消')}</div>
            </div>
          </div>
        </div>
      </div>
    </Modal>
  );
}
