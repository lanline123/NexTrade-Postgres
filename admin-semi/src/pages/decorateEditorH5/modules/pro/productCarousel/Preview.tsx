/**
 * 商品轮播 预览（移植自 view/admin editor/modules/mobilePro/productCarousel/Preview.vue）
 *
 * 居中露邻轮播：卡宽 cardWidthPercent%，激活卡 scale 1/opacity 1、其余 scale .8/opacity .8。
 * 原版 Swiper（slidesPerView auto + centeredSlides）在编辑器内以横滑轨 + 点击切换
 * 激活卡承接，autoplay 开启时每 3s 自动推进（原版 delay 3000）。
 * 取数口径同原版 buildDisplay：productList 快照为底，按 productId>0 拉最新商品信息覆盖
 * 名称/价格/图/销量；productList 为空时回退共享演示项。
 * buyBtnStyle 非对象（历史数字值）时回退默认对象；购买钮用共享 BuyButton。
 */
import { useEffect, useMemo, useState } from "react";
import { cssText, imageFormat, productImage, priceFormat } from "../../../shared/format";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { mergeDefaultModule, cloneDeep, defaultProductCarouselDemoItems } from "../../../contract/sharedDefaults";
import BuyButton from "../../../shared/BuyButton";
import { productCarouselDefaults } from "./index";
import "./Preview.css";

const ratioPaddingMap: Record<string, string> = { default: "", "1:1": "100%", "3:4": "133.333%", "3:2": "66.667%" };

export default function ProductCarouselPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, productCarouselDefaults);
        if (!merged.buyBtnStyle || typeof merged.buyBtnStyle !== "object") {
            merged.buyBtnStyle = cloneDeep(productCarouselDefaults.buyBtnStyle);
        }
        return merged;
    }, [data]);

    const snapshots: Array<Record<string, any>> =
        Array.isArray(module.productList) && module.productList.length ? module.productList : cloneDeep(defaultProductCarouselDemoItems);
    const realIds = snapshots.map((i) => i.productId).filter((id) => Number(id) > 0);
    const fetched = useDecorateProductList({ productSelectType: realIds.length ? 1 : 0, productIds: realIds });

    const displayList = useMemo(() => {
        const fresh = new Map<string, any>();
        fetched.forEach((r: any) => fresh.set(String(r.productId), r));
        return snapshots.map((it) => {
            const f = Number(it.productId) > 0 ? fresh.get(String(it.productId)) : null;
            if (!f) return it;
            return {
                ...it,
                productName: f.productName ?? it.productName,
                productDesc: f.productDesc ?? it.productDesc,
                productPrice: f.productPrice ?? it.productPrice,
                marketPrice: f.marketPrice ?? it.marketPrice,
                salesVolume: f.salesVolume ?? it.salesVolume,
                picUrl: f.picUrl ?? it.picUrl,
                picThumb: f.picThumb ?? it.picThumb
            };
        });
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [JSON.stringify(snapshots), fetched]);

    const [active, setActive] = useState(0);
    useEffect(() => {
        if (!Number(module.autoplay)) return;
        const timer = window.setInterval(() => {
            setActive((cur) => (displayList.length ? (cur + 1) % displayList.length : 0));
        }, 3000);
        return () => window.clearInterval(timer);
    }, [module.autoplay, displayList.length]);

    const f = module.frame ?? {};
    const outerParts: Array<string | false> = [
        `padding-left:${f.boxPadding ?? 0}px;`,
        `padding-right:${f.boxPadding ?? 0}px;`,
        `padding-top:${f.boxPaddingTop ?? 0}px;`,
        `padding-bottom:${f.boxPaddingBottom ?? 0}px;`,
        f.backgroundColor ? `background-color:${f.backgroundColor};` : ""
    ];
    const bgUrl = module.moduleBgImage?.picUrl;
    if (bgUrl) {
        const mode = module.bgImageMode || "normal";
        if (mode === "repeat") outerParts.push(`background-image:url(${imageFormat(bgUrl)});background-repeat:repeat;background-position:top center;`);
        else if (mode === "fixed") outerParts.push(`background-image:url(${imageFormat(bgUrl)});background-size:cover;background-position:center;background-repeat:no-repeat;background-attachment:fixed;`);
        else outerParts.push(`background-image:url(${imageFormat(bgUrl)});background-size:cover;background-position:center;background-repeat:no-repeat;`);
    }
    const outerStyle = cssText(...outerParts);

    const titleStyle = cssText(
        `color:${module.moduleTitleColor || "#fff"};`,
        `margin-bottom:${module.titleContentGap ?? 20}px;`,
        `text-align:${module.textAlign || "left"};`
    );

    const itemBase = (() => {
        const r = module.goodsRadius ?? 12;
        const pad = module.innerPadding ?? 10;
        let base = "";
        switch (module.cardStyle) {
            case "noBorderWhite":
                base = "background-color:#fff;";
                break;
            case "shadow":
                base = "background-color:#fff;box-shadow:0 2px 4px rgba(0,0,0,0.06);";
                break;
            case "border":
                base = "background-color:#fff;border:1px solid rgba(0,0,0,0.08);";
                break;
            default:
                base = "background-color:transparent;";
        }
        return `${base}border-radius:${r}px;padding:${pad}px;`;
    })();

    const ratio = ratioPaddingMap[module.productPicRatio || "1:1"] ?? "";
    const imgBoxStyle = cssText(
        (() => {
            const r = module.goodsRadius ?? 12;
            const radiusCss = module.picBottomRadius ? `border-radius:${r}px;` : `border-radius:${r}px ${r}px 0 0;`;
            return ratio ? `${radiusCss}padding-top:${ratio};height:0;` : radiusCss;
        })()
    );
    const imgStyle = cssText(ratio ? "position:absolute;inset:0;width:100%;height:100%;object-fit:cover;display:block;" : "width:100%;height:auto;display:block;");
    const infoStyle = cssText(`text-align:${module.textAlign || "left"};`);
    const nameStyle = cssText(`color:${module.productTitleColor || "#666"};`, `font-weight:${module.textWeight || 400};`);
    const descStyle = cssText(`color:${module.descColor || "#999"};`);
    const salesStyle = cssText(`color:${module.salesColor || "#999"};`);
    const priceStyle = cssText(`color:${module.priceColor || "#fd70a0"};`);
    const getSales = (item: any) => Number(item.salesVolume ?? item.virtualSales ?? 0);

    return (
        <div className="module-ad-con module-product_carousel" style={outerStyle}>
            {Number(module.showModuleTitle) === 1 && module.moduleTitle ? (
                <div className="pcr-module-title" style={titleStyle}>{module.moduleTitle}</div>
            ) : null}
            <div className="pcr-wrap">
                {!displayList || displayList.length === 0 ? (
                    <div className="pcr-empty"><span>请先选择商品</span></div>
                ) : (
                    <div className="pcr-track" style={cssText(`gap:${module.itemGap ?? 3}px;`)}>
                        {displayList.map((item: any, idx: number) => (
                            <div
                                key={`${item.productId || 0}_${idx}`}
                                className="pcr-slide"
                                style={cssText(`width:${module.cardWidthPercent || 82}%;`)}
                                onClick={() => setActive(idx)}
                            >
                                <div
                                    className="pcr-item"
                                    style={cssText(
                                        itemBase,
                                        idx === active
                                            ? "transform:scale3d(1,1,1);opacity:1;"
                                            : "transform:scale3d(0.8,0.8,1);opacity:0.8;"
                                    )}
                                >
                                    <div className="pcr-photo">
                                        <div className="pcr-img-box" style={imgBoxStyle}>
                                            <img src={productImage(item)} alt="" style={imgStyle} />
                                        </div>
                                    </div>
                                    <div className="pcr-info" style={infoStyle}>
                                        {Number(module.showName) === 1 ? (
                                            <div className="pcr-name" style={nameStyle}>{item.productName}</div>
                                        ) : null}
                                        {Number(module.showDesc) === 1 && item.productDesc ? (
                                            <div className="pcr-desc" style={descStyle}>{item.productDesc}</div>
                                        ) : null}
                                        {Number(module.showSales) === 1 ? (
                                            <div className="pcr-sales" style={salesStyle}>已售{getSales(item)}</div>
                                        ) : null}
                                        <div className="pcr-bottom">
                                            {Number(module.showPrice) === 1 ? (
                                                <div className="pcr-prices">
                                                    <span className="pcr-price-current" style={priceStyle}>
                                                        <i>¥</i><b>{priceFormat(Number(item.productPrice))}</b>
                                                    </span>
                                                    {Number(module.showMarketPrice) === 1 && Number(item.marketPrice) > Number(item.productPrice) ? (
                                                        <span className="pcr-price-market" style={{ color: module.marketPriceColor }}>
                                                            ¥{priceFormat(Number(item.marketPrice))}
                                                        </span>
                                                    ) : null}
                                                </div>
                                            ) : (
                                                <div />
                                            )}
                                            {module.buyBtnStyle?.showBtn ? <BuyButton styleOptions={module.buyBtnStyle} /> : null}
                                        </div>
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
