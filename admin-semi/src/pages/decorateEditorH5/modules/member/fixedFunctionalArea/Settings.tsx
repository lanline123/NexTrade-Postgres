/**
 * 固定功能区 设置（对齐 view/admin fixedFunctionalArea/Settings.vue）
 *
 * 无页签：仅「内容说明」静态提示 + CommonFrameEdit 全量（系统注入模块，无内容字段）。
 */
import { SettingRow, FrameSettingsField } from "../../../shared/settingWidgets";

export default function FixedFunctionalAreaSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingRow label="内容说明" block>
                <div style={{ color: "#999", fontSize: 12, lineHeight: 1.6 }}>该模块为系统固定功能区，内容自动生成，无需手动配置。</div>
            </SettingRow>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
