import { useEffect, useState } from 'react';
import { Button, Modal, Space, Switch, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';

interface RechargeRow {
  rechargeId: number;
  money?: number | string;
  discountMoney?: number | string;
  isShow?: number;
  sortOrder?: number;
  /** 赠送实物礼品（我方新增）：活动赠品池 gift_id，0 = 不送 */
  giftId?: number;
  giftQuantity?: number;
  giftName?: string;
}

interface GiftOptionRow {
  giftId: number;
  giftName?: string;
  giftStock?: number;
}

/** 充值设置：对应旧版 promotion/rechargeSetting/List.vue + Info.vue。
 * 「实物赠品」两字段为我方新增（2026-11-21 论坛帖 717721），原版仅有赠送金额。 */
export default function RechargeSettings() {
  const { t } = useI18n();
  const [reloadKey, setReloadKey] = useState(0);
  const [giftOptions, setGiftOptions] = useState<Array<{ label: string; value: number }>>([
    { label: '不送实物礼品', value: 0 },
  ]);

  const reload = () => setReloadKey((key) => key + 1);

  // 赠品下拉数据：既有活动赠品池（营销 → 活动赠品），档位只存 gift_id 一份引用
  useEffect(() => {
    apiGet<{ records?: GiftOptionRow[]; list?: GiftOptionRow[] }>('/promotion/productGift/list', {
      page: 1,
      size: 100,
    })
      .then((res) => {
        const rows = (res as { records?: GiftOptionRow[]; list?: GiftOptionRow[] })?.records
          || (res as { list?: GiftOptionRow[] })?.list
          || [];
        setGiftOptions([
          { label: '不送实物礼品', value: 0 },
          ...rows.map((g) => ({
            label: `${g.giftName || `赠品 ${g.giftId}`}（库存 ${g.giftStock ?? 0}）`,
            value: g.giftId,
          })),
        ]);
      })
      .catch(() => undefined);
  }, []);

  const updateField = async (row: RechargeRow, field: string, value: unknown) => {
    try {
      await apiPost('/promotion/rechargeSetting/updateField', { id: row.rechargeId, field, value });
      Toast.success({ content: t('修改成功') });
      reload();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('修改失败') });
    }
  };

  const columns: ColumnConfig<RechargeRow>[] = [
    {
      title: '充值金额',
      dataIndex: 'money',
      width: 160,
      render: (value: unknown) => <span title={t('充值金额')}>{value != null ? String(value) : '-'}</span>,
    },
    {
      title: '赠送金额',
      dataIndex: 'discountMoney',
      width: 160,
      render: (value: unknown) => <span title={t('赠送金额')}>{value != null ? String(value) : '-'}</span>,
    },
    {
      title: '赠送实物',
      dataIndex: 'giftName',
      width: 200,
      render: (_value: unknown, row: RechargeRow) =>
        row.giftId && row.giftId > 0 ? (
          <span>{`${row.giftName || `赠品 ${row.giftId}`} × ${row.giftQuantity ?? 1}`}</span>
        ) : (
          <span>-</span>
        ),
    },
    {
      title: '是否显示',
      dataIndex: 'isShow',
      width: 110,
      render: (value: number, row: RechargeRow) => (
        <Switch checked={Number(value) === 1} onChange={(next) => updateField(row, 'isShow', next ? 1 : 0)} />
      ),
    },
    { title: '排序', dataIndex: 'sortOrder', width: 90 },
  ];

  return (
    <ListPage<RechargeRow>
      key={reloadKey}
      api="/promotion/rechargeSetting/list"
      rowKey="rechargeId"
      pageSize={10}
      selectable
      actionWidth={150}
      filters={[
        { field: 'keyword', label: '充值金额', type: 'input', placeholder: '输入关键词', width: 220 },
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
                  await apiPost('/promotion/rechargeSetting/batch', {
                    ids: rows.map((row) => row.rechargeId),
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
      drawer={{
        title: '充值档位',
        addText: '添加充值档位',
        width: 520,
        labelWidth: 'auto',
        createApi: '/promotion/rechargeSetting/create',
        updateApi: '/promotion/rechargeSetting/update',
        detailApi: '/promotion/rechargeSetting/detail',
        deleteApi: '/promotion/rechargeSetting/del',
        idField: 'rechargeId',
        sections: [
          {
            key: 'basic',
            title: '档位信息',
            description:
              '提示：充值附赠赠送余额时，强烈建议关闭会员余额提现功能，避免产生资金套利风险。（去关闭：财务-提现管理-提现设置）',
            fields: [
              { field: 'money', label: t('充值金额'), type: 'number', required: true },
              { field: 'discountMoney', label: t('赠送金额'), type: 'number', required: true },
              { field: 'sortOrder', label: t('排序'), type: 'number', initValue: 1 },
              {
                field: 'isShow',
                label: t('是否显示'),
                type: 'radio',
                initValue: 1,
                options: [
                  { label: t('是'), value: 1 },
                  { label: t('否'), value: 0 },
                ],
              },
            ],
          },
          {
            key: 'gift',
            title: '实物赠品（可选）',
            description:
              '与赠送金额相互独立、可叠加：会员充值该档位并经后台确认到账后生成一条待领取赠品，会员在「我的赠品」选地址领取，系统生成赠品订单由对应商家发货（包邮）。赠品来自「营销 - 活动赠品」池，领取时扣减赠品库存；库存不足时记录会标记异常，可在「充值赠品记录」中处理。',
            fields: [
              {
                field: 'giftId',
                label: t('赠送礼品'),
                type: 'select',
                initValue: 0,
                options: giftOptions,
              },
              {
                field: 'giftQuantity',
                label: t('赠送数量'),
                type: 'number',
                initValue: 1,
                extraText: '选择赠品后生效（每笔充值赠送件数）',
              },
            ],
          },
        ],
        toPayload: (values) => ({
          ...values,
          money: Number(values.money ?? 0),
          discountMoney: Number(values.discountMoney ?? 0),
          sortOrder: Number(values.sortOrder ?? 1),
          isShow: Number(values.isShow ?? 1),
          giftId: Number(values.giftId ?? 0),
          giftQuantity: Number(values.giftQuantity ?? 1),
        }),
      }}
    />
  );
}
