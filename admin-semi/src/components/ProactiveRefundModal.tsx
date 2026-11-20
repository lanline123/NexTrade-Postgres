import { useEffect, useState} from 'react';
import { Button, InputNumber, Modal, Spin, TextArea, Toast, Typography} from '@douyinfe/semi-ui';
import { apiGet, apiPost} from '../api';
import { useI18n} from '../i18n';

const { Text} = Typography;

interface ProactiveRefundModalProps {
visible: boolean;
orderId?: number | null;
onCancel: () => void;
onSuccess?: () => void;
}

interface ProactiveRefundPreview {
orderId: number;
orderSn: string;
paidAmount: string | number;
refundableAmount: string | number;
isSettlement: number;
}

/**
* 主动退款弹窗：平台/商家对已支付订单协商退款
* （退差价等场景）。打开时向服务端预检（订单状态门 + 店铺
* 作用域强校验），金额默认 = 可退余额，原因必填；提交后生成
* 待审核退款申请，走财务审核出款链，不回补库存。
*/
export default function ProactiveRefundModal({ visible, orderId, onCancel, onSuccess}: ProactiveRefundModalProps) {
const { t} = useI18n();
const [loading, setLoading] = useState(false);
const [submitting, setSubmitting] = useState(false);
const [preview, setPreview] = useState<ProactiveRefundPreview | null>(null);
const [amount, setAmount] = useState<number | string>('');
const [reason, setReason] = useState('');

useEffect(() => {
if (!visible ||!orderId) return;
let alive = true;
setLoading(true);
setPreview(null);
setReason('');
setAmount('');
apiGet<ProactiveRefundPreview>('/finance/refundApply/proactivePreview', { orderId})
.then((result) => {
if (!alive) return;
setPreview(result);
setAmount(Number(result.refundableAmount || 0));
})
.catch((error) => {
if (!alive) return;
Toast.error({ content: error instanceof Error? error.message: t('加载失败')});
onCancel();
})
.finally(() => {
if (alive) setLoading(false);
});
return () => {
alive = false;
};
// eslint-disable-next-line react-hooks/exhaustive-deps
}, [visible, orderId]);

const refundable = Number(preview?.refundableAmount || 0);

const submit = async () => {
if (!orderId) return;
const value = Number(amount);
if (!value || value <= 0) {
Toast.warning({ content: t('退款金额必须大于0')});
return;
}
if (value > refundable) {
Toast.warning({ content: t('退款金额不能超过可退余额')});
return;
}
if (!reason.trim()) {
Toast.warning({ content: t('退款原因不能为空')});
return;
}
setSubmitting(true);
try {
await apiPost('/finance/refundApply/proactiveCreate', {
orderId,
amount: value,
reason: reason.trim(),
});
Toast.success({ content: t('已发起主动退款，等待财务审核')});
onSuccess?.();
onCancel();
} catch (error) {
Toast.error({ content: error instanceof Error? error.message: t('发起失败')});
} finally {
setSubmitting(false);
}
};

return (
<Modal
title={t('主动退款')}
visible={visible}
onCancel={onCancel}
width={480}
footer={
<>
<Button onClick={onCancel}>{t('取消')}</Button>
<Button
theme="solid"
type="primary"
loading={submitting}
disabled={loading ||!preview}
onClick={() => void submit()}
>
{t('确认发起')}
</Button>
</>
}
>
<Spin spinning={loading}>
{preview && (
<div style={{ display: 'flex', flexDirection: 'column', gap: 14}}>
<div>
<Text type="tertiary">{t('订单号')}：</Text>
<span className="mono">{preview.orderSn}</span>
</div>
<div style={{ display: 'flex', gap: 24}}>
<div>
<Text type="tertiary">{t('订单实付')}：</Text>¥ {Number(preview.paidAmount || 0).toFixed(2)}
</div>
<div>
<Text type="tertiary">{t('可退余额')}：</Text>¥ {refundable.toFixed(2)}
</div>
</div>
<div>
<div style={{ marginBottom: 6}}>
<Text strong>{t('退款金额')}</Text>
</div>
<InputNumber
value={amount}
onChange={(value) => setAmount(value?? '')}
min={0.01}
max={refundable}
step={0.01}
precision={2}
style={{ width: '100%'}}
/>
</div>
<div>
<div style={{ marginBottom: 6}}>
<Text strong>{t('退款原因')}</Text> <Text type="danger">*</Text>
</div>
<TextArea
value={reason}
onChange={setReason}
maxCount={255}
rows={3}
placeholder={t('如：协商退差价（必填，将同步通知买家）')}
/>
</div>
{Number(preview.isSettlement) === 1 && (
<Text type="tertiary" size="small">
{t('该订单已结算，退款完成后将从商家结算款中扣回。')}
</Text>
)}
<Text type="tertiary" size="small">
{t('提交后生成待审核退款申请，财务审核通过后原路退回买家；本操作不回补库存。')}
</Text>
</div>
)}
</Spin>
</Modal>
);
}
