import { useEffect, useState } from 'react';
import {
  Button,
  Card,
  InputNumber,
  Space,
  Spin,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

const { Text, Title } = Typography;

/**
 * 送礼物设置（设置 → 商城设置 → 送礼物设置，我方新增）。
 *
 * 买家在结算时选「送礼物」：付款后订单进入待领取，好友在领取
 * 有效期内登录并填写自己的收货地址领取；超时未领取自动原路
 * 退款给买家。本页只配领取有效期（天），存后端 config code
 * `giftOrderSettings`（默认 7 天，合法区间 1–90 天）。
 */
export default function GiftOrderSettings() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [claimExpireDays, setClaimExpireDays] = useState<number>(7);

  useEffect(() => {
    (async () => {
      try {
        const data = await apiGet<{ claimExpireDays?: number }>(
          '/setting/config/giftOrderSettings',
        );
        if (data && typeof data.claimExpireDays === 'number') {
          setClaimExpireDays(data.claimExpireDays);
        }
      } catch {
        Toast.error('加载送礼物设置失败');
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const save = async () => {
    const days = Math.trunc(claimExpireDays || 0);
    if (!days || days < 1 || days > 90) {
      Toast.warning('领取有效期需在 1–90 天之间');
      return;
    }
    setSaving(true);
    try {
      const data = await apiPost<{ claimExpireDays?: number }>(
        '/setting/config/saveGiftOrderSettings',
        { claimExpireDays: days },
      );
      if (data && typeof data.claimExpireDays === 'number') {
        setClaimExpireDays(data.claimExpireDays);
      }
      Toast.success('已保存');
    } catch {
      Toast.error('保存失败');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={{ padding: 16 }}>
      <Title heading={4} style={{ marginBottom: 4 }}>
        送礼物设置
      </Title>
      <Text type="tertiary">
        买家结算时可选「送礼物」：不填收货地址、付款后把领取链接分享给好友，好友登录填写自己的地址领取。超时未领取将自动原路退款给买家。
      </Text>
      <Card style={{ marginTop: 16, maxWidth: 640 }}>
        <Spin spinning={loading}>
          <Space align="center" spacing={12}>
            <Text strong>领取有效期</Text>
            <InputNumber
              min={1}
              max={90}
              step={1}
              precision={0}
              value={claimExpireDays}
              onChange={(v) => setClaimExpireDays(typeof v === 'number' ? v : 7)}
              style={{ width: 120 }}
            />
            <Text>天（付款成功后起算，默认 7 天）</Text>
          </Space>
          <div style={{ marginTop: 8 }}>
            <Text type="tertiary" size="small">
              有效期只影响新付款的礼物订单；已在待领取中的订单仍按其付款时起算的截止时间执行。修改后请保存。
            </Text>
          </div>
          <div style={{ marginTop: 16 }}>
            <Button theme="solid" loading={saving} onClick={save}>
              保存
            </Button>
          </div>
        </Spin>
      </Card>
    </div>
  );
}
