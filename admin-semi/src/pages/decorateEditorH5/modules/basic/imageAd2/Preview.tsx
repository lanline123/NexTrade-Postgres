/**
 * 图片广告2 预览（移植自 view/admin editor/modules/mobile/imageAd2/Preview.vue）
 *
 * 结构/类名与 Vue 版逐一致（module-ad-con / module-ad-warp / image-ad-warp / image-ad-con / image-ad-item），
 * 样式由移植的全局 module.css 提供；轮播使用 NutUI Swiper，showIndicator 控制指示器显隐。
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { Swiper, SwiperItem } from "@nutui/nutui-react";
import "./Preview.css";
import CommonTitle from "../../../shared/CommonTitle";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { imageAd2Defaults } from "./index";

import { useI18n } from '../../../../../i18n';
export default function ImageAd2Preview({ data }: { data: Record<string, any> }) {
    const { t } = useI18n();
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, imageAd2Defaults);
        return merged;
    }, [data]);

    const frameFormat = formatFrame(module.frame ?? {});
    const imgPaddingStyle = cssText(`padding:${module.imgPadding}px;`);
    const swiperPageColor = module.swiperPageColor || "#333";
    const columns = module.swiperPreView || 1;

    // 轮播定高（2026-11-21 与原版对齐改版）：原版编辑器 Preview 用 Swiper.js，容器高度由
    // 内容决定——单图模式 = 最高一张幻灯的自然高（img 宽 100%、高 auto、不裁剪），
    // 分组模式 = 最高一组的行高。此前我方借 banner 口径按首图比例定高 + object-fit:cover
    // 裁剪、且 useSwiperHeight 有 375px 上限封顶，高图/混比例图与原版明显不符。
    // 现按原版几何测高：离屏测全部图片的高宽比，单图取 max、分组取每组 max 再取 max；
    // 未测得的图片先按高宽比 0.5（原 fallbackRatio 2 的倒数）估算，测得后自动修正。
    const singleMode = !(module.swiperPreView && module.swiperPreView != 1);
    const swiperWrapRef = useRef<HTMLDivElement | null>(null);
    const [wrapWidth, setWrapWidth] = useState(0);
    const [hwRatios, setHwRatios] = useState<Record<string, number>>({});
    // 编辑器画布宽度固定，渲染后测一次容器实际宽度（与 useSwiperHeight 同测法）
    useEffect(() => {
        if (swiperWrapRef.current) {
            const w = swiperWrapRef.current.clientWidth;
            if (w > 0) setWrapWidth((prev) => (prev === w ? prev : w));
        }
    });
    useEffect(() => {
        let alive = true;
        (module.picList || []).forEach((p: any) => {
            const url = p?.picUrl ? imageFormat(p.picUrl) : "";
            if (!url) return;
            const im = new Image();
            im.onload = () => {
                if (alive && im.naturalWidth > 0 && im.naturalHeight > 0) {
                    setHwRatios((cur) => (cur[url] != null ? cur : { ...cur, [url]: im.naturalHeight / im.naturalWidth }));
                }
            };
            im.src = url;
        });
        return () => {
            alive = false;
        };
    }, [module.picList]);

    // NutUI 分页指示器：与 uniapp 端小圆点一致（active 高亮 swiperPageColor）；
    // Vue 版 showIndicator == 1 才显示 pagination
    const [slideIndex, setSlideIndex] = useState(0);
    const renderDots = (count: number) => (
        <div className={`image-ad-dots image-ad-dots--${module.picPageType || 1}`}>
            {Array.from({ length: count }).map((_, i) => (
                <i key={i} className={i === slideIndex ? "active" : ""} style={i === slideIndex ? { background: swiperPageColor } : undefined} />
            ))}
        </div>
    );

    // 分组轮播：按 swiperPreView 切片（与 Vue 版 getSwiperList 一致）
    const swiperList = useMemo(() => {
        const list: any[][] = [];
        const itemNum = module.swiperPreView || 1;
        const picList: any[] = module.picList || [];
        const count = Math.ceil(picList.length / itemNum);
        for (let index = 0; index < count; index++) {
            list.push(picList.slice(index * itemNum, (index + 1) * itemNum));
        }
        return list;
    }, [module.picList, module.swiperPreView]);

    // 容器高度（原版几何）：图片实际宽 = 容器宽 − item-content 左右 padding（imgPadding×2），
    // 高 = 图宽 × 高宽比 + 上下 padding；单图取全部图片 max、分组取每组 max 的 max。
    const FALLBACK_HW = 0.5;
    const hwOf = (p: any) => {
        const url = p?.picUrl ? imageFormat(p.picUrl) : "";
        return (url && hwRatios[url]) || FALLBACK_HW;
    };
    const imgPad = module.imgPadding || 0;
    let swiperHeight: number | undefined;
    if (wrapWidth > 0 && (module.picList || []).length > 0) {
        if (singleMode) {
            const imgW = Math.max(0, wrapWidth - imgPad * 2);
            swiperHeight = Math.round(imgW * Math.max(...(module.picList as any[]).map(hwOf)) + imgPad * 2);
        } else if (swiperList.length > 0) {
            const cellW = (wrapWidth - (columns - 1) * imgPad) / columns;
            const imgW = Math.max(0, cellW - imgPad * 2);
            swiperHeight = Math.round(Math.max(...swiperList.map((g) => imgW * Math.max(...g.map(hwOf)) + imgPad * 2)));
        }
    }

    return (
        <div
            className={
                "module-ad-con module-image_ad " +
                " ad-pic_type__" +
                module.picType +
                " ad-pic_page_type__" +
                module.picPageType +
                " ad-radio_style__" +
                module.picRadioStyle +
                " "
            }
        >
            <div
                className="module-ad-content"
                style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom, frameFormat.backgroundColor)}
            >
                <div className="module-ad-warp" style={cssText(frameFormat.boxRadius)}>
                    {module.title?.showTitle ? <CommonTitle data={module.title} /> : null}
                    {!module.picList || module.picList.length === 0 ? (
                        <div className="module-ad-empty empty-image_ad">
                            <div className="image-empty-bg" />
                            <div className="desc">{t('点击添加图片广告')}</div>
                        </div>
                    ) : null}
                    {module.picType == 2 ? (
                        <div className="image-ad-warp" style={cssText(frameFormat.innerPadding)}>
                            <div
                                ref={swiperWrapRef}
                                className={"image-ad-con image-ad2-con--fixedh" + (singleMode ? "" : " image-ad2-con--grouped")}
                                style={swiperHeight ? { height: swiperHeight } : undefined}
                            >
                                {module.swiperPreView && module.swiperPreView == 1 ? (
                                    <Swiper
                                        autoplay={3000}
                                        loop={module.picList.length > 1}
                                        onChange={(i: number) => setSlideIndex(i)}
                                        indicator={module.showIndicator == 1 ? renderDots(module.picList.length) : undefined}
                                    >
                                        {(module.picList as any[]).map((item, i) => (
                                            <SwiperItem key={i}>
                                                <div className="image-ad-item">
                                                    <div className="item-content" style={imgPaddingStyle}>
                                                        <div className="item-img-a">
                                                            <img className="item-img" src={imageFormat(item.picUrl)} alt="" />
                                                        </div>
                                                    </div>
                                                </div>
                                            </SwiperItem>
                                        ))}
                                    </Swiper>
                                ) : (
                                    <Swiper
                                        autoplay={3000}
                                        loop={swiperList.length > 1}
                                        onChange={(i: number) => setSlideIndex(i)}
                                        indicator={module.showIndicator == 1 ? renderDots(swiperList.length) : undefined}
                                    >
                                        {swiperList.map((group, gi) => (
                                            <SwiperItem key={gi}>
                                                <div
                                                    className="swiper-main"
                                                    style={{
                                                        display: "grid",
                                                        gridTemplateColumns: `repeat(${columns}, 1fr)`,
                                                        columnGap: `${module.imgPadding}px`,
                                                        boxSizing: "border-box",
                                                        width: "100%"
                                                    }}
                                                >
                                                    {group.map((subItem, si) => (
                                                        <div key={si}>
                                                            <div className="image-ad-item">
                                                                <div className="item-content" style={imgPaddingStyle}>
                                                                    <div className="item-img-a">
                                                                        <img className="item-img" src={imageFormat(subItem.picUrl)} alt="" />
                                                                    </div>
                                                                </div>
                                                            </div>
                                                        </div>
                                                    ))}
                                                </div>
                                            </SwiperItem>
                                        ))}
                                    </Swiper>
                                )}
                            </div>
                        </div>
                    ) : (
                        <div className="image-ad-warp" style={cssText(frameFormat.innerPadding)}>
                            <div className="image-ad-con">
                                {(module.picList as any[])?.map?.((pic, key) => (
                                    <div className="image-ad-item" key={key}>
                                        <div className="item-content" style={imgPaddingStyle}>
                                            {pic.picUrl ? (
                                                <div className="item-img-a">
                                                    <img className="item-img" src={imageFormat(pic.picUrl)} alt="" />
                                                </div>
                                            ) : (
                                                <div className="module-ad-empty empty-image_ad">
                                                    <div className="image-empty-bg" />
                                                    <div className="desc">{t('点击添加图片广告')}</div>
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
