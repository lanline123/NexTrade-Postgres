/**
 * 点单卡片 预览（对齐 view/admin mobilePro/storeOrderCard/Preview.vue）
 * - frame 归一口径同原版：缺失时以 defaultFrame+moduleFrameDefault 补齐，历史数据仅有
 *   innerPadding 时回填 innerPaddingLeftRight/TopBottom（渲染期计算，不写回数据）；
 * - cardList 按 LENGTH_MAP[picType] 补空卡渲染（同原版兜底语义，渲染期补齐）；
 * - 布局：picType 3 左一右二、6 两行五个（上 2 中卡 + 下 3 小卡）、其余单行等分栅格
 *   （1→large、2→medium、余→small）；卡片盒样式按 boxStyle 1~4 口径；
 * - CardBox 两态：cardStyle=2 图片模式（整卡图，图片边距叠加到内容内边距）、
 *   否则图文模式（column-reverse：图在上文在下；size-large 横排）。
 */
import "./Preview.css";
import { useMemo } from "react";
import { mergeDefaultModule, cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";
import { cssText, productImage } from "../../../shared/format";
import { storeOrderCardDefaults, moduleFrameDefault, LENGTH_MAP, buildDefaultCard } from "./index";

interface Props {
    data: Record<string, any>;
}

const COLS_MAP: Record<number, number> = { 1: 1, 2: 2, 4: 3, 5: 4 };

function CardBox({ item, cardStyle, size, frame }: { item: Record<string, any>; cardStyle: number; size: string; frame: Record<string, any> }) {
    const hasPic = !!(item?.picUrl || item?.picThumb);
    const paddingX = frame.innerPaddingLeftRight ?? frame.innerPadding ?? 0;
    const paddingY = frame.innerPaddingTopBottom ?? frame.innerPadding ?? 0;
    const imgTop = Number(frame.imagePaddingTop) || 0;
    const imgBottom = Number(frame.imagePaddingBottom) || 0;
    const imgLeftRight = Number(frame.imagePaddingLeftRight) || 0;
    if (cardStyle === 2) {
        return (
            <div
                className={`soc-card-content soc-mode-image soc-size-${size}`}
                style={{ padding: `${paddingY + imgTop}px ${paddingX + imgLeftRight}px ${paddingY + imgBottom}px` }}
            >
                {hasPic ? <img className="soc-full-img" src={productImage(item)} alt="" /> : <div className="soc-empty-img" />}
            </div>
        );
    }
    return (
        <div className={`soc-card-content soc-mode-textimg soc-size-${size}`} style={{ padding: `${paddingY}px ${paddingX}px` }}>
            <div className="soc-text">
                <div className="soc-title">{item?.title || ""}</div>
                <div className="soc-desc">{item?.desc || ""}</div>
            </div>
            <div
                className="soc-img"
                style={{ marginTop: `${imgTop}px`, marginBottom: `${imgBottom}px`, marginLeft: `${imgLeftRight}px`, marginRight: `${imgLeftRight}px` }}
            >
                {hasPic ? <img src={productImage(item)} alt="" /> : <div className="soc-img-placeholder" />}
            </div>
        </div>
    );
}

export default function StoreOrderCardPreview({ data }: Props) {
    const m = useMemo(() => { const merged = JSON.parse(JSON.stringify(data ?? {})); mergeDefaultModule(merged, cloneDeep(storeOrderCardDefaults)); return merged; }, [data]);
    const frame = useMemo(() => {
        const base: Record<string, any> = { ...defaultFrame, ...moduleFrameDefault };
        const f: Record<string, any> = m.frame && typeof m.frame === "object" ? { ...base, ...m.frame } : base;
        if (f.innerPaddingLeftRight === undefined || f.innerPaddingLeftRight === null) f.innerPaddingLeftRight = f.innerPadding ?? 10;
        if (f.innerPaddingTopBottom === undefined || f.innerPaddingTopBottom === null) f.innerPaddingTopBottom = f.innerPadding ?? 10;
        return f;
    }, [m.frame]);

    const picType = Number(m.picType) || 2;
    const cardList = useMemo(() => {
        const list: Array<Record<string, any>> = Array.isArray(m.cardList) ? [...m.cardList] : [];
        const expect = LENGTH_MAP[picType] || 2;
        while (list.length < expect) list.push(buildDefaultCard());
        return list;
    }, [m.cardList, picType]);

    const gap = m.cardGap ?? 8;
    const wrapStyle: React.CSSProperties = {
        backgroundColor: m.backgroundColor || "",
        paddingLeft: `${frame.boxPadding ?? 10}px`,
        paddingRight: `${frame.boxPadding ?? 10}px`,
        marginTop: `${frame.boxPaddingTop ?? 5}px`,
        marginBottom: `${frame.boxPaddingBottom ?? 5}px`,
        ["--soc-card-gap" as any]: `${gap}px`
    };
    const boxStyle = Number(m.boxStyle) || 1;
    const cardStyleObj: React.CSSProperties = { borderRadius: `${frame.boxRadius ?? 0}px` };
    if (boxStyle === 4) cardStyleObj.backgroundColor = "transparent";
    else if (boxStyle === 1) cardStyleObj.backgroundColor = "#ffffff";
    else cardStyleObj.backgroundColor = m.cardBackground || "#ffffff";
    if (boxStyle === 2) cardStyleObj.boxShadow = "0 2px 8px rgba(0, 0, 0, 0.08)";
    if (boxStyle === 3) cardStyleObj.border = "1px solid #E5E5E5";

    const cardStyle = Number(m.cardStyle) || 1;
    const renderItem = (item: Record<string, any>, size: string, key: string, extraClass = "") => (
        <div className={`soc-item ${extraClass}`} style={cardStyleObj} key={key}>
            <CardBox item={item} cardStyle={cardStyle} size={size} frame={frame} />
        </div>
    );

    return (
        <div className="soc-root" style={wrapStyle}>
            {picType === 3 ? (
                <div className="soc-row soc-layout-3">
                    <div className="soc-col soc-col-left">{renderItem(cardList[0], "medium", "l0", "soc-item-large")}</div>
                    <div className="soc-col soc-col-right">
                        {renderItem(cardList[1], "small", "r1")}
                        {renderItem(cardList[2], "small", "r2")}
                    </div>
                </div>
            ) : picType === 6 ? (
                <>
                    <div className="soc-row soc-layout-6-top">{cardList.slice(0, 2).map((item, i) => renderItem(item, "medium", `t-${i}`))}</div>
                    <div className="soc-row soc-layout-6-bot">{cardList.slice(2, 5).map((item, i) => renderItem(item, "small", `b-${i}`))}</div>
                </>
            ) : (
                <div className={`soc-row soc-grid-cols-${COLS_MAP[picType] || 2}`}>
                    {cardList.map((item, i) => renderItem(item, picType === 1 ? "large" : picType === 2 ? "medium" : "small", `g-${i}`))}
                </div>
            )}
        </div>
    );
}
