/**
 * 优惠券 预览（移植自 view/admin editor/modules/mobilePromotion/coupon/Preview.vue）
 *
 * 根 class 带 ad-color_style__<colorStyle>（配色走全局 module.css 的 .ad-color_style__1..4）；
 * 外层仅 frame 的 boxPaddingTop/Bottom + 模块 boxMargin 左右外边距；内容层背景/圆角为
 * frame 值后叠模块 backgroundColor/boxRadius（后者胜出，同原版字符串拼接顺序）；
 * warp 内边距取模块 boxPadding/boxPaddingTop/boxPaddingBottom；showTitle===1 显示标题栏
 * （标题 16px #2a3145 + 描述 12px #aaa + 更多）；券面为单张演示券（5.00 元 / 满60减5，itemPadding 内边距）。
 */
import { useMemo, type CSSProperties } from "react";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { couponDefaults } from "./index";
import "./Preview.css";

function ArrowRight({ size = 12 }: { size?: number }) {
    return (
        <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true" style={{ flexShrink: 0 }}>
            <polyline points="9 4.5 16.5 12 9 19.5" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

export default function CouponPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, couponDefaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    // 外层：frame 上下内边距（formatFrame 的 boxPaddingTop/Bottom 语义）+ 模块左右外边距
    const outerStyle: CSSProperties = {
        paddingTop: frame.boxPaddingTop ?? 0,
        paddingBottom: frame.boxPaddingBottom ?? 0,
        marginLeft: module.boxMargin ?? 0,
        marginRight: module.boxMargin ?? 0
    };
    const contentStyle: CSSProperties = {
        backgroundColor: module.backgroundColor || frame.backgroundColor || undefined,
        borderRadius: module.boxRadius ?? frame.boxRadius ?? 0
    };
    const warpStyle: CSSProperties = {
        paddingLeft: module.boxPadding ?? 0,
        paddingRight: module.boxPadding ?? 0,
        paddingTop: module.boxPaddingTop ?? 0,
        paddingBottom: module.boxPaddingBottom ?? 0
    };

    return (
        <div className={`module-ad-con module-image_hotarea ad-color_style__${module.colorStyle ?? 0} module-promotion-coupon`} style={outerStyle}>
            <div className="module-ad-content" style={contentStyle}>
                <div className="coupon-ad-warp" style={warpStyle}>
                    {module.showTitle === 1 ? (
                        <div className="coupon-title">
                            <div className="coupon-title-text">
                                <h4 className="coupon-maintitle">{module.title}</h4>
                                <span className="coupon-title-con">{module.desc}</span>
                            </div>
                            <div>
                                <a className="oupon-title-more">
                                    更多 <ArrowRight />
                                </a>
                            </div>
                        </div>
                    ) : null}
                    <div className="coupon-con">
                        <div className="coupon-item" style={{ padding: module.itemPadding ?? 10 }}>
                            <div className="item-coupon-con">
                                <div className="coupon-money">
                                    <div className="coupon-money-c">5.00</div>
                                    <div className="coupon-money-d">元</div>
                                </div>
                                <div className="coupon-name">
                                    <div className="coupon-name-text">满60减5</div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
