/**
 * 用户信息2 设置（对齐 view/admin userInfoV2/Settings.vue）
 *
 * 无页签、无内容字段：仅「内容说明」静态提示 + CommonFrameEdit 全量。
 */
import { SettingRow, FrameSettingsField } from "../../../shared/settingWidgets";

export default function UserInfoV2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingRow label="内容说明" block>
                <div style={{ color: "#999", fontSize: 12, lineHeight: 1.6 }}>该模块展示用户基本信息，无需手动配置。</div>
            </SettingRow>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
