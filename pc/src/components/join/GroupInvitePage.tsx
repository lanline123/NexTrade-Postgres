'use client';

import { useEffect, useState } from 'react';
import { Button, Card, Empty, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { formatPrice } from '@/lib/format';
import { CountdownBoxes, useCountdown } from '@/components/promotion/cards';

/**
 * 拼团邀请落地页。
 * Vue 基准：pages/joining/groupByInvited.vue（961 行，`?id=<grouponPromotionRecordId>`）
 *
 * 真实接口：GET `promotion/groupon/record/details?grouponPromotionRecordId=`
 *   → { joinStatus(1 拼团中), joinStatusDesc, productName, productSn, picThumb, minGrouponPrice,
 *       productPrice, minGroupNum, joinNumCount, endTime, shopId, shopTitle, orderId,
 *       joinUsers[{ userId, username, avatar, leader }] }
 *   id 不存在时是**业务错误**（code=500「拼团记录不存在」），不是 HTTP 错误。
 *
 * 与 Vue 的差异：
 * - 门店块（收藏门店/进店逛逛）未做——`/store/info` 这个路由本仓库没有，跳过去是死链；
 * - 「去参团」带上 `grouponPromotionRecordId` 跳到商品详情，与 Vue 的
 *   `urlFormat({..., type:'invited', grouponPromotionRecordId })` 等价；
 * - 分享图（`showInviteDialog` 的二维码）未做，改在「我的拼团」里复制链接。
 */
interface JoinUser {
    userId?: number;
    username?: string;
    avatar?: string;
    leader?: boolean;
}

interface JoiningDetail {
    joinStatus?: number;
    joinStatusDesc?: string;
    productId?: number;
    productName?: string;
    productSn?: string;
    picThumb?: string;
    picUrl?: string;
    minGrouponPrice?: string | number;
    productPrice?: string | number;
    minGroupNum?: number;
    joinNumCount?: number;
    endTime?: string;
    shopId?: number;
    shopTitle?: string;
    orderId?: number;
    grouponPromotionRecordId?: number;
    joinUsers?: JoinUser[];
}

const PLACEHOLDER = '待加入';

export default function GroupInvitePage({ recordId }: { recordId: number }) {
    const { t } = useTranslation();
    const [detail, setDetail] = useState<JoiningDetail | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');
    const cd = useCountdown(detail?.endTime ?? 0);

    useEffect(() => {
        if (!recordId) {
            setError(t('缺少拼团参数'));
            setLoading(false);
            return;
        }
        apiGet<JoiningDetail>('promotion/groupon/record/details', { grouponPromotionRecordId: recordId })
            .then((result) => setDetail(result))
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, [recordId, t]);

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('拼团不存在或已结束')} description={error} />;
    if (!detail) return <Empty title={t('拼团不存在或已结束')} />;

    const minGroupNum = Number(detail.minGroupNum ?? 0);
    const joined = Number(detail.joinNumCount ?? 0);
    const need = Math.max(0, minGroupNum - joined);
    const ongoing = detail.joinStatus === 1;

    // 团长排最前；人数不够补一个「待加入」占位（Vue 同口径）
    const users: JoinUser[] = [...(detail.joinUsers ?? [])].sort((a, b) =>
        a.leader ? -1 : b.leader ? 1 : 0,
    );
    if (joined < minGroupNum) {
        users.push({ userId: 0, username: PLACEHOLDER, avatar: '/assets/avatar/default.gif' });
    }

    const joinHref = `/product/${detail.productSn ?? detail.productId ?? ''}?grouponPromotionRecordId=${recordId}&type=invited`;

    return (
        <div className="pc-group-invite">
            <Card>
                <div className="pc-group-invite-product">
                    <img src={detail.picThumb || detail.picUrl || '/assets/avatar/default.gif'} alt={detail.productName ?? ''} />
                    <div className="pc-group-invite-info">
                        <Typography.Title heading={4}>{detail.productName}</Typography.Title>
                        <div className="pc-group-invite-tags">
                            {minGroupNum > 0 ? (
                                <Tag size="small" color="red">
                                    {minGroupNum}
                                    {t('人团')}
                                </Tag>
                            ) : null}
                            <span className="pc-muted">
                                {t('已有')}
                                {joined}
                                {t('人参团')}
                            </span>
                        </div>
                        <div className="pc-promo-prices">
                            <span className="pc-promo-price">{formatPrice(detail.minGrouponPrice)}</span>
                            <span className="pc-promo-origin">{formatPrice(detail.productPrice)}</span>
                        </div>
                    </div>
                </div>
            </Card>

            <Card>
                <div className="pc-group-invite-status">
                    <div>
                        <span className="pc-group-invite-desc">{detail.joinStatusDesc || ''}</span>
                        {ongoing ? (
                            <span className="pc-muted">
                                {detail.joinStatusDesc ? '，' : ''}
                                {t('还差')} <b className="pc-promo-price-inline">{need}</b> {t('人成团')}
                            </span>
                        ) : null}
                    </div>
                    {ongoing && detail.endTime ? (
                        <div className="pc-promo-foot">
                            {t('距结束')} <CountdownBoxes cd={cd} endedText={t('已结束')} />
                        </div>
                    ) : null}
                </div>

                {users.length > 0 ? (
                    <div className="pc-group-invite-users">
                        {users.map((user, index) => (
                            <div className="pc-group-invite-user" key={user.userId ?? index}>
                                <div className="pc-group-invite-avatar">
                                    <img src={user.avatar || '/assets/avatar/default.gif'} alt={user.username ?? ''} />
                                    {user.leader ? <span className="pc-group-invite-leader">{t('团长')}</span> : null}
                                </div>
                                <div className="pc-muted pc-ellipsis">
                                    {user.username === PLACEHOLDER ? t('待加入') : user.username}
                                </div>
                            </div>
                        ))}
                    </div>
                ) : (
                    <div className="pc-muted">{t('暂无拼团数据')}</div>
                )}
            </Card>

            <Card title={t('拼团流程')}>
                <div className="pc-group-steps">
                    {[t('1.开团/参加'), t('2.邀友参团'), t('3.人满成团')].map((step, index) => (
                        <div className="pc-group-step" key={step}>
                            <span className="pc-group-step-index">{index + 1}</span>
                            <span>{step.replace(/^\d+\./, '')}</span>
                        </div>
                    ))}
                </div>
            </Card>

            <div className="pc-group-invite-bar">
                <a href="/member/joining/list">{t('我的拼团')}</a>
                <div className="pc-group-invite-bar-right">
                    <a href={`/product/${detail.productSn ?? detail.productId ?? ''}`}>
                        <Button theme="borderless">{t('单独购买')}</Button>
                    </a>
                    <Button
                        theme="solid"
                        type="primary"
                        disabled={!ongoing}
                        onClick={() => {
                            if (!localStorage.getItem('accessToken') && !localStorage.getItem('token')) {
                                Toast.info(t('请先登录'));
                                window.location.href = `/member/login?returnUrl=${encodeURIComponent(joinHref)}`;
                                return;
                            }
                            window.location.href = joinHref;
                        }}
                    >
                        {ongoing ? t('去参团') : t('拼团已结束')}
                    </Button>
                </div>
            </div>
        </div>
    );
}
