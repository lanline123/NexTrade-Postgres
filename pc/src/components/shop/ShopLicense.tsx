'use client';

import { usePathname } from 'next/navigation';
import { useTranslation } from 'react-i18next';
import { useEffect, useState } from 'react';
import { apiGet } from '@/lib/client-api';

/**
 * 网店经营者营业执照公示页。
 * Vue 基准：pages/shop/license.vue（`?shopId=`，独立白页、无站点头尾、带水印）
 *
 * 接口：GET `shop/shop/detail?shopId=` → `businessLicensePic`（只认这个字段，没有就显示空态）。
 * 站点头尾的隐藏由 `layout/ChromeGate` 按路径处理。
 */
interface ShopDetail {
    shopTitle?: string;
    businessLicensePic?: string;
}

export default function ShopLicense({ shopId }: { shopId: number }) {
    const { t } = useTranslation();
    const [shop, setShop] = useState<ShopDetail | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        if (!shopId) {
            setLoading(false);
            return;
        }
        apiGet<ShopDetail>('shop/shop/detail', { shopId })
            .then((result) => setShop(result ?? null))
            .catch(() => setShop(null))
            .finally(() => setLoading(false));
    }, [shopId]);

    const licenseUrl = (shop?.businessLicensePic ?? '').trim();
    const watermark = `${shop?.shopTitle ?? ''}${t('营业执照信息公示专用')}`;

    return (
        <div className="pc-license">
            <div className="pc-license-watermark" aria-hidden="true">
                {Array.from({ length: 48 }, (_, index) => (
                    <span key={index}>{watermark}</span>
                ))}
            </div>

            <div className="pc-license-inner">
                <h1 className="pc-license-title">{t('网店经营者营业执照信息')}</h1>
                <p className="pc-license-notice">
                    <span className="pc-license-notice-icon">i</span>
                    {t('根据相关法律法规要求，网店经营相关资质信息公示如下：')}
                </p>

                <h2 className="pc-license-section">{t('证照')}</h2>

                {loading ? (
                    <div className="pc-loading">{t('加载中')}</div>
                ) : licenseUrl ? (
                    <div className="pc-license-cert">
                        <img src={licenseUrl} alt={t('营业执照')} />
                    </div>
                ) : (
                    <div className="pc-muted">{t('暂无营业执照')}</div>
                )}

                <div className="pc-license-notes">
                    <p>
                        {t(
                            '注：以上营业执照信息来源于卖家自行申报或市监部门系统数据，具体以市监部门登记为准。',
                        )}
                    </p>
                    <p>
                        {t(
                            '经营者需确保信息真实有效，平台也将定期核查，如与实际不符，为避免违规，请联系当地市监部门或平台客服更新。',
                        )}
                    </p>
                </div>
            </div>
        </div>
    );
}
