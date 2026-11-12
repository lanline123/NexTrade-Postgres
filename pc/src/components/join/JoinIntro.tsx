'use client';

import Link from 'next/link';
import { Button, Card, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';

/**
 * 入驻介绍页。
 * Vue 基准：pages/join/index.vue、intro.vue（1360 行协议长文）。
 * 当前只做流程说明与入口；协议正文与资质表单（merchant/merchant/apply）留到下一步。
 */
const STEPS = ['提交入驻申请', '平台资质审核', '签署入驻协议', '店铺上线经营'];

export default function JoinIntro() {
    const { t } = useTranslation();
    return (
        <div className="pc-catalog">
            <Typography.Title heading={2}>{t('商家入驻')}</Typography.Title>
            <Typography.Paragraph type="tertiary">
                {t('提交资料 → 平台审核 → 签署协议 → 开店经营，全流程线上完成。')}
            </Typography.Paragraph>

            <div className="pc-stat-grid">
                {STEPS.map((step, index) => (
                    <Card key={step} className="pc-stat-card">
                        <div className="pc-stat-value">{index + 1}</div>
                        <div className="pc-muted">{t(step)}</div>
                    </Card>
                ))}
            </div>

            <Card title={t('入驻须知')} style={{ marginTop: 16 }}>
                <ul className="pc-join-list">
                    <li>{t('需具备合法有效的营业执照与经营资质。')}</li>
                    <li>{t('提交后 1-3 个工作日内完成初审，结果在「入驻申请」中查看。')}</li>
                    <li>{t('审核通过后需签署平台入驻协议，方可上架商品。')}</li>
                </ul>
                <div className="pc-order-actions" style={{ marginTop: 12 }}>
                    <Link href="/joining/list">
                        <Button theme="solid" type="primary">{t('查看我的申请')}</Button>
                    </Link>
                    <Button theme="light" disabled>
                        {t('在线提交资料（待接入）')}
                    </Button>
                </div>
            </Card>
        </div>
    );
}
