'use client';

/**
 * 积分兑换优惠券按钮（我方新增，论坛帖 517907）。
 *
 * 券型兑换品不进商品详情/购物车链：点击后确认弹窗展示券名与所需积分，
 * 确认即调 POST product/exchange/exchangeCoupon（后端单事务扣积分+发券），
 * 成功后刷新列表。未登录时后端返回未登录错误，Toast 原样提示。
 */
import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { Modal, Toast } from '@douyinfe/semi-ui';
import { apiPost } from '@/lib/client-api';
import { useTranslation } from 'react-i18next';

interface Props {
  exchangeId: number;
  couponName: string;
  /** 生效积分价（后端已按自设/平台统一比解析） */
  points: number;
  integralName: string;
  /** platform 时在确认弹窗注明按平台统一比折算 */
  integralSource?: string;
}

export default function ExchangeCouponButton({ exchangeId, couponName, points, integralName, integralSource }: Props) {
  const { t } = useTranslation();
  const router = useRouter();
  const [submitting, setSubmitting] = useState(false);

  const onExchange = () => {
    Modal.confirm({
      title: t('确认兑换'),
      content: `${t('使用')} ${points} ${integralName}${t('兑换')}「${couponName}」${integralSource === 'platform' ? `（${t('按平台统一比折算')}）` : ''}？`,
      okText: t('确认兑换'),
      cancelText: t('取消'),
      onOk: async () => {
        if (submitting) return;
        setSubmitting(true);
        try {
          await apiPost('product/exchange/exchangeCoupon', { id: exchangeId });
          Toast.success({ content: t('兑换成功，优惠券已发放到您的账户') });
          router.refresh();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('兑换失败') });
        } finally {
          setSubmitting(false);
        }
      },
    });
  };

  return (
    <p className="exchange_btn" onClick={onExchange} style={{ cursor: 'pointer' }}>
      {t('立即兑换')}
    </p>
  );
}
