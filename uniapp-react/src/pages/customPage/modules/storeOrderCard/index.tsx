import { useMemo } from "react";
import { Image, View } from "@tarojs/components";

import { designPxToRpx } from "@/components/modules";
import { redirect } from "@/utils";
import { imageFormat, urlFormat } from "@/utils/format";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
    /** 店铺装修分发器注入的当前店铺 ID；卡片未配链接时点单入口兜底跳本店点单页 */
    shopId?: number;
}

/** 点单卡片单项（契约：原版编辑器 mobilePro/storeOrderCard/types.ts） */
interface StoreOrderCardItemData {
    picId?: number | string;
    picUrl?: string | { picUrl?: string; picThumb?: string };
    picThumb?: string;
    picLarge?: string;
    picName?: string;
    title?: string;
    desc?: string;
    /** SelectLink 值：编辑器写回为 { link: { link: <url> } } 嵌套结构，也可能是 { path, data } 结构化对象或字符串 */
    link?: unknown;
}

/** frame 在通用字段之外追加的图片边距与拆分内边距（原版 moduleFrameDefault） */
interface StoreOrderCardFrameData {
    boxRadius?: number;
    innerPadding?: number;
    innerPaddingLeftRight?: number;
    innerPaddingTopBottom?: number;
    imagePaddingTop?: number;
    imagePaddingBottom?: number;
    imagePaddingLeftRight?: number;
    boxPadding?: number;
    boxPaddingTop?: number;
    boxPaddingBottom?: number;
    [key: string]: any;
}

interface StoreOrderCardData {
    picType?: number;
    cardStyle?: number;
    boxStyle?: number;
    backgroundColor?: string;
    cardBackground?: string;
    cardGap?: number;
    frame?: StoreOrderCardFrameData;
    cardList?: StoreOrderCardItemData[];
}

/** 各模板对应卡片张数（原版 LENGTH_MAP） */
const LENGTH_MAP: Record<number, number> = { 1: 1, 2: 2, 3: 3, 4: 3, 5: 4, 6: 5 };
/** 等分栅格列数（原版 Preview colsMap） */
const COLS_MAP: Record<number, number> = { 1: 1, 2: 2, 4: 3, 5: 4 };

/** frame 归一后的完整形态（全部为设计稿 px 数值） */
interface NormalizedFrame {
    boxRadius: number;
    innerPadding: number;
    innerPaddingLeftRight: number;
    innerPaddingTopBottom: number;
    imagePaddingTop: number;
    imagePaddingBottom: number;
    imagePaddingLeftRight: number;
    boxPadding: number;
    boxPaddingTop: number;
    boxPaddingBottom: number;
}

/** frame 默认：通用默认叠加本模块覆盖（圆角 8、内边距 10/10/10、图片边距 0）+ 外框边距口径同原版 Preview 兜底 */
const FRAME_DEFAULTS: NormalizedFrame = {
    boxRadius: 8,
    innerPadding: 10,
    innerPaddingLeftRight: 10,
    innerPaddingTopBottom: 10,
    imagePaddingTop: 0,
    imagePaddingBottom: 0,
    imagePaddingLeftRight: 0,
    boxPadding: 10,
    boxPaddingTop: 5,
    boxPaddingBottom: 5
};

const EMPTY_CARD: StoreOrderCardItemData = { picId: 0, picThumb: "", picUrl: "", picName: "", title: "", desc: "", link: {} };

const numOr = (value: unknown, fallback: number): number => {
    if (value === undefined || value === null || value === "") return fallback;
    const n = Number(value);
    return Number.isFinite(n) ? n : fallback;
};

/**
 * frame 归一（同原版 Preview.vue 口径）：先在原始 frame 上做历史数据迁移——
 * 仅有 innerPadding 时回填 innerPaddingLeftRight/TopBottom，再叠加模块默认。
 */
function normalizeFrame(raw: StoreOrderCardFrameData | undefined): NormalizedFrame {
    const src: StoreOrderCardFrameData = raw && typeof raw === "object" ? raw : {};
    const migrated: StoreOrderCardFrameData = { ...src };
    if (migrated.innerPaddingLeftRight === undefined || migrated.innerPaddingLeftRight === null) {
        migrated.innerPaddingLeftRight = migrated.innerPadding ?? undefined;
    }
    if (migrated.innerPaddingTopBottom === undefined || migrated.innerPaddingTopBottom === null) {
        migrated.innerPaddingTopBottom = migrated.innerPadding ?? undefined;
    }
    return {
        boxRadius: numOr(migrated.boxRadius, FRAME_DEFAULTS.boxRadius),
        innerPadding: numOr(migrated.innerPadding, FRAME_DEFAULTS.innerPadding),
        innerPaddingLeftRight: numOr(migrated.innerPaddingLeftRight, FRAME_DEFAULTS.innerPaddingLeftRight),
        innerPaddingTopBottom: numOr(migrated.innerPaddingTopBottom, FRAME_DEFAULTS.innerPaddingTopBottom),
        imagePaddingTop: numOr(migrated.imagePaddingTop, FRAME_DEFAULTS.imagePaddingTop),
        imagePaddingBottom: numOr(migrated.imagePaddingBottom, FRAME_DEFAULTS.imagePaddingBottom),
        imagePaddingLeftRight: numOr(migrated.imagePaddingLeftRight, FRAME_DEFAULTS.imagePaddingLeftRight),
        boxPadding: numOr(migrated.boxPadding, FRAME_DEFAULTS.boxPadding),
        boxPaddingTop: numOr(migrated.boxPaddingTop, FRAME_DEFAULTS.boxPaddingTop),
        boxPaddingBottom: numOr(migrated.boxPaddingBottom, FRAME_DEFAULTS.boxPaddingBottom)
    };
}

/** 卡片图片地址：口径同编辑器 productImage/pickPic（picUrl → picThumb → picLarge），兼容 picUrl 为对象形态的存量数据 */
function cardPicSrc(item: StoreOrderCardItemData | undefined): string {
    const pick = (v: unknown): string => {
        if (typeof v === "string") return v;
        if (v && typeof v === "object") return (v as any).picUrl || (v as any).picThumb || "";
        return "";
    };
    return pick(item?.picUrl) || pick(item?.picThumb) || pick(item?.picLarge) || "";
}

/**
 * 卡片链接解析：编辑器 SelectLink 写回的是嵌套结构（link.link.link 为 URL 字符串），
 * 逐层下钻 .link 直到字符串或 { path, data } 结构化对象，再交本仓 urlFormat 组装；
 * 带 path 的对象是 SelectLink 终态值（含 base/custom 等以 link/appLink 为参数的形态），不下钻。
 */
function resolveCardUrl(link: unknown): string {
    let cur: any = link;
    for (let depth = 0; depth < 5; depth++) {
        if (typeof cur === "string") return urlFormat(cur);
        if (!cur || typeof cur !== "object") return "";
        if (typeof cur.path === "string" && cur.path) return urlFormat(cur);
        if (!("link" in cur)) return "";
        cur = cur.link;
    }
    return typeof cur === "string" ? urlFormat(cur) : "";
}

type CardSize = "large" | "medium" | "small";

function CardBox({ item, cardStyle, size, frame }: { item: StoreOrderCardItemData; cardStyle: number; size: CardSize; frame: NormalizedFrame }) {
    const src = cardPicSrc(item);
    const paddingX = frame.innerPaddingLeftRight;
    const paddingY = frame.innerPaddingTopBottom;
    const imgTop = frame.imagePaddingTop;
    const imgBottom = frame.imagePaddingBottom;
    const imgLeftRight = frame.imagePaddingLeftRight;

    if (cardStyle === 2) {
        // 图片模式：整卡即图片，图片边距叠加到内容内边距上
        const padding =
            `padding-top:${designPxToRpx(paddingY + imgTop)};` +
            `padding-bottom:${designPxToRpx(paddingY + imgBottom)};` +
            `padding-left:${designPxToRpx(paddingX + imgLeftRight)};` +
            `padding-right:${designPxToRpx(paddingX + imgLeftRight)};`;
        return (
            <View className={`store-order-card-content mode-image size-${size}`} style={padding}>
                {src ? <Image className="store-order-full-img" src={imageFormat(src)} mode="aspectFit" /> : <View className="store-order-empty-img" />}
            </View>
        );
    }

    // 图文模式：DOM 顺序文字在前、图片在后，样式以 column-reverse 实现图上文下（size-large 横排）
    const imgMargin =
        `margin-top:${designPxToRpx(imgTop)};` +
        `margin-bottom:${designPxToRpx(imgBottom)};` +
        `margin-left:${designPxToRpx(imgLeftRight)};` +
        `margin-right:${designPxToRpx(imgLeftRight)};`;
    return (
        <View
            className={`store-order-card-content mode-textimg size-${size}`}
            style={`padding-top:${designPxToRpx(paddingY)};padding-bottom:${designPxToRpx(paddingY)};padding-left:${designPxToRpx(paddingX)};padding-right:${designPxToRpx(paddingX)};`}
        >
            <View className="store-order-text">
                <View className="store-order-title">{t(item?.title || "")}</View>
                <View className="store-order-desc">{t(item?.desc || "")}</View>
            </View>
            <View className="store-order-img" style={imgMargin}>
                {src ? <Image src={imageFormat(src)} mode="aspectFit" /> : <View className="store-order-img-placeholder" />}
            </View>
        </View>
    );
}

/**
 * 点单卡片装修模块（type=storeOrderCard）。
 * 契约与渲染形态以原版为准：数据 = 原版编辑器 mobilePro/storeOrderCard
 * （picType 模板 1~6 / cardStyle 图文·图片 / boxStyle 1~4 / cardList 按
 * LENGTH_MAP 补空卡）；原版 uniapp v5.8.61 快照中无此模块的前端组件
 * （customPage、shopModules、会员装修三个分发器均未注册），故布局细节
 * 对齐原版编辑器 Preview.vue（与我方编辑器 Preview.tsx 逐项等价）。
 * 旧实现是「我的订单状态卡」，订单语义已由会员装修 orderEntry 承担，整段移除。
 * 2026-11-21 补衔接：店铺装修分发器注入 shopId，未配链接的卡片点击跳本店
 * 完整点单页 pages/shop/quickOrder（帖 127834 点单模式），配了链接仍走链接。
 */
export default function StoreOrderCard({ module, shopId }: Props) {
    const d = useMemo<StoreOrderCardData>(() => {
        const data = module?.data && typeof module.data === "object" ? module.data : {};
        return { ...(module ?? {}), ...data };
    }, [module]);

    const frame = useMemo(() => normalizeFrame(d.frame), [d.frame]);
    const picType = Number(d.picType) || 2;
    const cardStyle = Number(d.cardStyle) || 1;
    const boxStyle = Number(d.boxStyle) || 1;
    const gap = numOr(d.cardGap, 8);

    // cardList 按模板张数补空卡占位（渲染期补齐，不回写数据；超出部分同原版不截断）
    const cardList = useMemo(() => {
        const list: StoreOrderCardItemData[] = Array.isArray(d.cardList) ? [...d.cardList] : [];
        const expect = LENGTH_MAP[picType] || 2;
        while (list.length < expect) list.push({ ...EMPTY_CARD });
        return list;
    }, [d.cardList, picType]);

    const wrapStyle =
        `padding-left:${designPxToRpx(frame.boxPadding)};` +
        `padding-right:${designPxToRpx(frame.boxPadding)};` +
        `margin-top:${designPxToRpx(frame.boxPaddingTop)};` +
        `margin-bottom:${designPxToRpx(frame.boxPaddingBottom)};` +
        (d.backgroundColor ? `background-color:${d.backgroundColor};` : "") +
        `--card-gap:${designPxToRpx(gap)};`;

    const cardStyleStr =
        `border-radius:${designPxToRpx(frame.boxRadius)};` +
        (boxStyle === 4
            ? "background-color:transparent;"
            : boxStyle === 1
              ? "background-color:#ffffff;"
              : `background-color:${d.cardBackground || "#ffffff"};`) +
        (boxStyle === 2 ? "box-shadow:0 4rpx 16rpx rgba(0, 0, 0, 0.08);" : "") +
        (boxStyle === 3 ? "border:1px solid #E5E5E5;" : "");

    const renderItem = (item: StoreOrderCardItemData, size: CardSize, key: string, extraClass = "") => (
        <View
            className={`store-order-item ${extraClass}`}
            style={cardStyleStr}
            key={key}
            onClick={() => {
                const url = resolveCardUrl(item?.link);
                if (url) {
                    redirect({ url });
                    return;
                }
                // 点单卡片衔接完整点单模式（帖 127834）：店铺页内未配链接的
                // 卡片作为点单入口，跳本店点单页（分类+商品+购物车+下单一体）。
                if (shopId && shopId > 0) {
                    redirect({ url: `/pages/shop/quickOrder?shopId=${shopId}` });
                }
            }}
        >
            <CardBox item={item} cardStyle={cardStyle} size={size} frame={frame} />
        </View>
    );

    return (
        <View className={`module-ad-con module-store-order-card pic-type-${picType} card-style-${cardStyle} box-style-${boxStyle}`} style={wrapStyle}>
            {picType === 3 ? (
                <View className="store-order-row layout-3">
                    <View className="store-order-col store-order-col-left">{renderItem(cardList[0], "medium", "l0", "item-large")}</View>
                    <View className="store-order-col store-order-col-right">
                        {renderItem(cardList[1], "small", "r1")}
                        {renderItem(cardList[2], "small", "r2")}
                    </View>
                </View>
            ) : picType === 6 ? (
                <>
                    <View className="store-order-row layout-6-top">{cardList.slice(0, 2).map((item, i) => renderItem(item, "medium", `t-${i}`))}</View>
                    <View className="store-order-row layout-6-bot">{cardList.slice(2, 5).map((item, i) => renderItem(item, "small", `b-${i}`))}</View>
                </>
            ) : (
                <View className={`store-order-row grid-cols-${COLS_MAP[picType] || 2}`}>
                    {cardList.map((item, i) => renderItem(item, picType === 1 ? "large" : picType === 2 ? "medium" : "small", `g-${i}`))}
                </View>
            )}
        </View>
    );
}
