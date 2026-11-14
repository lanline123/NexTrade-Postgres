/**
 * 会员广告图 设置（覆盖渲染端读取的 pic/picLink/frame）
 */
import { SettingGroup, TextField, SinglePicField, FrameSettingsField } from "../../../shared/settingWidgets";

export default function AdImageSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const picLink = data?.picLink ?? {};
    const setPicLink = (patch: Record<string, any>) => onChange({ picLink: { ...picLink, ...patch } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="广告图">
                <SinglePicField data={data} onChange={onChange} field="pic" label="广告图片" />
                <TextField data={picLink} onChange={setPicLink} field="link" label="跳转链接" placeholder="/pages/..." />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
