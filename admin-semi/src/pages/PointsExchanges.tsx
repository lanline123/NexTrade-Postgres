import { useEffect, useState } from 'react';
import { Button, Modal, Select, Space, Switch, Tag, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import SkuPicker, { type SkuPick } from '../components/SkuPicker';
import type { ColumnConfig } from '../framework/types';

interface PointsExchangeRow {
  pointsExchangeId: number;
  productId?: number;
  productName?: string;
  exchangeIntegral?: number;
  pointsDeductedAmount?: number | string;
  isEnabled?: number;
  isHot?: number;
  /** 兑换类型（我方新增）：1=商品 2=优惠券 */
  exchangeType?: number;
  couponId?: number;
  couponName?: string;
  couponMoney?: number | string;
  minOrderAmount?: number | string;
  effectiveExchangeIntegral?: number;
  integralSource?: string;
}

interface CouponOption {
  couponId: number;
  couponName: string;
  couponMoney?: number | string;
  minOrderAmount?: number | string;
}

/** 积分商品：对应旧版 promotion/pointsExchange/List.vue + Info.vue。
 * 券型兑换品为我方新增（论坛帖 517907）：类型=优惠券、选券模板、
 * 兑换积分可空（空/0 = 跟随平台统一比「积分兑券比例」自动折算）。 */
export default function PointsExchanges() {
  const { t } = useI18n();
  const [reloadKey, setReloadKey] = useState(0);
  const [coupons, setCoupons] = useState<CouponOption[]>([]);
  // 平台统一比生效值（预览折算用，与后端解析同序：兑券比 → 积分换算比 → 1）
  const [couponScale, setCouponScale] = useState(1);

  useEffect(() => {
    (async () => {
      try {
        const res: any = await apiGet('/promotion/coupon/list', { page: 1, pageSize: 100 });
        const records = res?.records || res?.data?.records || [];
        setCoupons(
          records.map((c: any) => ({
            couponId: Number(c.couponId),
            couponName: c.couponName,
            couponMoney: c.couponMoney,
            minOrderAmount: c.minOrderAmount,
          })),
        );
      } catch {
        // 券列表加载失败时选择器为空，后端保存校验仍会拦截
      }
      try {
        const cfg: any = await apiGet('/setting/config/shoppingSettings');
        const data = cfg?.data || cfg || {};
        const dedicated = Number(data.integralCouponScale);
        const base = Number(data.integralScale);
        setCouponScale(dedicated > 0 ? dedicated : base > 0 ? base : 1);
      } catch {
        setCouponScale(1);
      }
    })();
  }, []);

  const reload = () => setReloadKey((key) => key + 1);

  const updateField = async (row: PointsExchangeRow, field: string, value: unknown) => {
    try {
      await apiPost('/promotion/pointsExchange/updateField', { id: row.pointsExchangeId, field, value });
      Toast.success({ content: t('修改成功') });
      reload();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('修改失败') });
    }
  };

  const columns: ColumnConfig<PointsExchangeRow>[] = [
    {
      title: '积分商品名称',
      dataIndex: 'productName',
      width: 350,
      render: (value: string, row: PointsExchangeRow) => (
        <span>{Number(row.exchangeType) === 2 ? row.couponName || '-' : value || '-'}</span>
      ),
    },
    {
      title: '类型',
      dataIndex: 'exchangeType',
      width: 90,
      render: (value: unknown) =>
        Number(value) === 2 ? <Tag color="blue">{t('优惠券')}</Tag> : <Tag>{t('商品')}</Tag>,
    },
    {
      title: '使用积分值',
      dataIndex: 'exchangeIntegral',
      width: 150,
      render: (value: unknown, row: PointsExchangeRow) => {
        if (Number(row.exchangeType) === 2) {
          const effective = row.effectiveExchangeIntegral;
          return (
            <span title={t('使用积分值')}>
              {effective != null ? String(effective) : '-'}
              {row.integralSource === 'platform' ? (
                <span style={{ color: '#999', fontSize: 12 }}>（{t('平台比')}）</span>
              ) : null}
            </span>
          );
        }
        return <span title={t('使用积分值')}>{value != null ? String(value) : '-'}</span>;
      },
    },
    {
      title: '减免金额',
      dataIndex: 'pointsDeductedAmount',
      width: 120,
      render: (value: unknown, row: PointsExchangeRow) => (
        <span title={t('减免金额')}>
          {Number(row.exchangeType) === 2
            ? row.couponMoney != null
              ? String(row.couponMoney)
              : '-'
            : value != null
              ? String(value)
              : '-'}
        </span>
      ),
    },
    {
      title: '是否可兑换',
      dataIndex: 'isEnabled',
      width: 120,
      render: (value: number, row: PointsExchangeRow) => (
        <span title={Number(value) === 1 ? t('可兑换') : t('不可兑换')}><Switch checked={Number(value) === 1} onChange={(next) => updateField(row, 'isEnabled', next ? 1 : 0)} /></span>
      ),
    },
    {
      title: '是否热销',
      dataIndex: 'isHot',
      width: 110,
      render: (value: number, row: PointsExchangeRow) => (
        <span title={Number(value) === 1 ? t('热销') : t('不热销')}><Switch checked={Number(value) === 1} onChange={(next) => updateField(row, 'isHot', next ? 1 : 0)} /></span>
      ),
    },
  ];

  return (
    <ListPage<PointsExchangeRow>
      key={reloadKey}
      api="/promotion/pointsExchange/list"
      rowKey="pointsExchangeId"
      pageSize={10}
      selectable
      actionWidth={150}
      filters={[
        { field: 'keyword', label: '积分商品', type: 'input', placeholder: '输入商品或优惠券名称', width: 240 },
      ]}
      columns={columns}
      batchActions={(rows, clear) => (
        <Button
          size="small"
          onClick={() => {
            Modal.confirm({
              title: t('您确认要批量删除所选数据吗？'),
              onOk: async () => {
                try {
                  await apiPost('/promotion/pointsExchange/batch', {
                    ids: rows.map((row) => row.pointsExchangeId),
                    type: 'del',
                  });
                  Toast.success({ content: t('批量删除成功') });
                  clear();
                  reload();
                } catch (error) {
                  Toast.error({ content: error instanceof Error ? error.message : t('批量删除失败') });
                }
              },
            });
          }}
        >
          {t('批量删除')}
        </Button>
      )}
      rowActions={(row) => (
        <Space spacing={2}>
          <Button theme="borderless" size="small">
            {t('编辑')}
          </Button>
          <Button
            theme="borderless"
            type="danger"
            size="small"
            onClick={() => {
              Modal.confirm({
                title: t('确认删除该积分商品吗？'),
                onOk: async () => {
                  try {
                    await apiPost('/promotion/pointsExchange/del', { id: row.pointsExchangeId });
                    Toast.success({ content: t('删除成功') });
                    reload();
                  } catch (error) {
                    Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
                  }
                },
              });
            }}
          >
            {t('删除')}
          </Button>
        </Space>
      )}
      drawer={{
        title: '积分商品',
        addText: '添加积分商品',
        width: 560,
        labelWidth: 'auto',
        createApi: '/promotion/pointsExchange/create',
        updateApi: '/promotion/pointsExchange/update',
        detailApi: '/promotion/pointsExchange/detail',
        deleteApi: '/promotion/pointsExchange/del',
        idField: 'pointsExchangeId',
        sections: [
          {
            key: 'basic',
            title: '兑换设置',
            fields: [
              {
                field: 'exchangeType',
                label: t('兑换类型'),
                type: 'radio',
                initValue: 1,
                options: [
                  { label: t('商品'), value: 1 },
                  { label: t('优惠券'), value: 2 },
                ],
              },
              { field: 'exchangeIntegral', label: t('兑换积分'), type: 'number', extraText: t('商品型必填；券型可留空/填0=跟随平台统一比（积分兑券比例），填正数=该品自设积分价') },
              { field: 'pointsDeductedAmount', label: t('减免金额'), type: 'number', extraText: t('仅商品型使用，券型忽略') },
              {
                field: 'isEnabled',
                label: t('兑换'),
                type: 'radio',
                initValue: 1,
                options: [
                  { label: t('是'), value: 1 },
                  { label: t('否'), value: 0 },
                ],
              },
              {
                field: 'isHot',
                label: t('是否热销'),
                type: 'radio',
                initValue: 0,
                options: [
                  { label: t('是'), value: 1 },
                  { label: t('否'), value: 0 },
                ],
              },
              {
                field: 'productId',
                label: t('选择商品'),
                type: 'custom',
                render: ({ formApi, values }) => {
                  if (Number(values.exchangeType ?? 1) === 2) {
                    return <span style={{ color: '#999' }}>{t('（券型不适用）')}</span>;
                  }
                  return (
                    <div>
                      <SkuPicker
                        value={Array.isArray(values.skuPick)
                          ? (values.skuPick as SkuPick[])
                          : values.productId
                            ? ([{ productId: Number(values.productId), skuId: Number(values.skuId || 0) }] as SkuPick[])
                            : []}
                        onChange={(picks) => {
                          formApi?.setValue('skuPick', picks);
                          formApi?.setValue('productId', picks[0]?.productId);
                          formApi?.setValue('skuId', picks[0]?.skuId ?? 0);
                        }}
                        max={1}
                      />
                    </div>
                  );
                },
              },
              {
                field: 'couponId',
                label: t('选择优惠券'),
                type: 'custom',
                render: ({ formApi, values }) => {
                  if (Number(values.exchangeType ?? 1) !== 2) {
                    return <span style={{ color: '#999' }}>{t('（商品型不适用）')}</span>;
                  }
                  const selected = coupons.find((c) => c.couponId === Number(values.couponId));
                  const integral = Number(values.exchangeIntegral ?? 0);
                  const money = Number(selected?.couponMoney ?? 0);
                  const preview =
                    selected && integral <= 0 && money > 0
                      ? Math.ceil((money * 100) / couponScale)
                      : null;
                  return (
                    <div>
                      <Select
                        filter
                        showClear
                        style={{ width: '100%' }}
                        placeholder={t('请选择优惠券模板')}
                        value={values.couponId ? Number(values.couponId) : undefined}
                        onChange={(v) => formApi?.setValue('couponId', v)}
                      >
                        {coupons.map((c) => (
                          <Select.Option key={c.couponId} value={c.couponId}>
                            {`${c.couponName}（${Number(c.couponMoney) > 0 ? `¥${c.couponMoney}` : t('折扣券')} / ${t('门槛')} ¥${c.minOrderAmount ?? 0}）`}
                          </Select.Option>
                        ))}
                      </Select>
                      {selected ? (
                        <div style={{ marginTop: 6, color: '#666', fontSize: 12 }}>
                          {integral > 0
                            ? t('已自设积分价，以自设为准')
                            : preview != null
                              ? `${t('跟随平台统一比')}：${t('约')} ${preview} ${t('积分')}`
                              : t('该券无固定面额，跟随平台比无法折算，请自设兑换积分')}
                        </div>
                      ) : null}
                    </div>
                  );
                },
              },
            ],
          },
        ],
        toPayload: (values) => {
          const base = {
            isEnabled: Number(values.isEnabled ?? 1),
            isHot: Number(values.isHot ?? 0),
          };
          if (Number(values.exchangeType ?? 1) === 2) {
            if (!values.couponId) {
              throw new Error(t('请选择优惠券!'));
            }
            return {
              ...values,
              ...base,
              exchangeType: 2,
              couponId: Number(values.couponId),
              productId: 0,
              skuId: 0,
              exchangeIntegral: Number(values.exchangeIntegral ?? 0),
              pointsDeductedAmount: 0,
            };
          }
          const picks = (values.skuPick as SkuPick[]) || [];
          const productIds = picks.length
            ? picks.map((item) => item.productId)
            : Array.isArray(values.productId)
              ? values.productId
              : values.productId
                ? [Number(values.productId)]
                : [];
          if (!productIds.length) {
            throw new Error(t('请选择商品!'));
          }
          if (!(Number(values.exchangeIntegral ?? 0) > 0)) {
            throw new Error(t('请输入兑换积分'));
          }
          return {
            ...values,
            ...base,
            exchangeType: 1,
            couponId: undefined,
            productId: productIds[0],
            // SKU 级选择（对齐旧版 SelectProduct isSku）：skuId=0 表示商品级
            skuId: picks[0]?.skuId ?? Number(values.skuId || 0),
            exchangeIntegral: Number(values.exchangeIntegral ?? 0),
            pointsDeductedAmount: Number(values.pointsDeductedAmount ?? 0),
          };
        },
      }}
    />
  );
}
