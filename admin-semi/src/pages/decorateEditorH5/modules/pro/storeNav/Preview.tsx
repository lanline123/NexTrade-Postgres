/**
 * 门店导航 预览（对齐 view/admin mobilePro/storeNav/Preview.vue）
 * - 门店数据无真实来源（编辑器预览口径与原版一致）：自动获取按 storeNum（钳制 1~20）铺演示门店，
 *   指定门店按 storeList 顺序渲染（空时 1 家演示门店兜底）；
 * - 样式一：标题行（文案固定「附近门店」，字号/颜色/粗细取 subTitle）+ 门店列表
 *   （评分行/固定演示地址/距离胶囊均为原版写死演示值）；
 * - 样式二：门店卡（关注钮/评分/起送价/配送费/距离按开关）+ 演示商品横滑行；
 * - 评分标签图 store-tag.png 为原版 admin 本地资源，本工程无此资源文件，预览中略去该图，
 *   仅保留「5.00」评分文案（已在小结注明）；导航图标以文案承接。
 */
import "./Preview.css";
import { useMemo } from "react";
import { mergeDefaultModule, cloneDeep, formatCommonStyle } from "../../../contract/sharedDefaults";
import { cssText, imageFormat } from "../../../shared/format";
import { storeNavDefaults } from "./index";

interface Props {
    data: Record<string, any>;
}

const DEMO_STORE_TITLE = "门店名称门店名称";
const DEMO_STORE_LOGO = "https://oss.tigshop.com/img/gallery/202508/1756284426i6dOj6OQOWT7PEgTsu.jpeg";
const DEMO_PRODUCTS = [
    { productName: "婴儿连体衣 会员特价", productPrice: 19.8, picThumb: "https://oss.tigshop.com/img/gallery/202411/17319190474y0XWFxO8GJZF0dg64.jpg" },
    { productName: "纯棉新生儿套装", productPrice: 12.9, picThumb: "https://oss.tigshop.com/img/gallery/202411/1731919071axvkfUDkQNFbIzF1iY.jpg" },
    { productName: "可折叠婴儿浴盆", productPrice: 39.9, picThumb: "https://oss.tigshop.com/img/gallery/202411/1731919071lOa0ZBx2mYlvq1lgjr.jpg" },
    { productName: "婴儿洗护礼盒 热卖中", productPrice: 12.9, picThumb: "https://oss.tigshop.com/img/gallery/202411/1731919071xUnObq2LokKPFecwXq.jpg" }
];

export default function StoreNavPreview({ data }: Props) {
    const m = useMemo(() => { const merged = JSON.parse(JSON.stringify(data ?? {})); mergeDefaultModule(merged, cloneDeep(storeNavDefaults)); return merged; }, [data]);

    const previewStores = useMemo(() => {
        if (Number(m.storeSource) === 2) {
            const assigned: Array<Record<string, any>> = Array.isArray(m.storeList) ? m.storeList : [];
            if (assigned.length > 0) {
                return assigned.map((item) => ({ shopTitle: item.shopTitle || DEMO_STORE_TITLE, shopLogo: item.shopLogo || DEMO_STORE_LOGO }));
            }
            return [{ shopTitle: DEMO_STORE_TITLE, shopLogo: DEMO_STORE_LOGO }];
        }
        const num = Math.min(Math.max(Math.round(Number(m.storeNum) || 1), 1), 20);
        return Array.from({ length: num }, () => ({ shopTitle: DEMO_STORE_TITLE, shopLogo: DEMO_STORE_LOGO }));
    }, [m.storeSource, m.storeList, m.storeNum]);

    const common = formatCommonStyle(m.moduleStyle ?? {}, m.contentStyle ?? {}, m.subTitle ?? {});
    const outerStyle = cssText(common.moduleStyle.boxPaddingTopBottom + common.moduleStyle.boxPadding + common.moduleStyle.backgroundColor);
    const contentStyle = cssText(
        common.moduleContentStyle.backgroundColor +
            common.moduleContentStyle.backgroundImage +
            common.moduleContentStyle.backgroundSize +
            common.moduleContentStyle.boxRadius2 +
            common.moduleContentStyle.boxShadow +
            common.moduleContentStyle.paddingTopBottom +
            common.moduleContentStyle.paddingLeftRight
    );
    const titleStyle = cssText(`font-size: ${m.subTitle?.titleFontSize}px; color: ${m.subTitle?.titleColor}; font-weight: ${m.subTitle?.titleFontBold};`);
    const sts = m.storeTitleStyle ?? storeNavDefaults.storeTitleStyle;
    const storeTitleStyle: React.CSSProperties = {
        fontSize: `${sts.titleFontSize ?? 14}px`,
        color: sts.titleColor || "#1a1a1a",
        fontWeight: sts.titleFontBold ?? 700
    };
    const boxBase = (index: number): React.CSSProperties => ({
        backgroundColor: m.storeBackgroundColor || "#ffffff",
        padding: `${m.storePaddingY ?? 0}px ${m.storePaddingX ?? 0}px`,
        borderRadius: `${m.storeRadius ?? 0}px`,
        overflow: "hidden",
        ...(index > 0 ? { marginTop: `${m.storeGap ?? 15}px` } : {})
    });

    return (
        <div className="stn-root" style={outerStyle}>
            <div style={contentStyle}>
                {Number(m.storeStyle) !== 2 ? (
                    <div className="stn-store-nav-box">
                        <div className="stn-store-header" style={{ padding: `${m.titlePaddingY ?? 0}px ${m.titlePaddingX ?? 0}px` }}>
                            <div className="stn-header-title" style={titleStyle}>
                                附近门店
                            </div>
                            <div className="stn-header-more">
                                更多<span aria-hidden="true">›</span>
                            </div>
                        </div>
                        <div>
                            {previewStores.map((store, index) => (
                                <div className="stn-stores-item" key={index} style={boxBase(index)}>
                                    <div className="stn-item-left">
                                        <img src={imageFormat(store.shopLogo || DEMO_STORE_LOGO)} alt="" />
                                    </div>
                                    <div className="stn-item-right">
                                        <div style={storeTitleStyle}>{store.shopTitle}</div>
                                        {Number(m.showRating) === 1 ? <div className="stn-store-desc">5.00</div> : null}
                                        <div className="stn-store-address-info">
                                            <div className="stn-address stn-line1">江西省南昌市青山湖区艾溪湖312123312</div>
                                            <div className="stn-distance">
                                                <span aria-hidden="true">➤</span> 520m
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                ) : (
                    <div className="stn-store-card-box">
                        {previewStores.map((store, index) => (
                            <div className="stn-store-card" key={index} style={boxBase(index)}>
                                <div className="stn-card-header">
                                    <div className="stn-shop-logo">
                                        <img src={imageFormat(store.shopLogo || DEMO_STORE_LOGO)} alt="" />
                                    </div>
                                    <div className="stn-shop-main">
                                        <div className="stn-title-row">
                                            <div className="stn-shop-title stn-line1" style={storeTitleStyle}>
                                                {store.shopTitle}
                                            </div>
                                            {Number(m.showFollow) === 1 ? (
                                                <div className="stn-collect-btn">
                                                    <span className="stn-collect-text">关注</span>
                                                </div>
                                            ) : null}
                                        </div>
                                        <div className="stn-meta-row">
                                            {Number(m.showRating) === 1 ? <span className="stn-shop-star">5.00分</span> : null}
                                            {Number(m.showMinOrder) === 1 ? <span className="stn-meta-item">起送 ¥1</span> : null}
                                            {Number(m.showDeliveryFee) === 1 ? <span className="stn-meta-item">配送费 ¥1</span> : null}
                                            {Number(m.showDistance) === 1 ? <span className="stn-meta-item">距离 556.88km</span> : null}
                                        </div>
                                    </div>
                                </div>
                                <div className="stn-product-scroll">
                                    <div className="stn-product-row" style={{ columnGap: `${m.productGap ?? 8}px` }}>
                                        {DEMO_PRODUCTS.map((product) => (
                                            <div className="stn-product-item" key={product.productName}>
                                                <div className="stn-product-image">
                                                    <img src={product.picThumb} alt="" />
                                                </div>
                                                {Number(m.showProductName) === 1 ? (
                                                    <div className={`stn-product-name ${Number(m.productNameRow) === 2 ? "stn-line2" : "stn-line1"}`}>{product.productName}</div>
                                                ) : null}
                                                {Number(m.showProductPrice) === 1 ? <div className="stn-product-price">¥{product.productPrice}</div> : null}
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}
