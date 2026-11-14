/**
 * 商品搜索 设置（对齐 view/admin editor/modules/mobile/searchBar/Settings.vue）
 *
 * 搜索框文字 + CommonFrameEdit 专属形态（type=searchBar：文字颜色/组件背景/组件圆角/组件高度 + 背景/边距）。
 */
import { SettingGroup, TextField, FrameSettingsField } from "../../../shared/settingWidgets";

export default function SearchBarSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="商品搜索">
                <TextField data={data} onChange={onChange} field="text" label="搜索框文字" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} variant="searchBar" />
        </div>
    );
}
