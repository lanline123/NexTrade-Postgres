/**
 * 账号 设置（对齐 view/admin account/Settings.vue）
 *
 * 页签 内容设置/扩展设置。内容：模块标题（maxlength 10）+ 分隔线 + 账号列表 PicList
 * （showText、showLink=false、showAddButton=false——PicListField 已补该门控，固定入口不可新增）；
 * 扩展：CommonFrameEdit 全量。
 */
import { SettingGroup, TextField, PicListEditorField, DecDivider, FrameSettingsField } from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";

export default function AccountSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <div>
                    <SettingGroup>
                        <TextField data={data} onChange={onChange} field="moduleTitle" label="模块标题" placeholder="请输入标题" />
                    </SettingGroup>
                    <DecDivider />
                    <PicListEditorField data={data} onChange={onChange} field="accountList" label="账号列表设置" showText showDesc showLink={false} showAddButton={false} />
                </div>
                <div>
                    <FrameSettingsField data={data} onChange={onChange} />
                </div>
            </SettingsTabs>
        </div>
    );
}
