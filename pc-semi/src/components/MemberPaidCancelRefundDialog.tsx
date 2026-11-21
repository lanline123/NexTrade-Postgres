'use client';

/**
 * 取消并退款弹窗（PC 会员中心 · 订单列表 / 订单详情共用）。我方新增，
 * 论坛帖 347748，用户 2026-11-21 拍板口径。
 *
 * 已支付未发货订单的买家自助取消：二次确认 + 可选取消原因，
 * 提交 `POST /user/order/cancelPaidRefund`（参数 `{ orderId, reason }`，
 * 金额由服务端按实付推导，不经前端）。退款按平台设置走
 * 小额免审 / 财务审核队列 / 直达出款，完成后订单关闭；
 * 入口是否出现由后端 `availableActions.cancelPaidRefund` 驱动
 * （设置主开关关时恒不下发）。
 */
import { useEffect, useState } from 'react';
import { Button, Modal, TextArea, Toast, Typography } from '@douyinfe/semi-ui';
import { apiPost } from '@/lib/client-api';
import { useTranslation } from 'react-i18next';

const { Text } = Typography;

export default function MemberPaidCancelRefundDialog({
  visible,
  orderId,
  onClose,
  onSuccess,
}: {
  visible: boolean;
  orderId: number;
  onClose: () => void;
  onSuccess: () => void;
}) {
  const { t } = useTranslation();
  const [reason, setReason] = useState('');
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (visible) setReason('');
  }, [visible, orderId]);

  const submit = async () => {
    if (!orderId || submitting) return;
    setSubmitting(true);
    try {
      await apiPost('user/order/cancelPaidRefund', {
        orderId,
        reason: reason.trim() || undefined,
      });
      Toast.success({ content: t('已提交取消退款申请') });
      onClose();
      onSuccess();
    } catch (error) {
      Toast.error({
        content: error instanceof Error ? error.message : t('提交失败'),
      });
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal
      title={t('取消并退款')}
      visible={visible}
      width={480}
      closeOnEsc
      onCancel={onClose}
      footer={
        <>
          <Button onClick={onClose}>{t('再想想')}</Button>
          <Button
            theme="solid"
            type="primary"
            loading={submitting}
            onClick={() => void submit()}
          >
            {t('确认取消并退款')}
          </Button>
        </>
      }
    >
      <Text>
        {t(
          '该订单已支付且尚未发货。确认后订单将取消，款项原路退回；如需财务审核，审核通过后到账。已发货订单请走售后通道。',
        )}
      </Text>
      <div style={{ marginTop: 16 }}>
        <TextArea
          placeholder={t('取消原因（可选）')}
          value={reason}
          maxCount={255}
          rows={3}
          onChange={(value) => setReason(String(value ?? ''))}
        />
      </div>
    </Modal>
  );
}
