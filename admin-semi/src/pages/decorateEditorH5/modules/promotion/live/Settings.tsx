/**
 * 小程序直播 设置（对齐 view/admin live/Settings.vue）
 *
 * 顶部说明 tip + 排列方式（list 列表模式/bigImage 大图模式）+ 原版自有下划线页签（内容设置/样式设置）。
 * 内容：直播内容（auto 自动/manual 手动）；manual 时已选直播间列表（封面 72×72、标题、房间号、删除；
 * 原版 vuedraggable 拖拽排序，本实现以 ↑/↓ 按钮承接排序，数据结构不变）+「添加直播」打开选择弹窗
 * （原版 DialogForm SelectLive 数据选择器，以既有接口 /promotion/wechatLive/list 落地，勾选后写回
 * selectedLiveList，上限 10）+ 未选空提示 + 底部说明；auto 时直播数量 autoLiveNum（整数 1..10）。
 * 样式：CommonFrameEdit type="live"（无分隔线/无「模块设置」标题——FrameSettingsField hideHead 承接）。
 * 注：原版 Settings 打开时有一段 syncSelectedLiveList 副作用（auto 时把自动列表前 N 条写回 selectedLiveList），
 * 本实现不复刻该静默写回（编辑器打开设置不应改数据），Preview 的 manual/auto 取数语义不受影响，差异见摘要。
 */
import { useEffect, useState } from "react";
import { Button, Checkbox, InputNumber, Modal, Spin } from "@douyinfe/semi-ui";
import { apiGet } from "../../../../../api";
import { SettingGroup, SettingRow, SegmentedRadio, FrameSettingsField, RadioRowField } from "../../../shared/settingWidgets";

const TIP = "小程序直播组件为小程序直播插件的配套组件：通过该组件可以实现自定义进入小程序直播的直播间，帮助商户提升直播相关内容的曝光率，提升直播热度。";
const BOTTOM_TIP = "最多10个，可拖拽排序直播组件目前仅支持小程序，预览页仅提供样式规范直播封面图会自动读取官方收录直播卡片封面图";

export default function LiveSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const [tab, setTab] = useState<"content" | "style">("content");
    const sourceType = data?.sourceType ?? "auto";
    const selected: any[] = Array.isArray(data?.selectedLiveList) ? data.selectedLiveList : [];
    const [pickerOpen, setPickerOpen] = useState(false);
    const [loading, setLoading] = useState(false);
    const [rooms, setRooms] = useState<any[]>([]);
    const [checkedIds, setCheckedIds] = useState<any[]>([]);

    useEffect(() => {
        if (!pickerOpen) return;
        let alive = true;
        setLoading(true);
        setCheckedIds(selected.map((s) => s.wechatLiveId || s.roomId));
        apiGet<any>("/promotion/wechatLive/list", { keyword: "", page: 1, size: 50 })
            .then((res: any) => {
                if (alive) setRooms(res?.records || res?.list || []);
            })
            .catch(() => {
                if (alive) setRooms([]);
            })
            .finally(() => {
                if (alive) setLoading(false);
            });
        return () => {
            alive = false;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [pickerOpen]);

    const confirmPicker = () => {
        const result = rooms.filter((r) => checkedIds.includes(r.wechatLiveId || r.roomId)).slice(0, 10);
        onChange({ selectedLiveList: result });
        setPickerOpen(false);
    };
    const removeAt = (index: number) => onChange({ selectedLiveList: selected.filter((_, i) => i !== index) });
    const move = (index: number, dir: -1 | 1) => {
        const next = [...selected];
        const target = index + dir;
        if (target < 0 || target >= next.length) return;
        [next[index], next[target]] = [next[target], next[index]];
        onChange({ selectedLiveList: next });
    };

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
                                        style={{ display: "flex", alignItems: "center", gap: 12, padding: 12, background: "#f7f8fa", borderRadius: 4 }}
                                    >
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
                                        <Button size="small" onClick={() => move(index, -1)} disabled={index === 0}>
                                            ↑
                                        </Button>
                                        <Button size="small" onClick={() => move(index, 1)} disabled={index === selected.length - 1}>
                                            ↓
                                        </Button>
                                        <Button size="small" type="danger" theme="borderless" onClick={() => removeAt(index)}>
                                            删除
                                        </Button>
                                    </div>
                                ))}
                            </div>
                            <div
                                onClick={() => setPickerOpen(true)}
                                style={{
                                    display: "flex",
                                    alignItems: "center",
                                    justifyContent: "center",
                                    height: 40,
                                    border: "1px solid #4f67ff",
                                    color: "#4f67ff",
                                    fontSize: 16,
                                    cursor: "pointer",
                                    gap: 5,
                                    marginBottom: 16
                                }}
                            >
                                <span style={{ fontSize: 20, lineHeight: 1 }}>+</span>
                                <span style={{ fontSize: 14 }}>添加直播</span>
                            </div>
                            {!selected.length ? <div style={{ color: "#999", fontSize: 12, lineHeight: "20px", marginBottom: 16 }}>暂未选择直播间，请点击“添加直播”。</div> : null}
                        </>
                    ) : (
                        <SettingGroup>
                            <SettingRow label="直播数量">
                                <InputNumber
                                    value={Number(data?.autoLiveNum ?? 10)}
                                    min={1}
                                    max={10}
                                    step={1}
                                    style={{ width: 240, maxWidth: "100%" }}
                                    onChange={(v) => onChange({ autoLiveNum: Math.min(Math.max(Math.round(Number(v) || 10), 1), 10) })}
                                />
                            </SettingRow>
                        </SettingGroup>
                    )}
                    <div style={{ color: "#999", fontSize: 12, lineHeight: "20px" }}>{BOTTOM_TIP}</div>
                </div>
            ) : (
                <FrameSettingsField data={data} onChange={onChange} hideHead />
            )}
            <Modal title="直播间列表" visible={pickerOpen} onCancel={() => setPickerOpen(false)} onOk={confirmPicker} okText="确定" cancelText="取消" width={720}>
                <Spin spinning={loading}>
                    <div style={{ maxHeight: 420, overflowY: "auto", display: "flex", flexDirection: "column", gap: 6 }}>
                        {rooms.map((r) => {
                            const id = r.wechatLiveId || r.roomId;
                            return (
                                <label key={id} style={{ display: "flex", alignItems: "center", gap: 10, padding: "8px 10px", background: "#f7f8fa", borderRadius: 4, cursor: "pointer" }}>
                                    <Checkbox
                                        checked={checkedIds.includes(id)}
                                        onChange={(e: any) => setCheckedIds((prev) => (e?.target?.checked ? [...prev, id] : prev.filter((x) => x !== id)))}
                                    />
                                    {r.feedsImg || r.coverImg ? (
                                        <img src={r.feedsImg || r.coverImg} alt="" style={{ width: 48, height: 48, borderRadius: 4, objectFit: "cover" }} />
                                    ) : null}
                                    <span style={{ fontSize: 13 }}>{r.wechatLiveTitle || "-"}</span>
                                    <span style={{ fontSize: 12, color: "#999" }}>房间号：{r.roomIdStr || r.roomId || "-"}</span>
                                    {r.liveStatusText ? <span style={{ fontSize: 12, color: "#999" }}>{r.liveStatusText}</span> : null}
                                </label>
                            );
                        })}
                        {!loading && !rooms.length ? <div style={{ color: "#999", fontSize: 12, padding: 12 }}>暂无直播间</div> : null}
                    </div>
                </Spin>
            </Modal>
        </div>
    );
}
