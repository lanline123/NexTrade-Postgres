/**
 * 优惠券 预览（DOM 类名逐字对齐 uniapp-react 渲染端 coupon/index.tsx + index.scss）
 *
 * 券列表在渲染端由 getHomeCoupon 接口拉取，编辑器预览用演示券兜底；
 * 演示字段与接口/渲染端使用一致：couponId、couponType（2=折扣券）、couponMoney、
 * couponDiscount、couponName。
 */
import { useMemo } from "react";
import { cssText, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { couponDefaults } from "./index";
import "./Preview.css";

/** 演示券（仅预览兜底，不写入模块数据） */
const DEMO_COUPONS = [
    { couponId: 1, couponType: 1, couponMoney: 50, couponDiscount: 0, couponName: "满300元可用" },
    { couponId: 2, couponType: 1, couponMoney: 20, couponDiscount: 0, couponName: "满199元可用" },
    { couponId: 3, couponType: 2, couponMoney: 0, couponDiscount: 8.5, couponName: "全场折扣券" },
    { couponId: 4, couponType: 1, couponMoney: 10, couponDiscount: 0, couponName: "无门槛优惠券" }
];

export default function CouponPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, couponDefaults);
        return merged;
    }, [data]);

    const frameFormat = formatFrame(module.frame ?? {});
    const couponFormat = {
        boxPadding: `padding-left:${module.boxPadding ?? 0}px;padding-right:${module.boxPadding ?? 0}px;`,
        boxMargin: `margin-left:${module.boxMargin ?? 0}px;margin-right:${module.boxMargin ?? 0}px;`,
        boxPaddingBottom: `padding-bottom:${module.boxPaddingBottom ?? 0}px;`,
        boxPaddingTop: `padding-top:${module.boxPaddingTop ?? 0}px;`,
        boxRadius: `border-radius:${module.boxRadius ?? 0}px;`,
        itemPadding: `padding:${module.itemPadding ?? 0}px;`,
        backgroundColor: module.backgroundColor ? `background-color:${module.backgroundColor};` : ""
    };

    return (
        <div
            className={`module-ad-con module-image_hotarea ad-color_style__${module.colorStyle}`}
            style={cssText(frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom, couponFormat.boxMargin)}
        >
            <div
                className="module-ad-content"
                style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius, couponFormat.boxRadius, couponFormat.backgroundColor)}
            >
                <div
                    className="coupon-ad-warp"
                    style={cssText(couponFormat.boxPadding, couponFormat.boxPaddingBottom, couponFormat.boxPaddingTop)}
                >
                    {Number(module.showTitle) === 1 ? (
                        <div className="coupon-title">
                            <div className="coupon-title-text">
                                <div className="coupon-maintitle">{module.title}</div>
                                <div className="coupon-title-con">{module.desc}</div>
                            </div>
                            <div>
                                <div className="more-box flex align-center">
                                    <span className="more">更多</span>
                                    <span className="iconfont-h5 icon-youjiantou" />
                                </div>
                            </div>
                        </div>
                    ) : null}
                    <div className="coupon-con">
                        {DEMO_COUPONS.map((item) => (
                            <div key={item.couponId} className="coupon-item" style={cssText(couponFormat.itemPadding)}>
                                <div className="item-coupon-con">
                                    <div className="coupon-money">
                                        {item.couponType === 2 ? (
                                            <>
                                                <div className="zhekou">{item.couponDiscount}</div>
                                                <div className="zhe">折</div>
                                            </>
                                        ) : (
                                            <span>¥{priceFormat(item.couponMoney)}</span>
                                        )}
                                    </div>
                                    <div className="coupon-name">
                                        <div className="coupon-name-text">{item.couponName}</div>
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}
