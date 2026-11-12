'use client';

import { useEffect, useState } from 'react';
import { Button, Card, Empty, Spin, Tag } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { ProductBuyPanel, ProductDescSection, ProductGallery, ProductSpecTable } from './parts';
import { fetchProductDetailStyle, useProductDetail, type ProductUiVersion } from './useProductDetail';

/**
 * 商品详情壳：一份取数，三套布局。
 * Vue 基准：pages/item/src/Product.vue（约 1500 行，内含 container-new / container-overseas / 老版三段模板）
 * 与两条入口路由的切换规则：
 * - `/product/[...slug]`：Vue 写死 `ui-version="overseas"`（SEO 路径恒为跨境版）；
 * - `/item/:id`：读后台「商品页风格」`common/config/productDetailStyle`（1=old 2=new 3=overseas）。
 *
 * 两套入口都支持 `?uiStyle=old|new|overseas` 强制预览（Vue 只在 dev 放开，这里一直放开便于验收）。
 *
 * 三套布局的结构差异（对照 Vue）：
 * | 版本 | 头部 | 独有区块 |
 * |---|---|---|
 * | overseas | CommonPageHeader | 分类导航面包屑 + 「分享至」社交行 + 店铺信息条 + 吸顶锚点 Tab |
 * | new | PcHeaderTopBarRuntime | ProductHead 店铺横幅 + 店铺信息卡 |
 * | old | CommonPageHeader | 商品编号/收藏工具行 + 左侧竞品栏 + 下方左右分栏 |
 */

export default function ProductShell({
    idOrSn,
    forceVersion,
    invited,
    grouponRecordId,
}: {
    idOrSn: string;
    /** 传入则不读后台配置（/product 写死 overseas 的口径） */
    forceVersion?: ProductUiVersion;
    invited?: boolean;
    grouponRecordId?: number;
}) {
    const { t } = useTranslation();
    const { detail, loading, error, skuId, setSkuId, skus, activeSku, price, stock } = useProductDetail(
        idOrSn,
        grouponRecordId,
    );
    const [version, setVersion] = useState<ProductUiVersion | null>(forceVersion ?? null);

    useEffect(() => {
        if (forceVersion) {
            setVersion(forceVersion);
            return;
        }
        fetchProductDetailStyle()
            .then(setVersion)
            .catch(() => setVersion('overseas'));
    }, [forceVersion]);

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('商品加载失败')} description={error} />;
    if (!detail?.item) return <Empty title={t('商品不存在')} />;

    const data = { detail, skus, skuId, setSkuId, price, stock, invited };

    return (
        <Spin spinning={false}>
            <StyleSwitchPreview version={version} onChange={setVersion} />
            {version === 'new' ? (
                <ProductLayoutNew {...data} detail={detail} />
            ) : version === 'old' ? (
                <ProductLayoutOld {...data} detail={detail} />
            ) : (
                <ProductLayoutOverseas {...data} detail={detail} />
            )}
        </Spin>
    );
}

/** 预览切换条：只在带 ?uiStyle= 时渲染，便于逐套验收。 */
function StyleSwitchPreview({
    version,
    onChange,
}: {
    version: ProductUiVersion | null;
    onChange: (v: ProductUiVersion) => void;
}) {
    const { t } = useTranslation();
    if (!version) return null;
    return (
        <div className="pc-pd-style-switch">
            <Tag size="small" color="blue">
                {t('UI 预览')}
            </Tag>
            {(['old', 'new', 'overseas'] as const).map((item) => (
                <Button
                    key={item}
                    size="small"
                    theme={version === item ? 'solid' : 'borderless'}
                    onClick={() => onChange(item)}
                >
                    {item === 'old' ? t('样式一·经典') : item === 'new' ? t('样式二·新版') : t('样式三·跨境')}
                </Button>
            ))}
        </div>
    );
}

type LayoutProps = {
    detail: NonNullable<ReturnType<typeof useProductDetail>['detail']>;
    skus: NonNullable<ReturnType<typeof useProductDetail>['skus']>;
    skuId: number;
    setSkuId: (id: number) => void;
    price?: number | string;
    stock?: number;
    invited?: boolean;
};

/** 样式三·跨境版（默认，/product 入口）：分类导航 + 分享行 + 店铺条 + 吸顶锚点。 */
function ProductLayoutOverseas(props: LayoutProps) {
    const { t } = useTranslation();
    const { detail } = props;
    const item = detail.item;
    const shopId = 0; // 详情接口未带店铺信息时按自营展示（Vue 里 shopId=0 隐藏店铺条）

    return (
        <div className="pc-pd pc-pd-overseas">
            <div className="pc-pd-breadcrumb">
                <a href="/list">{t('首页')}</a>
                <span>/</span>
                <a href={`/list?cat=${item?.categoryId ?? ''}`}>{t('商品分类')}</a>
                <span>/</span>
                <span className="pc-ellipsis">{item?.productName}</span>
            </div>

            <div className="pc-pd-share-row">
                <span className="pc-muted">{t('分享至')}</span>
                <ShareButtons />
            </div>

            {shopId !== 0 ? <div className="pc-pd-shop-bar">{t('店铺信息')}</div> : null}

            <Card>
                <div className="pc-pd-top">
                    <ProductGallery detail={detail} skus={props.skus} />
                    <ProductBuyPanel {...props} />
                </div>
            </Card>

            <div className="pc-pd-anchor">
                {[t('商品详情'), t('规格参数'), t('购买须知')].map((label, index) => (
                    <a key={label} href={`#pc-pd-sec-${index}`}>
                        {label}
                    </a>
                ))}
            </div>

            <Card>
                <section id="pc-pd-sec-0">
                    <div className="pc-pd-sec-title">{t('商品详情')}</div>
                    <ProductDescSection detail={detail} />
                </section>
                <section id="pc-pd-sec-1">
                    <div className="pc-pd-sec-title">{t('规格参数')}</div>
                    <ProductSpecTable detail={detail} />
                </section>
                <section id="pc-pd-sec-2">
                    <div className="pc-pd-sec-title">{t('购买须知')}</div>
                    <div className="pc-muted">
                        {t('所有发票由商品供货商开具；使用礼品卡支付部分的金额，不支持开发票。')}
                    </div>
                </section>
            </Card>
        </div>
    );
}

/** 样式二·新版：店铺横幅 + 店铺信息卡 + 图文同区。 */
function ProductLayoutNew(props: LayoutProps) {
    const { t } = useTranslation();
    const { detail } = props;

    return (
        <div className="pc-pd pc-pd-new">
            <Card className="pc-pd-shop-banner">
                <div className="pc-pd-shop-banner-inner">
                    <div>
                        <div className="pc-pd-shop-name">{t('官方自营旗舰店')}</div>
                        <div className="pc-muted">{t('品牌创立于 2020 年 · 专精特新中小企业')}</div>
                    </div>
                    <a href="/shop/list">
                        <Button size="small" theme="light">
                            {t('进店逛逛')}
                        </Button>
                    </a>
                </div>
            </Card>

            <Card>
                <div className="pc-pd-top">
                    <ProductGallery detail={detail} skus={props.skus} />
                    <ProductBuyPanel {...props} />
                </div>
            </Card>

            <Card>
                <div className="pc-pd-sec-title">{t('商品详情')}</div>
                <ProductDescSection detail={detail} />
                <div className="pc-pd-sec-title" style={{ marginTop: 20 }}>
                    {t('规格参数')}
                </div>
                <ProductSpecTable detail={detail} />
            </Card>
        </div>
    );
}

/** 样式一·经典：编号/收藏工具行 + 下方左右分栏（左竞品栏占位，右 Tab）。 */
function ProductLayoutOld(props: LayoutProps) {
    const { t } = useTranslation();
    const { detail } = props;
    const [tab, setTab] = useState(0);
    const tabs = [t('商品详情'), t('规格参数'), t('用户评价')];

    return (
        <div className="pc-pd pc-pd-old">
            <Card>
                <div className="pc-pd-top">
                    <ProductGallery detail={detail} skus={props.skus} />
                    <ProductBuyPanel {...props} />
                </div>
                <div className="pc-pd-tools-row">
                    <span className="pc-muted">
                        {t('商品编号')}：{detail.item?.productSn ?? '-'}
                    </span>
                    <span className="pc-muted">{t('收藏 / 原图下载等工具栏占位')}</span>
                </div>
            </Card>

            <div className="pc-pd-old-body">
                <aside className="pc-pd-side">
                    <Card title={t('同类推荐')}>
                        <div className="pc-muted">{t('竞品/同类商品栏位占位（接口 product/productStyle 体系外，待接）')}</div>
                    </Card>
                </aside>
                <Card className="pc-pd-old-tabs">
                    <div className="pc-tabs">
                        {tabs.map((label, index) => (
                            <button
                                key={label}
                                type="button"
                                className={tab === index ? 'is-active' : ''}
                                onClick={() => setTab(index)}
                            >
                                {label}
                            </button>
                        ))}
                    </div>
                    <div style={{ marginTop: 14 }}>
                        {tab === 0 ? <ProductDescSection detail={detail} /> : null}
                        {tab === 1 ? <ProductSpecTable detail={detail} /> : null}
                        {tab === 2 ? <div className="pc-muted">{t('评价列表待接（评论接口另批迁移）')}</div> : null}
                    </div>
                </Card>
            </div>
        </div>
    );
}

/** 社交分享（跨境版独有）：直链各平台分享页，不做弹窗。 */
function ShareButtons() {
    const { t } = useTranslation();
    if (typeof window === 'undefined') return null;
    const url = encodeURIComponent(window.location.href);
    const links: Array<[string, string]> = [
        ['WhatsApp', `https://wa.me/?text=${url}`],
        ['Facebook', `https://www.facebook.com/sharer/sharer.php?u=${url}`],
        ['X', `https://twitter.com/intent/tweet?url=${url}`],
        ['Email', `mailto:?body=${url}`],
    ];
    return (
        <div className="pc-pd-share-links">
            {links.map(([label, href]) => (
                <a key={label} href={href} target="_blank" rel="noreferrer">
                    {label}
                </a>
            ))}
            <span className="pc-muted" style={{ fontSize: 12 }}>
                {t('原图下载按钮待接')}
            </span>
        </div>
    );
}
