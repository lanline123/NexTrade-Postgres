/**
 * 商品 预览（移植自 view/admin editor/modules/mobile/product/Preview.vue）
 *
 * 类名与 Vue 版逐一致（module-goods_ad / goods-ad-warp / goods-ad-item），
 * 样式由移植的全局 module.css 提供；商品列表按来源实时拉取。
 * 样式 7（轮播滑动）按每页 3 个切片轮播，指示器颜色可配。
 */
import { useMemo, useState } from "react";
import { Swiper, SwiperItem } from "@nutui/nutui-react";
import CommonTitle from "../../../shared/CommonTitle";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { productDefaults } from "./index";

import { useI18n } from '../../../../../i18n';
import "./Preview.css";

/**
 * 旧版数字 buyBtnStyle（与 Vue 版 lyecs-buyBtn 模板逐一致）：
 * - 5/6 为纯文字 span「购买」、7/8 为纯文字 span「马上抢」，均无图标样式（不得套 .iconh5-style）；
 * - 1~4 为 .iconh5-style 图标 span（color #f23030 / font-size 18px / margin-right 5px，见 Preview.css），
 *   字形分别为 icon-gouwuche / icon-gouwuche1 / icon-jia / icon-jia1 四个不同字形；
 *   本实现以语义对应的内联 SVG 承接：购物车 / 购物车变体（篮筐） / 加号（方） / 加号变体（圆）。
 */
function LegacyBuyButton({ style }: { style: number }) {
    const cartPath = "M401.3 829.9c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m400.4 0c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m211.9-622.1c-11-12.7-27.2-19.9-44.1-19.9H238.9l-17.4-96.8C217.4 55.4 185.5 29 149.5 29H63.4C37.8 29 17 49.8 17 75.4s20.8 46.4 46.4 46.4h86.1c3.2 0 5.9 2.3 6.5 5.4l76.7 425.7c8.5 47.6 49.8 82.1 98.2 82.1h438.3c46.4 0 86.9-33 96.2-78.5l54.7-270.4c3.4-18.1-1.3-36.6-13.5-50.3z";
    const cartVariantPath = "M896 149.3H725.3l-27.7-55.5c-8.1-16.2-24.6-26.5-42.7-26.5H369.1c-18.1 0-34.6 10.2-42.7 26.5l-27.7 55.5H128c-35.3 0-64 28.7-64 64v618.7c0 35.3 28.7 64 64 64h768c35.3 0 64-28.7 64-64V213.3c0-35.3-28.7-64-64-64z m-64 576H192V320h640v405.3z";
    const plusPath = "M448 128h128v320h320v128H576v320H448V576H128V448h320V128z";
    const plusVariantPath = "M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64z m213.3 480H544v181.3c0 17.6-14.4 32-32 32s-32-14.4-32-32V544H298.7c-17.6 0-32-14.4-32-32s14.4-32 32-32H480V298.7c0-17.6 14.4-32 32-32s32 14.4 32 32V480h181.3c17.6 0 32 14.4 32 32s-14.4 32-32 32z";
    const iconSpan = (path: string) => (
        <span className="iconh5-style">
            <svg viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor" style={{ display: "block" }}>
                <path d={path} />
            </svg>
        </span>
    );
    const render = () => {
        if (style === 5 || style === 6) return <span>购买</span>;
        if (style === 7 || style === 8) return <span>马上抢</span>;
        if (style === 1) return iconSpan(cartPath);
        if (style === 2) return iconSpan(cartVariantPath);
        if (style === 3) return iconSpan(plusPath);
        if (style === 4) return iconSpan(plusVariantPath);
        return null;
    };
    return <a className="buy-btn lyecs-buyBtn">{render()}</a>;
}
export default function ProductPreview({ data }: { data: Record<string, any> }) {
    const { t } = useI18n();
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, productDefaults);
        return merged;
    }, [data]);

    const productList = useDecorateProductList(module.products);
    const frameFormat = formatFrame(module.frame ?? {});
    // 与 Vue 版 allFormat 一致：模块级样式串（backgroundColor 为空时不输出，避免覆盖 frame 背景）
    const allFormat = {
        backgroundColor: module.backgroundColor ? `background-color:${module.backgroundColor};` : "",
        boxPadding: `margin-left:${module.boxPadding ?? 10}px;margin-right:${module.boxPadding ?? 10}px;`,
        boxPaddingBottom: `margin-bottom:${module.boxPaddingBottom ?? 5}px;`,
        boxPaddingTop: `margin-top:${module.boxPaddingTop ?? 5}px;`,
        boxRadius: `border-radius:${module.boxRadius ?? 0}px;`,
        innerPadding: `padding:${module.innerPadding ?? 0}px;`
    };
    const goodsPadding = cssText(`padding:${module.goodsPadding ?? 5}px;`);

    // 样式 7：每页 3 个商品切片轮播（与 Vue 版 getSwiperList 一致）
    const swiperList = useMemo(() => {
        const list: any[][] = [];
        if (Number(module.style) === 7) {
            const count = Math.ceil(productList.length / 3);
            for (let index = 0; index < count; index++) {
                list.push(productList.slice(index * 3, (index + 1) * 3));
            }
        }
        return list;
    }, [productList, module.style]);

    const [slideIndex, setSlideIndex] = useState(0);
    const swiperPageColor = module.swiperPageColor || "#333";
    // 样式7 分页点固定为样式一形态（原版 product/Preview.vue scoped：bullet 12×2px、radius 0、margin 0 1px）
    const renderDots = (count: number) => (
        <div className="image-ad-dots image-ad-dots--1">
            {Array.from({ length: count }).map((_, i) => (
                <i key={i} className={i === slideIndex ? "active" : ""} style={i === slideIndex ? { background: swiperPageColor } : undefined} />
            ))}
        </div>
    );

    const renderItemInfo = (item: any) => (
        <div className="item-info">
            <div className="item-name">
                {Number(module.showName) === 1 ? <a className="item-name-a">{item.productName}</a> : null}
                {Number(module.showBrief) === 1 && item.productBrief ? <a className="item-brief">{item.productBrief}</a> : null}
            </div>
            {Number(module.showPrice) === 1 ? (
                <div className="item-action">
                    <div className="item-price">
                        <b className="price_format">{priceFormat(Number(item.productPrice))}</b>
                    </div>
                    {/* 样式7 的 .item-buy 在原版为空（无购买按钮），其余样式才渲染购买区 */}
                    <div className="item-buy">
                        {Number(module.style) === 7 ? null : <LegacyBuyButton style={Number(module.buyBtnStyle)} />}
                    </div>
                </div>
            ) : null}
        </div>
    );

    return (
        <div
            className={
                "module-ad-con module-goods_ad ad-style__" + module.style +
                " ad-buy_btn_style__" + module.buyBtnStyle +
                " ad-goods_style__" + module.goodsStyle +
                " ad-goods_radio_style__" + module.goodsRadioStyle +
                " ad-text_align__" + module.textAlign +
                " ad-text_weight__" + module.textWeight +
                " ad-goods_name_row__" + module.goodsNameRow +
                " ad-goods_name_padding__" + module.goodsNamePadding +
                " ad-goods-title_style__" + module.title?.titleStyle
            }
            style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}
        >
            {/* 与 Vue 版一致：module-ad-content 承载 frame 的 backgroundColor/boxRadius +
                模块级 backgroundColor/外边距（boxPadding*→margin）/boxRadius/innerPadding（同名属性后者胜出，与原版 style 串顺序一致） */}
            <div
                className="module-ad-content"
                style={cssText(
                    frameFormat.backgroundColor,
                    frameFormat.boxRadius,
                    allFormat.backgroundColor,
                    allFormat.boxPadding,
                    allFormat.boxPaddingBottom,
                    allFormat.boxPaddingTop,
                    allFormat.boxRadius,
                    allFormat.innerPadding
                )}
            >
                {!productList || productList.length === 0 ? (
                    <div className="module-ad-empty empty-image_ad">
                        <div className="image-empty-goods-bg" />
                        <div className="desc">{t('点击添加商品')}</div>
                    </div>
                ) : null}
                {module.title?.showTitle ? <CommonTitle data={module.title} /> : null}
                <div className="goods-ad-warp">
                    <div className="goods-ad-con">
                        {Number(module.style) === 7 ? (
                            <Swiper
                                autoplay={3000}
                                loop={swiperList.length > 1}
                                onChange={(i: number) => setSlideIndex(i)}
                                indicator={renderDots(swiperList.length)}
                            >
                                {swiperList.map((group, gi) => (
                                    <SwiperItem key={gi}>
                                        {group.map((item, index) => (
                                            <div className="goods-ad-item" key={index}>
                                                <div className="item-content" style={goodsPadding}>
                                                    <div className="item-con">
                                                        <div className="item-photo">
                                                            <a className="item-image-a"><img src={productImage(item)} alt="" /></a>
                                                        </div>
                                                        {renderItemInfo(item)}
                                                    </div>
                                                </div>
                                            </div>
                                        ))}
                                    </SwiperItem>
                                ))}
                            </Swiper>
                        ) : (
                            productList.map((item) => (
                                <div className="goods-ad-item" key={item.productId}>
                                    <div className="item-content" style={goodsPadding}>
                                        <div className="item-con">
                                            <div className="item-photo">
                                                <a className="item-image-a"><img src={productImage(item)} alt="" /></a>
                                            </div>
                                            {renderItemInfo(item)}
                                        </div>
                                    </div>
                                </div>
                            ))
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
