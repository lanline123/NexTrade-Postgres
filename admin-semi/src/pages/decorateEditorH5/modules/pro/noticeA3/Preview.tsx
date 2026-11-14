/**
 * 商城公告3 预览（移植自 view/admin editor/modules/mobilePro/noticeA3/Preview.vue）
 *
 * 头图 + 竖直轮播公告（3s 自动切换），支持前缀标签/边框/内容底色。
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { noticeA3Defaults } from "./index";
import "./Preview.css";

export default function NoticeA3Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, noticeA3Defaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {});
    const noticeList: Array<{ title?: string; prefix?: string }> = module.noticeList || [];
    const [currentIndex, setCurrentIndex] = useState(0);
    // 原版 Vue transition notice-slide 为进出双向：离场项与入场项同播 0.5s，
    // 这里保留上一项渲染至离场动画结束（React 无内置 leave 钩子，等效实现）
    const [prevIndex, setPrevIndex] = useState<number | null>(null);
    const indexRef = useRef(0);

    useEffect(() => {
        indexRef.current = 0;
        setCurrentIndex(0);
        setPrevIndex(null);
        if (noticeList.length <= 1) return;
        const timer = window.setInterval(() => {
            const cur = indexRef.current;
            const next = (cur + 1) % noticeList.length;
            setPrevIndex(cur);
            indexRef.current = next;
            setCurrentIndex(next);
        }, 3000);
        return () => window.clearInterval(timer);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [noticeList.length]);

    useEffect(() => {
        if (prevIndex === null) return;
        const t = window.setTimeout(() => setPrevIndex(null), 500);
        return () => window.clearTimeout(t);
    }, [prevIndex, currentIndex]);

    const currentItem = noticeList[currentIndex] || noticeList[0];
    const prevItem = prevIndex !== null ? noticeList[prevIndex] : undefined;
    const wrapStyle: React.CSSProperties =
        Number(module.showBorder) === 1 && module.borderColor ? { border: `1px solid ${module.borderColor}` } : {};
    const bodyStyle: React.CSSProperties = module.contentBgColor ? { backgroundColor: module.contentBgColor } : {};

    return (
        <div
            className="module-ad-con module-notice-a3"
            style={cssText(commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}
        >
            <div
                className="module-ad-content"
                style={{
                    ...cssText(
                        commonStyle.moduleContentStyle.backgroundColor,
                        commonStyle.moduleContentStyle.backgroundImage,
                        commonStyle.moduleContentStyle.backgroundSize,
                        commonStyle.moduleContentStyle.boxRadius2,
                        commonStyle.moduleContentStyle.boxShadow,
                        commonStyle.moduleContentStyle.paddingTopBottom,
                        commonStyle.moduleContentStyle.paddingLeftRight,
                    ),
                    ...wrapStyle,
                }}
            >
                {noticeList.length === 0 ? (
                    <div className="notice-a3-empty">
                        <div className="desc">点击添加公告内容</div>
                    </div>
                ) : (
                    <div className="notice-a3-wrap">
                        {Number(module.showHeader) === 1 && module.headerImage?.picUrl && (
                            <div className="notice-a3-header">
                                <img src={imageFormat(module.headerImage.picUrl)} alt="" style={{ width: `${module.headerImageWidth}px`, height: "auto" }} />
                                <div className="notice-a3-divider" />
                            </div>
                        )}
                        <div className="notice-a3-body" style={bodyStyle}>
                            <div className="notice-a3-scroll">
                                {prevItem ? (
                                    <div className="notice-a3-item notice-a3-item--leave" key={`leave-${prevIndex}`}>
                                        {Number(module.showPrefix) === 1 && prevItem?.prefix && (
                                            <span className="notice-a3-prefix">{prevItem.prefix}</span>
                                        )}
                                        <span className="notice-a3-text" style={{ color: module.textColor || "#666666" }}>
                                            {prevItem?.title}
                                        </span>
                                    </div>
                                ) : null}
                                <div className={`notice-a3-item${prevItem ? " notice-a3-item--enter" : ""}`} key={currentIndex}>
                                    {Number(module.showPrefix) === 1 && currentItem?.prefix && (
                                        <span className="notice-a3-prefix">{currentItem.prefix}</span>
                                    )}
                                    <span className="notice-a3-text" style={{ color: module.textColor || "#666666" }}>
                                        {currentItem?.title}
                                    </span>
                                </div>
                            </div>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}
