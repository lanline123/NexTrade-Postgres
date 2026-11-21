import { Form } from '@douyinfe/semi-ui';
import { useI18n } from '../i18n';
import MapLocationPicker from './MapLocationPicker';
import { isValidLngLat, type LngLat } from './mapShared';

/**
 * 门店/自提点表单的「位置」字段块（框架 custom 字段渲染）：
 * 经度/纬度数字框（注册为 Semi 表单字段 shopLongitude/shopLatitude，
 * 提交时由页面 toPayload 转字符串——后端 Decimal 走 serde-with-str）
 * + 地图选点（MapLocationPicker）。选点回填经纬度，并把逆地理地址
 * 同步回填 shopDetailedAddress；手工改经纬度时地图标记跟随。
 */
export default function ShopLocationField({
  formApi,
  values,
}: {
  formApi: any;
  values: Record<string, any>;
}) {
  const { t } = useI18n();
  const lng = Number(values?.shopLongitude);
  const lat = Number(values?.shopLatitude);
  const point: LngLat | null =
    Number.isFinite(lng) && Number.isFinite(lat) && (lng !== 0 || lat !== 0)
      ? [lng, lat]
      : null;

  return (
    <div className="shop-location-field">
      <div className="shop-location-field__coords">
        <Form.InputNumber
          field="shopLongitude"
          label={t('经度')}
          placeholder={t('如 120.123456')}
          min={-180}
          max={180}
          precision={6}
          step={0.000001}
          style={{ width: 190 }}
          rules={[{ required: true, message: t('请填写经度，或在下方地图上选点') }]}
        />
        <Form.InputNumber
          field="shopLatitude"
          label={t('纬度')}
          placeholder={t('如 30.123456')}
          min={-90}
          max={90}
          precision={6}
          step={0.000001}
          style={{ width: 190 }}
          rules={[{ required: true, message: t('请填写纬度，或在下方地图上选点') }]}
        />
      </div>
      <MapLocationPicker
        value={isValidLngLat(point) ? point : null}
        defaultKeyword={
          typeof values?.shopDetailedAddress === 'string'
            ? values.shopDetailedAddress
            : undefined
        }
        onChange={(next, address) => {
          formApi?.setValue('shopLongitude', next[0]);
          formApi?.setValue('shopLatitude', next[1]);
          if (address) {
            formApi?.setValue('shopDetailedAddress', address);
          }
        }}
      />
    </div>
  );
}
