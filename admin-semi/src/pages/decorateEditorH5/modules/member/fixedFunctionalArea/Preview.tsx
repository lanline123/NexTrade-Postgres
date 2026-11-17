/**
 * 固定功能区 预览（移植自 view/admin editor/modules/mobileMember/fixedFunctionalArea/Preview.vue）
 *
 * 门控与原版一致：整体 hasContent = isB2b()||isStore() 才渲染，账期卡本身另有
 * v-if="isB2b()"（仅门店版时只剩空 wrap，与原版同构）。
 * frame 消费口径与原版一致：外层根节点吃 boxPadding 三项；.module-ad-content
 * （fixed-area-wrap）吃 wrapStyle = backgroundColor/boxRadius/innerPadding。
 * 卡 margin 10px 0、圆角 9、背景图 cover/center（原版为本地资源 termPay_card.png，
 * ref 快照缺失该文件，暂用既有 OSS 图，待核）；标题栏高 44、标题 13px/600 #ae5d34 +
 * 隐藏态眼睛图标（icon-buxianshi 语义）+「查看详情」胶囊（内含 8px 更多箭头）；
 * 内容区 min-height 60px、padding 15px 0，三列 信用额度/已用额度/剩余额度（值 ******）。
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { isB2b, isStore } from "../../../contract/version";
import { fixedFunctionalAreaDefaults } from "./index";
import "./Preview.css";

const AMOUNTS = [
    { label: "信用额度", value: "******" },
    { label: "已用额度", value: "******" },
    { label: "剩余额度", value: "******" }
];

/** 隐藏态眼睛（对齐原版 iconfont-h5 icon-buxianshi，15px） */
function EyeOffIcon() {
    return (
        <svg
            viewBox="0 0 24 24"
            width="15"
            height="15"
            aria-hidden="true"
            className="ffa-card__toggle-eye ffa-card__toggle-eye--term"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            style={{ flexShrink: 0 }}
        >
            <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" />
            <line x1="1" y1="1" x2="23" y2="23" />
        </svg>
    );
}

/** 更多箭头（对齐原版 iconfont-h5 icon-a-gengduo20，8px） */
function MoreArrowIcon() {
    return (
        <svg viewBox="0 0 16 16" width="8" height="8" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ flexShrink: 0 }}>
            <polyline points="6 3 11 8 6 13" />
        </svg>
    );
}

export default function FixedFunctionalAreaPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, fixedFunctionalAreaDefaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);

    if (!isB2b() && !isStore()) return null;

    // 原版 wrapStyle：wrap 消费 frame 的背景色/圆角/内边距
    const wrapStyle: React.CSSProperties = {
        backgroundColor: frame.backgroundColor || "transparent",
        borderRadius: `${frame.boxRadius ?? 0}px`,
        padding: `${frame.innerPadding ?? 0}px`,
        boxSizing: "border-box"
    };

    return (
        <div
            className="module-ad-con module-member-fixed-functional-area"
            style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}
        >
            <div className="module-ad-content fixed-area-wrap" style={wrapStyle}>
                <div className="fixed-area-container">
                    {isB2b() ? (
                        <div className="ffa-card ffa-card--term">
                            <div className="ffa-card__title">
                                <div className="ffa-card__title-left">
                                    <span className="ffa-card__title-text ffa-card__title-text--term">账期支付</span>
                                    <EyeOffIcon />
                                </div>
                                <div className="ffa-card__title-more ffa-card__title-more--term">
                                    <span className="ffa-card__title-more-text">查看详情</span>
                                    <MoreArrowIcon />
                                </div>
                            </div>
                            <div className="ffa-card__content">
                                <div className="ffa-card__wrap">
                                    {AMOUNTS.map((item) => (
                                        <div className="ffa-card__item" key={item.label}>
                                            <div className="ffa-card__item-number ffa-card__item-number--term">{item.value}</div>
                                            <div className="ffa-card__item-text ffa-card__item-text--term">{item.label}</div>
                                        </div>
                                    ))}
                                </div>
                            </div>
                        </div>
                    ) : null}
                </div>
            </div>
        </div>
    );
}
