/**
 * 拼团 设置（对齐 view/admin groupon/Settings.vue；原版为手写全字段而非 CommonActivityFrameEdit）
 *
 * 选择模板（1 大图模式/2 一行两个/3 一行三个/5 详细列表，选项集与活动框不同，故单独渲染）→
 * 拼团类型（1 普通拼团/2 阶梯拼团）→ 选择活动（原版 DialogForm SelectActivity 数据选择器：
 * 本实现以既有接口 /promotion/groupon/list 落地为弹窗多选，按 groupType 过滤；
 * 确认后 grouponActivityItem 取首个，删除任一 chip 即置 grouponActivityItem=null——均同原版回调语义）→
 * 商品卡/容器字段（与 ActivityFrameFields 同字段集，隐藏其「选择模板/瀑布流」两组：
 * 原版瀑布流组已注释、showBrief 未渲染）→ CommonTitleEdit（TitleSettingsField）。
 */
import { useEffect, useState } from "react";
import { Button, Checkbox, Modal, Spin } from "@douyinfe/semi-ui";
import { apiGet } from "../../../../../api";
import { SettingGroup, SettingRow, SegmentedRadio, DecDivider, TitleSettingsField, RadioRowField } from "../../../shared/settingWidgets";
import ActivityFrameFields from "../../../shared/ActivityFrameFields";

export default function GrouponSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const groupType = data?.groupType ?? 1;
    const [selected, setSelected] = useState<any[]>(() => (data?.grouponActivityItem ? [data.grouponActivityItem] : []));
    const [pickerOpen, setPickerOpen] = useState(false);
    const [loading, setLoading] = useState(false);
    const [activities, setActivities] = useState<any[]>([]);
    const [checkedIds, setCheckedIds] = useState<any[]>([]);

    useEffect(() => {
        if (!pickerOpen) return;
        let alive = true;
        setLoading(true);
        setCheckedIds(selected.map((s) => s.grouponPromotionId));
        apiGet<any>("/promotion/groupon/list", { page: 1, size: 50 })
            .then((res: any) => {
                if (!alive) return;
                const rows = res?.records || res?.list || [];
                setActivities(rows.filter((r: any) => r.groupType === undefined || r.groupType === null || Number(r.groupType) === Number(groupType)));
            })
            .catch(() => {
                if (alive) setActivities([]);
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
        const result = activities.filter((a) => checkedIds.includes(a.grouponPromotionId));
        setSelected(result);
        onChange({ grouponActivityItem: result.length ? result[0] : null });
        setPickerOpen(false);
    };
    const removeActivity = (index: number) => {
        setSelected((prev) => prev.filter((_, i) => i !== index));
        onChange({ grouponActivityItem: null });
    };

    return (
        <div className="h5-editor-settings-body">
            <SettingGroup>
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="style"
                    label="选择模板"
                    options={[
                        { value: 1, label: "大图模式" },
                        { value: 2, label: "一行两个" },
                        { value: 3, label: "一行三个" },
                        { value: 5, label: "详细列表" }
                    ]}
                />
            </SettingGroup>
            <DecDivider />
            <SettingGroup>
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="groupType"
                    label="拼团类型"
                    options={[
                        { value: 1, label: "普通拼团" },
                        { value: 2, label: "阶梯拼团" }
                    ]}
                />
                <SettingRow label="拼团活动" block>
                    <Button type="primary" onClick={() => setPickerOpen(true)}>
                        选择活动
                    </Button>
                    <div style={{ display: "flex", flexDirection: "column", gap: 5, paddingTop: 10 }}>
                        {selected.map((item, index) => (
                            <div
                                key={item.grouponPromotionId ?? index}
                                style={{
                                    width: "100%",
                                    paddingLeft: 10,
                                    height: 25,
                                    lineHeight: "25px",
                                    fontSize: 12,
                                    backgroundColor: "rgba(223,225,232,0.4)",
                                    borderRadius: 2,
                                    position: "relative",
                                    boxSizing: "border-box"
                                }}
                            >
                                {item.name}
                                <span
                                    onClick={() => removeActivity(index)}
                                    style={{ position: "absolute", right: 15, top: 0, cursor: "pointer", color: "#FB0606", fontSize: 14 }}
                                >
                                    ×
                                </span>
                            </div>
                        ))}
                    </div>
                </SettingRow>
            </SettingGroup>
            <DecDivider />
            <ActivityFrameFields data={data} onChange={onChange} hideStyle hideWaterfall />
            <TitleSettingsField data={data} onChange={onChange} field="title" />
            <Modal
                title="选择活动"
                visible={pickerOpen}
                onCancel={() => setPickerOpen(false)}
                onOk={confirmPicker}
                okText="确定"
                cancelText="取消"
                width={600}
            >
                <Spin spinning={loading}>
                    <div style={{ maxHeight: 380, overflowY: "auto", display: "flex", flexDirection: "column", gap: 6 }}>
                        {activities.map((a) => (
                            <label
                                key={a.grouponPromotionId}
                                style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 10px", background: "#f7f8fa", borderRadius: 4, cursor: "pointer" }}
                            >
                                <Checkbox
                                    checked={checkedIds.includes(a.grouponPromotionId)}
                                    onChange={(e: any) =>
                                        setCheckedIds((prev) =>
                                            e?.target?.checked ? [...prev, a.grouponPromotionId] : prev.filter((id) => id !== a.grouponPromotionId)
                                        )
                                    }
                                />
                                <span style={{ fontSize: 13 }}>{a.name}</span>
                                {a.startTime ? <span style={{ fontSize: 12, color: "#999" }}>{a.startTime}</span> : null}
                                {a.statusName ? <span style={{ fontSize: 12, color: "#999" }}>{a.statusName}</span> : null}
                            </label>
                        ))}
                        {!loading && !activities.length ? <div style={{ color: "#999", fontSize: 12, padding: 12 }}>暂无可选活动</div> : null}
                    </div>
                </Spin>
            </Modal>
        </div>
    );
}
