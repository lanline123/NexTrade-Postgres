/**
 * 大图滑动 预览（类名对齐 uniapp-react slideLargeImage：slidelarge-image）
 */
import { useMemo, useState } from "react";
import { Swiper, SwiperItem } from "@nutui/nutui-react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { slideLargeImageDefaults } from "./index";
import "./Preview.css";

export default function SlideLargeImagePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, slideLargeImageDefaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const picList: any[] = module.picList || [];
    const [idx, setIdx] = useState(0);
    return (
        <div className="module-ad-con" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                {module.moduleTitle?.title ? <div style={{ textAlign: module.moduleTitle.titlePosition ?? "left", fontWeight: 600, marginBottom: 6 }}>{module.moduleTitle.title}</div> : null}
                <div className="slidelarge-image" style={{ marginTop: `${module.picMarginTop ?? 0}px` }}>
                    {picList.length === 0 ? (<div className="module-ad-empty empty-image_ad"><div className="image-empty-bg" /><div className="desc">点击添加图片</div></div>) : (
                        <Swiper autoplay={Number(module.slidingSpeed) || 3000} loop={picList.length > 1} onChange={(i: number) => setIdx(i)} indicator={false}>
                            {picList.map((item, i) => (
                                <SwiperItem key={i}>
                                    <div style={{ margin: `0 ${module.picMargin ?? 8}px`, borderRadius: `${module.picRadius ?? 8}px`, overflow: "hidden", position: "relative" }}>
                                        <img src={imageFormat(item.picUrl)} alt="" style={{ width: "100%", display: "block" }} />
                                        {(item.title || item.desc) ? <div style={{ position: "absolute", left: 0, right: 0, bottom: 0, padding: 8, backgroundColor: module.picTitleDesc?.backgroundColor || "rgba(0,0,0,0.35)", color: "#fff", textAlign: module.picTitleDesc?.fontPosition ?? "left" }}><div style={{ fontSize: 14, fontWeight: 600 }}>{item.title}</div><div style={{ fontSize: 12 }}>{item.desc}</div></div> : null}
                                    </div>
                                </SwiperItem>
                            ))}
                        </Swiper>
                    )}
                    {picList.length > 1 ? <div className="slidelarge-dots">{picList.map((_, i) => (<i key={i} className={i === idx ? "on" : ""} />))}</div> : null}
                </div>
            </div>
        </div>
    );
}
