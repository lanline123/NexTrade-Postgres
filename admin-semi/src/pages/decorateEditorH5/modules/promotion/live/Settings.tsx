/**
 * 小程序直播 设置（对齐 view/admin live/Settings.vue）
 *
 * 顶部说明 tip + 排列方式（list 列表模式/bigImage 大图模式）+ 原版自有下划线页签（内容设置/样式设置）。
 * 内容：直播内容（auto 自动/manual 手动）；manual 时已选直播间列表（封面 72×72、标题、房间号、删除；
 * 原版 vuedraggable 拖拽排序，本实现以 HTML5 拖拽等价承接——同编辑器 ProductInfoSelect 先例，拖拽手柄
 * 为原版三点 handle，数据结构不变）+「添加直播」打开选择弹窗（原版 DialogForm 抽屉 SelectLive 数据
 * 选择器：关键词搜索、表格三列——直播间信息（封面 56×56 + 标题 + 主播名）/房间号/直播状态、分页、
 * 跨页保留选择且提交顺序 = 选择顺序、超 10 个即时拦截并警告；本实现以 Semi Modal 960px + Table 承接
 * 同一功能契约，接口 /promotion/wechatLive/list）+ 未选空提示 + 底部说明；auto 时直播数量
 * autoLiveNum（原版 TigInput integer，宽 275、min 1/max 10，钳制在挂载 normalize 与预览侧，
 * 输入过程透传写回）。
 * 样式：CommonFrameEdit type="live"（无分隔线/无「模块设置」标题——FrameSettingsField hideHead 承接）。
 * 写回行为对齐原版 syncSelectedLiveList：挂载时先 normalizeAutoLiveNum 并以当前 selectedLiveList
 * 初始化 manual 镜像、拉取自动列表（过滤 liveStatus!==4、上限 10）；sourceType/autoLiveNum/自动列表
 * 任一变化即同步写回 selectedLiveList——auto 写自动列表前 autoLiveNum 条快照，manual 写 manual 镜像。
 * 写回带内容 diff 守卫（与当前值相同不重复写），避免与原版无条件赋值等价语义下产生无意义脏标记/循环。
 */
import { useEffect, useRef, useState } from "react";
import { Button, Input, InputNumber, Modal, Pagination, Spin, Table, Toast } from "@douyinfe/semi-ui";
import { apiGet } from "../../../../../api";
import { SettingGroup, SettingRow, FrameSettingsField, RadioRowField } from "../../../shared/settingWidgets";
import AddItemButton from "../../../shared/AddItemButton";

const TIP = "小程序直播组件为小程序直播插件的配套组件：通过该组件可以实现自定义进入小程序直播的直播间，帮助商户提升直播相关内容的曝光率，提升直播热度。";
const BOTTOM_TIP = "最多10个，可拖拽排序直播组件目前仅支持小程序，预览页仅提供样式规范直播封面图会自动读取官方收录直播卡片封面图";
/** 原版 SelectLive 分页 size 取 config pageSize（tigshop 默认 20） */
const PICKER_PAGE_SIZE = 20;
const MAX_LIVE = 10;

const roomIdOf = (r: any) => r?.wechatLiveId || r?.roomId;

export default function LiveSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const [tab, setTab] = useState<"content" | "style">("content");
    const sourceType = data?.sourceType ?? "auto";
    const selected: any[] = Array.isArray(data?.selectedLiveList) ? data.selectedLiveList : [];
    const [pickerOpen, setPickerOpen] = useState(false);
    const [loading, setLoading] = useState(false);
    const [rooms, setRooms] = useState<any[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [keywordInput, setKeywordInput] = useState("");
    const [keyword, setKeyword] = useState("");
    /** 弹窗内已选（完整行对象、选择顺序即提交顺序，跨页保留——同原版 SelectLive selectedRows） */
    const [pickerSelected, setPickerSelected] = useState<any[]>([]);
    const [dragIndex, setDragIndex] = useState<number | null>(null);
    const [overIndex, setOverIndex] = useState<number | null>(null);

    // ---- syncSelectedLiveList 写回（对齐原版 onMounted + watch([sourceType, autoLiveNum, autoLiveList], { deep: true })） ----
    const dataRef = useRef(data);
    dataRef.current = data;
    const onChangeRef = useRef(onChange);
    onChangeRef.current = onChange;
    const [autoLiveList, setAutoLiveList] = useState<any[]>([]);
    const manualListRef = useRef<any[]>([]);

    useEffect(() => {
        // normalizeAutoLiveNum：仅当已存值存在且归一化后不同才写回（新模块默认值 10 不触发）
        const raw = dataRef.current?.autoLiveNum;
        if (raw !== undefined && raw !== null && raw !== "") {
            const n = Number(raw || 10);
            const normalized = !n || n < 1 ? 1 : Math.min(n, 10);
            if (normalized !== Number(raw)) onChangeRef.current({ autoLiveNum: normalized });
        }
        // manual 镜像以打开时的 selectedLiveList 初始化（同原版 onMounted 顺序，在 sync 之前）
        manualListRef.current = (Array.isArray(dataRef.current?.selectedLiveList) ? dataRef.current.selectedLiveList : []).slice(0, 10);
        let alive = true;
        apiGet<any>("/promotion/wechatLive/list", { keyword: "", page: 1, size: 10 })
            .then((res: any) => {
                if (!alive) return;
                const rows = res?.records || res?.list || [];
                setAutoLiveList(rows.filter((item: any) => Number(item.liveStatus) !== 4).slice(0, 10));
            })
            .catch(() => {
                if (alive) setAutoLiveList([]);
            });
        return () => {
            alive = false;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    useEffect(() => {
        const current = Array.isArray(dataRef.current?.selectedLiveList) ? dataRef.current.selectedLiveList : [];
        let target: any[];
        if (sourceType === "manual") {
            target = manualListRef.current.slice(0, 10);
        } else {
            const n = Math.min(Math.max(Number(dataRef.current?.autoLiveNum || 10), 1), 10);
            target = autoLiveList.slice(0, Number.isNaN(n) ? 0 : n);
        }
        if (JSON.stringify(current) !== JSON.stringify(target)) {
            onChangeRef.current({ selectedLiveList: target });
        }
    }, [sourceType, data?.autoLiveNum, autoLiveList]);

    // ---- 选间弹窗（原版 SelectLive：搜索 + 表格 + 分页 + 跨页保留选择） ----
    useEffect(() => {
        if (!pickerOpen) return;
        let alive = true;
        setLoading(true);
        apiGet<any>("/promotion/wechatLive/list", { keyword, page, size: PICKER_PAGE_SIZE })
            .then((res: any) => {
                if (!alive) return;
                setRooms(res?.records || res?.list || []);
                setTotal(Number(res?.total ?? res?.count ?? 0));
            })
            .catch(() => {
                if (alive) {
                    setRooms([]);
                    setTotal(0);
                }
            })
            .finally(() => {
                if (alive) setLoading(false);
            });
        return () => {
            alive = false;
        };
    }, [pickerOpen, page, keyword]);

    const openPicker = () => {
        setPickerSelected(selected.slice(0, MAX_LIVE));
        setPage(1);
        setKeyword("");
        setKeywordInput("");
        setPickerOpen(true);
    };

    /** 同原版 updateSelectedRows：本页外已选保留、本页选择按表格序合并；单选超限拦截警告，全选超限截断 */
    const handlePickerSelect = (selectedRowKeys: any[], selectedRows: any[]) => {
        const currentPageIds = rooms.map(roomIdOf);
        const otherPageRows = pickerSelected.filter((r) => !currentPageIds.includes(roomIdOf(r)));
        const merged = [...otherPageRows, ...selectedRows];
        if (merged.length > MAX_LIVE) {
            const prevIds = pickerSelected.map(roomIdOf);
            const added = selectedRowKeys.filter((k) => !prevIds.includes(k));
            if (added.length === 1) {
                Toast.warning({ content: "最多只能选择10个直播间" });
                return;
            }
            setPickerSelected(merged.slice(0, MAX_LIVE));
            return;
        }
        setPickerSelected(merged);
    };

    const confirmPicker = () => {
        const result = pickerSelected.slice(0, MAX_LIVE);
        manualListRef.current = result;
        onChange({ selectedLiveList: result });
        setPickerOpen(false);
    };
    const removeAt = (index: number) => {
        const next = selected.filter((_, i) => i !== index);
        manualListRef.current = next;
        onChange({ selectedLiveList: next });
    };
    const reorder = (from: number, to: number) => {
        if (from === to) return;
        const next = [...selected];
        const [moved] = next.splice(from, 1);
        next.splice(to, 0, moved);
        manualListRef.current = next;
        onChange({ selectedLiveList: next });
    };

    const pickerColumns = [
        {
            title: "直播间信息",
            dataIndex: "wechatLiveTitle",
            render: (_: any, r: any) => (
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                    {r.feedsImg || r.coverImg ? (
                        <img src={r.feedsImg || r.coverImg} alt="直播封面" style={{ width: 56, height: 56, borderRadius: 4, objectFit: "cover", background: "#f2f3f5", flexShrink: 0 }} />
                    ) : (
                        <div style={{ width: 56, height: 56, borderRadius: 4, background: "#f2f3f5", flexShrink: 0 }} />
                    )}
                    <div style={{ minWidth: 0 }}>
                        <div style={{ color: "#333", fontWeight: 600, lineHeight: "22px", wordBreak: "break-all" }}>{r.wechatLiveTitle || "-"}</div>
                        <div style={{ color: "#999", fontSize: 12, lineHeight: "18px" }}>{r.anchorName || "-"}</div>
                    </div>
                </div>
            )
        },
        { title: "房间号", dataIndex: "roomId", width: 150, render: (v: any) => v ?? "-" },
        { title: "直播状态", dataIndex: "liveStatusText", width: 150, render: (v: any) => v || "-" }
    ];

    return (
        <div className="h5-editor-settings-body">
            <div style={{ color: "#999", fontSize: 12, lineHeight: "20px", padding: "0 0 12px" }}>{TIP}</div>
            <SettingGroup>
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="displayMode"
                    label="排列方式"
                    options={[
                        { value: "list", label: "列表模式" },
                        { value: "bigImage", label: "大图模式" }
                    ]}
                />
            </SettingGroup>
            <div style={{ display: "flex", margin: "10px 0 16px", borderBottom: "1px solid #ebeef5" }}>
                {(
                    [
                        { key: "content", label: "内容设置" },
                        { key: "style", label: "样式设置" }
                    ] as const
                ).map((t) => (
                    <div
                        key={t.key}
                        onClick={() => setTab(t.key)}
                        style={{
                            position: "relative",
                            flex: 1,
                            padding: "0 4px 12px",
                            color: tab === t.key ? "#155bd4" : "#999",
                            fontWeight: tab === t.key ? 600 : 400,
                            fontSize: 14,
                            cursor: "pointer",
                            textAlign: "center"
                        }}
                    >
                        {t.label}
                        {tab === t.key ? <span style={{ position: "absolute", left: 0, right: 0, bottom: -1, height: 3, background: "#155bd4" }} /> : null}
                    </div>
                ))}
            </div>
            {tab === "content" ? (
                <div>
                    <SettingGroup>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="sourceType"
                            label="直播内容"
                            options={[
                                { value: "auto", label: "自动" },
                                { value: "manual", label: "手动" }
                            ]}
                        />
                    </SettingGroup>
                    {sourceType === "manual" ? (
                        <>
                            <div style={{ display: "flex", flexDirection: "column", gap: 10, margin: "4px 0 12px" }}>
                                {selected.map((item, index) => (
                                    <div
                                        key={item.wechatLiveId || item.roomId || index}
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
                                            display: "flex",
                                            alignItems: "center",
                                            gap: 12,
                                            padding: 12,
                                            background: "#f7f8fa",
                                            borderRadius: 4,
                                            opacity: dragIndex === index ? 0.5 : 1,
                                            outline: overIndex === index && dragIndex !== null && dragIndex !== index ? "1px solid #155bd4" : "none"
                                        }}
                                    >
                                        <div style={{ display: "flex", flexDirection: "column", gap: 4, cursor: "move", flexShrink: 0 }} title="拖拽排序">
                                            <span style={{ width: 4, height: 4, borderRadius: "50%", background: "#c0c4cc" }} />
                                            <span style={{ width: 4, height: 4, borderRadius: "50%", background: "#c0c4cc" }} />
                                            <span style={{ width: 4, height: 4, borderRadius: "50%", background: "#c0c4cc" }} />
                                        </div>
                                        {item.feedsImg || item.coverImg ? (
                                            <img src={item.feedsImg || item.coverImg} alt="直播封面" style={{ width: 72, height: 72, borderRadius: 4, objectFit: "cover", background: "#e5e7eb" }} />
                                        ) : (
                                            <div style={{ width: 72, height: 72, borderRadius: 4, background: "#e5e7eb", flexShrink: 0 }} />
                                        )}
                                        <div style={{ minWidth: 0, flex: 1 }}>
                                            <div style={{ color: "#333", fontSize: 14, lineHeight: "22px", fontWeight: 600, wordBreak: "break-all" }}>
                                                {item.wechatLiveTitle || "-"}
                                            </div>
                                            <div style={{ marginTop: 4, color: "#999", fontSize: 12, lineHeight: "18px" }}>
                                                房间号：{item.roomIdStr || item.roomId || "-"}
                                            </div>
                                        </div>
                                        <Button size="small" type="danger" theme="borderless" onClick={() => removeAt(index)}>
                                            删除
                                        </Button>
                                    </div>
                                ))}
                            </div>
                            <AddItemButton onClick={openPicker}>添加直播</AddItemButton>
                            {!selected.length ? <div style={{ color: "#999", fontSize: 12, lineHeight: "20px", marginBottom: 16 }}>暂未选择直播间，请点击“添加直播”。</div> : null}
                        </>
                    ) : (
                        <SettingGroup>
                            <SettingRow label="直播数量">
                                <InputNumber
                                    value={data?.autoLiveNum === undefined || data?.autoLiveNum === null || data?.autoLiveNum === "" ? 10 : Number(data.autoLiveNum)}
                                    min={1}
                                    max={10}
                                    step={1}
                                    style={{ width: 275, maxWidth: "100%" }}
                                    onChange={(v) => onChange({ autoLiveNum: v === undefined || v === null ? "" : Math.round(Number(v)) })}
                                />
                            </SettingRow>
                        </SettingGroup>
                    )}
                    <div style={{ color: "#999", fontSize: 12, lineHeight: "20px" }}>{BOTTOM_TIP}</div>
                </div>
            ) : (
                <FrameSettingsField data={data} onChange={onChange} hideHead />
            )}
            <Modal title="直播间列表" visible={pickerOpen} onCancel={() => setPickerOpen(false)} onOk={confirmPicker} okText="确定" cancelText="取消" width={960}>
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
                    <Input
                        value={keywordInput}
                        placeholder="输入直播间名称"
                        showClear
                        style={{ width: 280 }}
                        onChange={(v) => setKeywordInput(v)}
                        onEnterPress={() => {
                            setKeyword(keywordInput);
                            setPage(1);
                        }}
                        onClear={() => {
                            setKeyword("");
                            setPage(1);
                        }}
                    />
                    <Button
                        type="primary"
                        theme="outline"
                        onClick={() => {
                            setKeyword(keywordInput);
                            setPage(1);
                        }}
                    >
                        搜索
                    </Button>
                    <Button
                        theme="outline"
                        onClick={() => {
                            setKeywordInput("");
                            setKeyword("");
                            setPage(1);
                        }}
                    >
                        重置
                    </Button>
                </div>
                <Spin spinning={loading}>
                    <Table
                        columns={pickerColumns}
                        dataSource={rooms}
                        rowKey={(r: any) => roomIdOf(r)}
                        pagination={false}
                        scroll={{ y: 440 }}
                        empty="暂无直播间"
                        rowSelection={{
                            selectedRowKeys: pickerSelected.map(roomIdOf),
                            onChange: (selectedRowKeys: any, selectedRows: any) => handlePickerSelect(selectedRowKeys || [], selectedRows || [])
                        }}
                    />
                </Spin>
                {total > 0 ? (
                    <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 12 }}>
                        <Pagination total={total} pageSize={PICKER_PAGE_SIZE} currentPage={page} onPageChange={(p) => setPage(p)} />
                    </div>
                ) : null}
            </Modal>
        </div>
    );
}
