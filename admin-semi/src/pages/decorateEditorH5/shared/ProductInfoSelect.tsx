/**
 * 商品选择（快照式选品，移植自 view/admin components/decorate/src/ProductInfoSelect.vue）
 *
 * 与 ProductsSourceField 的区别：那套写 productIds（来源式），本组件写「商品快照数组」
 * 本身（productList / pruductList 等字段），与原版 ProductInfoSelect 的 v-model:productList
 * 契约一致——数组元素为商品记录对象，核心字段 productId / productSn / productName /
 * productDesc / productPrice / marketPrice / picThumb（演示商品 productId=0，其余字段同构）。
 *
 * 能力（对齐原版）：已选列表平铺缩略图、可删除（原版 del 按下标 splice）、可拖拽排序
 * （原版 vuedraggable，此处以 HTML5 拖拽等价承接）、底部「添加商品」打开选品弹窗
 * （共享 ProductPickerModal，与 PC 编辑器同一套选品 UI）。
 *
 * 确认回填口径：原版 onAdd 以 getProductList({ ids, size: 100, page: 1 }) 拉取最新商品
 * 记录并写入快照——此处同口径拉取（接口失败时回退弹窗带回的行）。弹窗回传的是全量
 * 选中 ids：已在列表中的真实商品复用既有快照对象（不无故改写存量数据），新选商品写入
 * 最新记录，未勾选的真实商品随之移除（即「更换」）；演示商品（productId 非正数）弹窗
 * 无法表达，原位保留，只能手动删除——与原版 ids 过滤 productId!==0 的行为一致。
 */
import { useState } from "react";
import { IconPlus } from "@douyinfe/semi-icons";
import { Toast } from "@douyinfe/semi-ui";
import { apiGet } from "../../../api";
import { useI18n } from "../../../i18n";
import ProductPickerModal from "../../../components/ProductPickerModal";
import { imageFormat, pickPic } from "./format";

export interface ProductInfoSelectProps {
    /** 商品快照数组（即模块数据中的 productList / pruductList 字段值） */
    value: Array<Record<string, any>>;
    onChange: (next: Array<Record<string, any>>) => void;
    /** 是否多选（原版四处调用均为 true）；单选时确认后真实商品只保留所选一件 */
    isMultiple?: boolean;
    /** 最多可选数量，0/不传 = 不限（原版 maxSelection 默认 0） */
    maxSelection?: number;
}

const isDemoItem = (item: Record<string, any>) => !(Number(item?.productId) > 0);

export default function ProductInfoSelect({ value, onChange, isMultiple = true, maxSelection = 0 }: ProductInfoSelectProps) {
    const { t } = useI18n();
    const list = Array.isArray(value) ? value : [];
    const [pickerVisible, setPickerVisible] = useState(false);
    const [busy, setBusy] = useState(false);
    const [dragIndex, setDragIndex] = useState<number | null>(null);
    const [overIndex, setOverIndex] = useState<number | null>(null);

    const selectedIds = list.filter((item) => !isDemoItem(item)).map((item) => Number(item.productId));

    const removeAt = (index: number) => onChange(list.filter((_, i) => i !== index));

    const reorder = (from: number, to: number) => {
        if (from === to) return;
        const next = [...list];
        const [moved] = next.splice(from, 1);
        next.splice(to, 0, moved);
        onChange(next);
    };

    /** 按原版 onAdd 口径拉取最新商品记录；失败/为空时回退弹窗带回的行 */
    const fetchSnapshots = async (ids: number[], fallbackRows: Array<Record<string, any>>): Promise<Map<number, Record<string, any>>> => {
        const byId = new Map<number, Record<string, any>>();
        try {
            const result = await apiGet<{ records?: Array<Record<string, any>>; list?: Array<Record<string, any>> }>("/product/product/list", {
                ids: ids.join(","),
                size: 100,
                page: 1
            });
            (result.records || result.list || []).forEach((row) => {
                const id = Number(row?.productId);
                if (Number.isFinite(id) && id > 0) byId.set(id, row);
            });
        } catch {
            // 拉取失败走 fallbackRows 兜底
        }
        if (byId.size === 0) {
            fallbackRows.forEach((row) => {
                const id = Number(row?.productId);
                if (Number.isFinite(id) && id > 0) byId.set(id, row as Record<string, any>);
            });
        }
        return byId;
    };

    const handleOk = async (ids: number[], rows: Array<Record<string, any>>) => {
        setPickerVisible(false);
        if (ids.length === 0) {
            // 全部取消勾选：仅保留演示商品（真实商品被更换为空）
            onChange(list.filter(isDemoItem));
            return;
        }
        setBusy(true);
        try {
            const freshById = await fetchSnapshots(ids, rows);
            const existingById = new Map<number, Record<string, any>>();
            list.forEach((item) => {
                if (!isDemoItem(item)) existingById.set(Number(item.productId), item);
            });
            // 真实商品段按确认顺序重建：已存在的复用既有快照，新选的写最新记录
            const realQueue: Array<Record<string, any>> = [];
            ids.forEach((id) => {
                const hit = existingById.get(id) ?? freshById.get(id);
                if (hit) realQueue.push(hit);
            });
            if (!isMultiple && realQueue.length > 1) realQueue.splice(1);
            // 演示商品原位保留，真实商品按原槽位顺序填入，多出的追加到末尾
            const next: Array<Record<string, any>> = [];
            list.forEach((item) => {
                if (isDemoItem(item)) next.push(item);
                else if (realQueue.length > 0) next.push(realQueue.shift() as Record<string, any>);
            });
            next.push(...realQueue);
            onChange(next);
        } catch {
            Toast.error(t("商品信息加载失败，请重试"));
        } finally {
            setBusy(false);
        }
    };

    return (
        <div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
                {list.map((item, index) => {
                    const pic = pickPic(item);
                    return (
                        <div
                            key={index}
                            style={{ width: 60, textAlign: "center", opacity: dragIndex === index ? 0.5 : 1 }}
                            title={item.productName ?? ""}
                            draggable
                            onDragStart={() => setDragIndex(index)}
                            onDragOver={(event) => {
                                if (dragIndex === null) return;
                                event.preventDefault();
                                if (overIndex !== index) setOverIndex(index);
                            }}
                            onDrop={(event) => {
                                event.preventDefault();
                                if (dragIndex !== null) reorder(dragIndex, index);
                                setDragIndex(null);
                                setOverIndex(null);
                            }}
                            onDragEnd={() => {
                                setDragIndex(null);
                                setOverIndex(null);
                            }}
                        >
                            <div
                                style={{
                                    position: "relative",
                                    width: 60,
                                    height: 60,
                                    border: overIndex === index && dragIndex !== null && dragIndex !== index ? "1px solid #155bd4" : "1px solid #e5e5e5",
                                    background: "#f9f9f9",
                                    borderRadius: 2,
                                    overflow: "visible",
                                    cursor: "grab"
                                }}
                            >
                                <div style={{ width: "100%", height: "100%", overflow: "hidden", borderRadius: 2 }}>
                                    {pic ? <img src={imageFormat(pic)} alt="" style={{ width: "100%", height: "100%", objectFit: "contain" }} /> : null}
                                </div>
                                <button
                                    type="button"
                                    aria-label={t("删除")}
                                    onClick={(event) => {
                                        event.stopPropagation();
                                        removeAt(index);
                                    }}
                                    style={{
                                        position: "absolute",
                                        top: -7,
                                        right: -7,
                                        width: 16,
                                        height: 16,
                                        padding: 0,
                                        border: "none",
                                        borderRadius: "50%",
                                        background: "rgba(0, 0, 0, 0.55)",
                                        color: "#fff",
                                        fontSize: 11,
                                        lineHeight: "16px",
                                        textAlign: "center",
                                        cursor: "pointer",
                                        zIndex: 2
                                    }}
                                >
                                    ×
                                </button>
                            </div>
                            <div style={{ fontSize: 11, color: "#666", marginTop: 4, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                                {item.productName || `商品${index + 1}`}
                            </div>
                        </div>
                    );
                })}
                <div style={{ width: 60, textAlign: "center" }}>
                    <button
                        type="button"
                        disabled={busy}
                        onClick={() => setPickerVisible(true)}
                        style={{
                            width: 60,
                            height: 60,
                            border: "1px dashed #c8cdd4",
                            background: "#fff",
                            borderRadius: 2,
                            color: "#646a73",
                            cursor: busy ? "wait" : "pointer",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            fontSize: 18
                        }}
                        title={t("添加商品")}
                    >
                        <IconPlus />
                    </button>
                    <div style={{ fontSize: 11, color: "#666", marginTop: 4 }}>{t("添加商品")}</div>
                </div>
            </div>
            {pickerVisible && (
                <ProductPickerModal
                    visible
                    selectedIds={selectedIds}
                    multiple={isMultiple}
                    max={maxSelection > 0 ? maxSelection : undefined}
                    onOk={(ids, rows) => void handleOk(ids, rows as unknown as Array<Record<string, any>>)}
                    onCancel={() => setPickerVisible(false)}
                />
            )}
        </div>
    );
}
