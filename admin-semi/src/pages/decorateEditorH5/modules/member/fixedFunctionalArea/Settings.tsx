/**
 * 固定功能区域 设置（渲染端无可配业务字段，仅 frame 外壳样式）
 */
import { SettingGroup, FrameSettingsField } from "../../../shared/settingWidgets";

export default function FixedFunctionalAreaSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="说明">
                <div style={{ color: "#646a73", fontSize: 12 }}>内容固定为账期支付卡，由渲染端按用户授信状态决定是否显示，无业务字段可配。</div>
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
