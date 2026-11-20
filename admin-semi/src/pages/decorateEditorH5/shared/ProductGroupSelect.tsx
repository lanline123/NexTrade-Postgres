/**
 * 商品分组选择（共享，移植自 view/admin components/decorate/src/ProductGroupSelect.vue）
 *
 * 与 ProductInfoSelect 的区别：那套写「商品快照数组」，本组件写「商品分组条目数组」
 * （groupList / productGroups 等字段），与原版 ProductGroupSelect 的
 * v-model:productGroups 契约一致——数组元素核心字段 productGroupId /
 * productGroupName，外加每组可编辑的 menuTitle / productNumType / productNum，
 * 以及 type="productGroup" 时的 title / subTitle / bannerPic。
 *
 * 能力（对齐原版）：
 * - 已选分组以卡片平铺（原版 .dec-goods-group-item 同式：白底 / 2px 圆角 /
 *   0 0 4px 阴影），卡内字段以 { data, onChange } 子协议写回单组；
 * - 「商品来源」分组名是链接式按钮，点击打开分组选择弹窗换组——按原版
 *   onChange→loadFilter(index) 口径：只改 productGroupId / productGroupName /
 *   menuTitle（=新分组名），其余已调字段（title/productNum 等）原样保留；
 * - 卡片可拖拽排序（原版 vuedraggable，此处以 HTML5 拖拽等价承接，与
 *   ProductInfoSelect 同机制）、可删除（徽标形态从本编辑器 GroupCardList
 *   现行口径：直角 / 常显，不用原版圆形 hover 徽标）；
 * - 「添加商品分组」走共享 AddItemButton，仅在 isMultiple 或列表为空时出现
 *   （原版 v-if 口径逐字）；确认后按原版 loadFilter 形状新建条目：
 *   { productGroupId, productGroupName, title: 分组名, subTitle: "",
 *     menuTitle: 分组名, productNumType: "number",
 *     productNum: 分组商品数 || 6, bannerPic: {}, bannerLink: { link: {}, title: "" } }。
 *
 * 数据源：GET /product/productGroup/list（与后台商品分组管理页同一接口，
 * 支持 productGroupIds 逗号串 IN 过滤 / keyword 名称模糊 / 分页），与原版
 * getProductGroupList({ productGroupIds, size, page }) 同口径；接口失败时
 * 回退弹窗带回的行。
 *
 * 我方口径（与原版的两点有意差异，均为防数据损坏）：
 * - 添加时已在列表中的分组在弹窗内锁定（已添加不可重复勾选），确认只追加
 *   新分组——原版靠其 DialogForm 内部行为防重，契约不可见，此处显式防重；
 * - 按 ids 回拉记录时 size 取 max(ids 数, 6)（上限 100），不用原版写死的
 *   size=6，避免一次选超过 6 个分组时静默截断。
 *
 * 消费方：productGrouping 来源组（groupList，isMultiple + type="productGroup"
 * + showCustomImage）、specialOffer（productGroups，全默认）。
 */
import { useEffect, useRef, useState } from "react";
import { IconEdit } from "@douyinfe/semi-icons";
import { Button, Checkbox, Input, InputNumber, Modal, Pagination, Radio, Space, Spin, Toast, Typography } from "@douyinfe/semi-ui";
import { apiGet } from "../../../api";
import { useI18n } from "../../../i18n";
import { SettingRow, RadioRowField, TextField, PicSelectField } from "./settingWidgets";
import AddItemButton from "./AddItemButton";

const { Text } = Typography;

export interface ProductGroupSelectProps {
    /** 分组条目数组（即模块数据中的 groupList / productGroups 字段值） */
    value: Array<Record<string, any>>;
    onChange: (next: Array<Record<string, any>>) => void;
    /** 是否可多组（原版默认 false；false 时仅列表为空可添加，specialOffer 即此口径） */
    isMultiple?: boolean;
    /** "productGroup" 时多出主标题/副标题行（原版 type 口径，默认 "goods"） */
    type?: string;
    /** 是否启用「自定义图片」设置项（原版默认 false，仅 productGrouping 开启） */
    showCustomImage?: boolean;
}

interface GroupRow {
    productGroupId: number;
    productGroupName?: string;
    productGroupSn?: string;
    productGroupDescription?: string;
    productIds?: number[];
}

type PickerState = { mode: "add" } | { mode: "replace"; index: number } | null;

/** 分组选择弹窗（内部件）：搜索 + 分页 + 单/多选；add 模式下已在列表的分组锁定 */
function ProductGroupPickerModal({
    visible,
    multiple,
    lockedIds,
    seedId,
    onOk,
    onCancel
}: {
    visible: boolean;
    multiple: boolean;
    /** add 模式：已在列表中的分组 id（锁定不可重复选） */
    lockedIds: number[];
    /** replace 模式：当前组 id（初始选中） */
    seedId?: number;
    onOk: (ids: number[], rows: GroupRow[]) => void;
    onCancel: () => void;
}) {
    const { t } = useI18n();
    const [keyword, setKeyword] = useState("");
    const [rows, setRows] = useState<GroupRow[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(false);
    const [picked, setPicked] = useState<number[]>([]);
    const rowsByIdRef = useRef<Map<number, GroupRow>>(new Map());

    useEffect(() => {
        if (visible) {
            setPicked(seedId ? [seedId] : [...lockedIds]);
            setKeyword("");
            setPage(1);
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [visible]);

    const load = (nextPage = page, nextKeyword = keyword) => {
        setLoading(true);
        apiGet<{ records?: GroupRow[]; list?: GroupRow[]; total?: number }>("/product/productGroup/list", {
            keyword: nextKeyword || undefined,
            page: nextPage,
            size: 20
        })
            .then((res) => {
                const list = res.records || res.list || [];
                list.forEach((row) => rowsByIdRef.current.set(Number(row.productGroupId), row));
                setRows(list);
                setTotal(Number(res.total ?? list.length));
            })
            .catch(() => setRows([]))
            .finally(() => setLoading(false));
    };

    useEffect(() => {
        if (visible) load(1, "");
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [visible]);

    const toggle = (row: GroupRow) => {
        const id = Number(row.productGroupId);
        if (lockedIds.includes(id)) return;
        rowsByIdRef.current.set(id, row);
        setPicked((prev) => {
            if (prev.includes(id)) return prev.filter((x) => x !== id);
            if (!multiple) return [id];
            return [...prev, id];
        });
    };

    const newCount = picked.filter((id) => !lockedIds.includes(id)).length;

    return (
        <Modal
            title={t("选择商品分组")}
            visible={visible}
            width={600}
            onOk={() =>
                onOk(
                    picked,
                    picked.map((id) => rowsByIdRef.current.get(id)).filter((row): row is GroupRow => !!row)
                )
            }
            onCancel={onCancel}
            okText={t("确定")}
            cancelText={t("取消")}
        >
            <Space style={{ marginBottom: 12, width: "100%" }} align="center">
                <Input
                    value={keyword}
                    onChange={setKeyword}
                    onEnterPress={() => {
                        setPage(1);
                        load(1);
                    }}
                    placeholder={t("输入分组名称搜索")}
                    style={{ width: 260 }}
                    showClear
                    onClear={() => {
                        setKeyword("");
                        setPage(1);
                        load(1, "");
                    }}
                />
                <Button
                    onClick={() => {
                        setPage(1);
                        load(1);
                    }}
                >
                    {t("搜索")}
                </Button>
                <span style={{ fontSize: 12, color: "#646a73" }}>
                    {multiple ? t("已选 {count} 个", { count: newCount }) : ""}
                </span>
            </Space>
            <div style={{ minHeight: 240 }}>
                {loading ? (
                    <div style={{ textAlign: "center", paddingTop: 90 }}>
                        <Spin />
                    </div>
                ) : rows.length === 0 ? (
                    <Text type="tertiary">{t("暂无数据")}</Text>
                ) : (
                    rows.map((row) => {
                        const id = Number(row.productGroupId);
                        const locked = lockedIds.includes(id);
                        const active = picked.includes(id);
                        return (
                            <div
                                key={id}
                                onClick={() => toggle(row)}
                                style={{
                                    display: "flex",
                                    alignItems: "center",
                                    gap: 10,
                                    padding: "9px 10px",
                                    borderBottom: "1px solid #f0f1f3",
                                    cursor: locked ? "not-allowed" : "pointer",
                                    background: active && !locked ? "#f2f7ff" : undefined
                                }}
                            >
                                {multiple ? (
                                    <Checkbox checked={active} disabled={locked} />
                                ) : (
                                    <Radio checked={active} disabled={locked} />
                                )}
                                <span style={{ fontSize: 13, color: "#1c1f23" }}>{row.productGroupName || `分组${id}`}</span>
                                {row.productGroupSn ? (
                                    <span style={{ fontSize: 12, color: "#999" }}>{row.productGroupSn}</span>
                                ) : null}
                                <span style={{ fontSize: 12, color: "#999", marginLeft: "auto" }}>
                                    {t("{count} 个商品", { count: Array.isArray(row.productIds) ? row.productIds.length : 0 })}
                                </span>
                                {locked ? (
                                    <span style={{ fontSize: 12, color: "#fa8c16" }}>{t("已添加")}</span>
                                ) : null}
                            </div>
                        );
                    })
                )}
            </div>
            {total > 20 ? (
                <div style={{ textAlign: "right", marginTop: 12 }}>
                    <Pagination
                        currentPage={page}
                        pageSize={20}
                        total={total}
                        onPageChange={(p) => {
                            setPage(p);
                            load(p);
                        }}
                        size="small"
                    />
                </div>
            ) : null}
        </Modal>
    );
}

/** 按原版 loadFilter 口径以 ids 回拉分组记录；失败/缺失时回退弹窗带回的行 */
async function fetchGroupRows(ids: number[], fallbackRows: GroupRow[]): Promise<Map<number, GroupRow>> {
    const byId = new Map<number, GroupRow>();
    try {
        const result = await apiGet<{ records?: GroupRow[]; list?: GroupRow[] }>("/product/productGroup/list", {
            productGroupIds: ids.join(","),
            size: Math.min(100, Math.max(ids.length, 6)),
            page: 1
        });
        (result.records || result.list || []).forEach((row) => {
            const id = Number(row?.productGroupId);
            if (Number.isFinite(id) && id > 0) byId.set(id, row);
        });
    } catch {
        // 拉取失败走 fallbackRows 兜底
    }
    fallbackRows.forEach((row) => {
        const id = Number(row?.productGroupId);
        if (Number.isFinite(id) && id > 0 && !byId.has(id)) byId.set(id, row);
    });
    return byId;
}

/** 原版 loadFilter 新增条目形状（逐字段） */
function buildGroupItem(row: GroupRow): Record<string, any> {
    return {
        productGroupId: Number(row.productGroupId),
        productGroupName: row.productGroupName ?? "",
        title: row.productGroupName ?? "",
        subTitle: "",
        menuTitle: row.productGroupName ?? "",
        productNumType: "number",
        productNum: Array.isArray(row.productIds) && row.productIds.length ? row.productIds.length : 6,
        bannerPic: {},
        bannerLink: { link: {}, title: "" }
    };
}

export default function ProductGroupSelect({
    value,
    onChange,
    isMultiple = false,
    type = "goods",
    showCustomImage = false
}: ProductGroupSelectProps) {
    const { t } = useI18n();
    const list: Array<Record<string, any>> = Array.isArray(value) ? value : [];
    const [picker, setPicker] = useState<PickerState>(null);
    const [busy, setBusy] = useState(false);
    const [dragIndex, setDragIndex] = useState<number | null>(null);
    const [overIndex, setOverIndex] = useState<number | null>(null);

    const patchItem = (index: number, patch: Record<string, any>) =>
        onChange(list.map((g, i) => (i === index ? { ...g, ...patch } : g)));
    const removeAt = (index: number) => onChange(list.filter((_, i) => i !== index));
    const reorder = (from: number, to: number) => {
        if (from === to) return;
        const next = [...list];
        const [moved] = next.splice(from, 1);
        next.splice(to, 0, moved);
        onChange(next);
    };

    const existingIds = list.map((g) => Number(g?.productGroupId)).filter((id) => Number.isFinite(id) && id > 0);

    const handleOk = async (ids: number[], rows: GroupRow[]) => {
        const state = picker;
        setPicker(null);
        if (!state) return;
        if (state.mode === "replace") {
            const id = ids.find((x) => x > 0);
            if (!id || id === Number(list[state.index]?.productGroupId)) return;
            setBusy(true);
            try {
                const byId = await fetchGroupRows([id], rows);
                const row = byId.get(id);
                if (!row) {
                    Toast.error(t("分组信息加载失败，请重试"));
                    return;
                }
                // 原版换组口径：只改 id / 名称 / 菜单名称，其余已调字段保留
                patchItem(state.index, {
                    productGroupId: Number(row.productGroupId),
                    productGroupName: row.productGroupName ?? "",
                    menuTitle: row.productGroupName ?? ""
                });
            } finally {
                setBusy(false);
            }
            return;
        }
        // add：只追加未在列表中的新分组（防重口径见文件头）
        const newIds = ids.filter((id) => id > 0 && !existingIds.includes(id));
        if (!isMultiple && newIds.length > 1) newIds.splice(1);
        if (newIds.length === 0) return;
        setBusy(true);
        try {
            const byId = await fetchGroupRows(newIds, rows);
            const appended: Array<Record<string, any>> = [];
            newIds.forEach((id) => {
                const row = byId.get(id);
                if (row) appended.push(buildGroupItem(row));
            });
            if (appended.length === 0) {
                Toast.error(t("分组信息加载失败，请重试"));
                return;
            }
            onChange([...list, ...appended]);
        } finally {
            setBusy(false);
        }
    };

    const showAdd = isMultiple || list.length < 1;

    return (
        <div>
            {list.length === 0 ? (
                <div style={{ color: "#999", fontSize: 12, lineHeight: 1.6 }}>{t("暂无已选商品分组")}</div>
            ) : null}
            {list.map((group, index) => {
                const sub = { data: group, onChange: (patch: Record<string, any>) => patchItem(index, patch) };
                return (
                    <div
                        key={index}
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
                        style={{
                            position: "relative",
                            backgroundColor: "#fff",
                            borderRadius: 2,
                            boxShadow: "0 0 4px 0 rgba(10, 42, 97, 0.2)",
                            padding: "6px 0",
                            marginTop: 12,
                            opacity: dragIndex === index ? 0.5 : 1,
                            outline: overIndex === index && dragIndex !== null && dragIndex !== index ? "1px solid #155bd4" : undefined
                        }}
                    >
                        <SettingRow label="商品来源">
                            <button
                                type="button"
                                disabled={busy}
                                onClick={() => setPicker({ mode: "replace", index })}
                                style={{
                                    background: "none",
                                    border: "none",
                                    padding: 0,
                                    fontSize: 13,
                                    color: "#155bd4",
                                    cursor: busy ? "wait" : "pointer",
                                    display: "inline-flex",
                                    alignItems: "center",
                                    gap: 4
                                }}
                                title={t("更换商品分组")}
                            >
                                {group.productGroupName || `分组${group.productGroupId ?? ""}`}
                                <IconEdit />
                            </button>
                        </SettingRow>
                        <TextField {...sub} field="menuTitle" label="菜单名称" placeholder="请输入菜单名称" />
                        <RadioRowField
                            {...sub}
                            field="productNumType"
                            label="商品个数"
                            options={[
                                { value: "all", label: "全部" },
                                { value: "number", label: "指定数量" }
                            ]}
                        />
                        {group.productNumType === "number" ? (
                            <SettingRow label="商品数量">
                                <InputNumber
                                    value={Number(group.productNum ?? 6)}
                                    min={1}
                                    style={{ width: 120 }}
                                    onChange={(v: any) => patchItem(index, { productNum: Number(v) || 0 })}
                                />
                            </SettingRow>
                        ) : null}
                        {type === "productGroup" ? (
                            <>
                                <TextField {...sub} field="title" label="主标题" placeholder="请输入标题内容" />
                                <TextField {...sub} field="subTitle" label="副标题" placeholder="请输入标题内容" />
                                {showCustomImage ? <PicSelectField {...sub} field="bannerPic" label="自定义图片" /> : null}
                            </>
                        ) : null}
                        <span
                            title={t("删除")}
                            onClick={() => removeAt(index)}
                            style={{
                                position: "absolute",
                                right: -10,
                                top: -10,
                                color: "#fff",
                                background: "#bbb",
                                border: "2px solid #fff",
                                borderRadius: 0,
                                zIndex: 2,
                                width: 18,
                                height: 18,
                                textAlign: "center",
                                lineHeight: "16px",
                                cursor: "pointer",
                                fontSize: 12
                            }}
                        >
                            ×
                        </span>
                    </div>
                );
            })}
            {showAdd ? (
                <AddItemButton
                    onClick={() => {
                        if (!busy) setPicker({ mode: "add" });
                    }}
                >
                    {t("添加商品分组")}
                </AddItemButton>
            ) : null}
            {picker && (
                <ProductGroupPickerModal
                    visible
                    multiple={picker.mode === "add" ? isMultiple : false}
                    lockedIds={picker.mode === "add" ? existingIds : []}
                    seedId={picker.mode === "replace" ? Number(list[picker.index]?.productGroupId) || undefined : undefined}
                    onOk={(ids, rows) => void handleOk(ids, rows)}
                    onCancel={() => setPicker(null)}
                />
            )}
        </div>
    );
}
