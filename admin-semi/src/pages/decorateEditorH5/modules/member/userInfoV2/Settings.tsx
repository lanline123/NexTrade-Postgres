/**
 * 会员信息 V2 设置（渲染端仅读 frame；昵称/描述为演示占位）
 */
import { SettingGroup, TextField, FrameSettingsField } from "../../../shared/settingWidgets";

export default function UserInfoV2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="演示信息">
                <TextField data={data} onChange={onChange} field="nickname" label="昵称(演示)" />
                <TextField data={data} onChange={onChange} field="description" label="描述(演示)" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
