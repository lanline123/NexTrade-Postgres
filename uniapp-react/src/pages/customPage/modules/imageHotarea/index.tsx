import { useMemo, useState } from "react";
import { Text, View } from "@tarojs/components";

import { designPxToRpx, formatFrame } from "@/components/modules";
import TigImage from "@/components/tig/tig-image/TigImage";
import CommonTitle from "@/pages/customPage/modules/commonTitle/index";
import { redirect } from "@/utils";
import { urlFormat } from "@/utils/format";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

/**
 * 是否处于后台编辑器嵌入态（decoratePreview iframe）。
 * 2026-11-20 症状三：热图未选图时本模块此前无任何在流占位——空 picList 只剩
 * frame padding、空 picUrl 条目 TigImage 塌成 0 高，编辑器 decorate-layout
 * 如实上报 ~0 高，覆盖槽位与下一模块挤在同一带、视觉重叠。按 PC 编辑器
 * HotareaPreview 口径，仅在嵌入态补在流占位（空态块 / 40% 比例块）；
 * 前台真实访问（非 iframe）行为与此前完全一致，不显示编辑占位。
 */
const inEditorEmbed = (): boolean => {
    try {
        if (typeof window === "undefined") return false;
        return window.self !== window.top || window.location.href.indexOf("decoratePreview") >= 0;
    } catch {
        return false;
    }
};

export default function ImageHotarea({ module }: Props) {
    const frameFormat = useMemo(() => {
        return formatFrame(module.frame ?? {});
    }, [module.frame]);

    const format = useMemo(() => {
        return {
            imgPadding: "padding:" + designPxToRpx(module?.imgPadding) + ";"
        };
    }, [module?.imgPadding]);

    const embed = inEditorEmbed();
    const picListEmpty = !module.picList || module.picList.length === 0;

    function convertPxToRpx(position: string): string {
        return position.replace(/(\d+\.?\d*)px/g, (match, num) => {
            return parseFloat(num) * 2 + "rpx";
        });
    }

    /** 热区坐标基准宽（画板口径 500，与 admin 预览 / PC 渲染器一致） */
    const HOTAREA_BASE = 500;

    /**
     * 热区样式派生。数据契约（types/decorate HotareaType + 画板写回 + 现网存量数据）：
     * hotarea[] 存 {left,top,width,height} 数字字段（500 基准坐标系），position 只是
     * 原版编辑器侧渲染器（Preview/View 的 updateWidth）在渲染时派生的可选字段，
     * 存储数据里从来没有——旧实现直接 convertPxToRpx(subItem.position) 对 undefined
     * 调 .replace，未画热区时不触发、画完热区必抛 TypeError 被错误边界降级为
     * 「该模块渲染失败，已跳过」（2026-11-21 10106 实测）。
     * 换算与原版 resize(容器宽/500) 等价且与分辨率无关：left/width 取容器宽百分比；
     * top/height 取容器高百分比（容器高 = 图片渲染高，故按图片原始宽高比
     * aspect = 原始宽/原始高 折算：top% = top/500 × aspect × 100）。aspect 由
     * TigImage onLoad 的原始尺寸得到，未取得时返回 undefined（暂不渲染该热区，
     * 图片就绪即出现）。更老世代数据若只有 position 字符串，沿用旧换算兜底
     * （与 PC 渲染器 parseCssText 兜底同义）。
     */
    function hotareaStyle(subItem: any, aspect: number | undefined): string | undefined {
        const num = (v: any) => {
            const n = Number(v);
            return Number.isFinite(n) ? n : 0;
        };
        const hasNumeric = ["left", "top", "width", "height"].some((k) => subItem?.[k] !== undefined && subItem?.[k] !== null && subItem?.[k] !== "");
        if (hasNumeric) {
            if (!aspect) return undefined;
            const left = (num(subItem.left) / HOTAREA_BASE) * 100;
            const width = (num(subItem.width) / HOTAREA_BASE) * 100;
            const top = (num(subItem.top) / HOTAREA_BASE) * aspect * 100;
            const height = (num(subItem.height) / HOTAREA_BASE) * aspect * 100;
            return `left:${left}%;top:${top}%;width:${width}%;height:${height}%;`;
        }
        if (typeof subItem?.position === "string" && subItem.position) {
            return convertPxToRpx(subItem.position);
        }
        return undefined;
    }

    /** 各图片条目的原始宽高比（onLoad 取得；热区纵向百分比换算用） */
    const [imgAspects, setImgAspects] = useState<Record<number, number>>({});
    const handleImgLoad = (itemIndex: number) => (e: any) => {
        const w = Number(e?.detail?.width);
        const h = Number(e?.detail?.height);
        if (w > 0 && h > 0) {
            const aspect = w / h;
            setImgAspects((cur) => (cur[itemIndex] === aspect ? cur : { ...cur, [itemIndex]: aspect }));
        }
    };

    return (
        <View
            className={`module-ad-con decorate-image-hotarea module-ad-con- ad-radio_style__${module.radioStyle}`}
            style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
        >
            <View className="module-ad-content" style={frameFormat.backgroundColor + " " + frameFormat.boxRadius}>
                {module.title.showTitle == 1 && <CommonTitle module={module.title} />}
                {embed && picListEmpty ? (
                    /* 空态在流占位（PC 口径：paddingTop 30 / paddingBottom 50 / #f2f4f6 /
                       100×100 占位盒 + 文案，总高约 210 设计 px） */
                    <View className="image-hotarea-empty">
                        <View className="image-hotarea-empty__box" />
                        <Text className="image-hotarea-empty__text">点击添加热图绘制</Text>
                    </View>
                ) : null}
                {module.picList && (
                    <View className="image-ad-warp image-hotarea-warp" style={frameFormat.innerPadding}>
                        <View className="image-ad-con">
                            {module.picList.map((item: any, itemIndex: number) => (
                                <View className="image-ad-item" key={item.picId ?? itemIndex}>
                                    <View className="item-content" style={format.imgPadding}>
                                        <View className="item-content-hotatrea">
                                            {/* TigImage 不接收 className，旧 <tig-image class="item-img-bg"> 的样式由外层 View 承接 */}
                                            {embed && !item.picUrl ? (
                                                /* 未选图条目在流比例占位（PC 同口径 paddingTop:40%） */
                                                <View className="item-img-bg image-hotarea-img-placeholder" />
                                            ) : (
                                                <View className="item-img-bg">
                                                    <TigImage src={item.picUrl} mode="widthFix" onLoad={handleImgLoad(itemIndex)} />
                                                </View>
                                            )}
                                            {item.hotarea &&
                                                item.hotarea.map((subItem: any, index: number) => {
                                                    const style = hotareaStyle(subItem, imgAspects[itemIndex]);
                                                    if (style === undefined) return null;
                                                    return (
                                                        <View
                                                            key={index}
                                                            className="image-hotarea-link"
                                                            style={style}
                                                            onClick={() => {
                                                                if (!subItem.link) return;
                                                                const url = urlFormat(subItem.link);
                                                                if (url) redirect({ url });
                                                            }}
                                                        >
                                                            <Text className="area_box_con_text">{subItem.link?.name ?? ""}</Text>
                                                        </View>
                                                    );
                                                })}
                                        </View>
                                    </View>
                                </View>
                            ))}
                        </View>
                    </View>
                )}
            </View>
        </View>
    );
}
