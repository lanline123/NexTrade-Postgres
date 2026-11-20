/**
 * 分割线 设置（字段对齐 view/admin whiteLine/Settings.vue）
 */
import { useMemo } from "react";
import { SettingGroup, RadioRowField, FrameSettingsField } from "../../../shared/settingWidgets";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { whiteLineDefaults } from "./index";

export default function WhiteLineSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    // 存量兜底：设置面拿到的是原始存储 data（面板不合并默认值），缺 lineType/lineStyle
    // 的存量模块若直接喂控件会整组无选中，而画布预览经 mergeDefaultModule 已按默认值
    // 渲染——两边对不上。此处按同一合并口径（单一来源 whiteLineDefaults）补齐后再喂
    // 两个单选字段，使缺字段时回显「分割线/实线」且与渲染一致；仅影响读取显示，
    // onChange 仍按原契约只回写被改字段，用户点选后字段即落入存储数据。
    const radioData = useMemo(() => {
        const merged: Record<string, any> = { ...(data ?? {}) };
        mergeDefaultModule(merged, whiteLineDefaults);
        return merged;
    }, [data]);

    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="空白分割">
                <RadioRowField
                    data={radioData}
                    onChange={onChange}
                    field="lineType"
                    label="分割类型"
                    options={[
                        { value: 1, label: "空白" },
                        { value: 2, label: "分割线" }
                    ]}
                />
                {/* 原版「选择样式」恒显示（不随 lineType 条件隐藏） */}
                <RadioRowField
                    data={radioData}
                    onChange={onChange}
                    field="lineStyle"
                    label="选择样式"
                    options={[
                        { value: 1, label: "实线" },
                        { value: 2, label: "虚线" },
                        { value: 3, label: "点状线" }
                    ]}
                />
            </SettingGroup>
            {/* 原版 CommonFrameEdit type="line"：隐藏圆角/内边距两项，经 frameType="line" 门控 */}
            <FrameSettingsField data={data} onChange={onChange} label="边框设置" frameType="line" />
        </div>
    );
}
