/**
 * 公告 设置（字段对齐 view/admin notice/Settings.vue）
 *
 * 原版无额外分组标题（仅 dec-edit-title「公告」后平铺字段），故此处不设组标题；
 * 文本字段标签为「公告内容」（el-input 宽 240px，与 TextField 一致）。
 */
import { SettingGroup, TextField, RadioRowField, SinglePicField, FrameSettingsField } from "../../../shared/settingWidgets";

export default function NoticeSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup>
                <TextField data={data} onChange={onChange} field="text" label="公告内容" />
                <SinglePicField data={data} onChange={onChange} field="icoPic" label="公告图标" />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="nociceEffect"
                    label="公告文字效果"
                    options={[
                        { value: "scroll", label: "滚动" },
                        { value: "still", label: "静止" }
                    ]}
                />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} variant="notice" />
        </div>
    );
}
