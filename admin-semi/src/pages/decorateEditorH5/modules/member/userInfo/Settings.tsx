/**
 * 会员信息 设置（覆盖渲染端读取的 signIn/settings/gradient/frame；nickname/description 仅 editor 演示）
 */
import { SettingGroup, TextField, SwitchField, ColorField, SinglePicField, FrameSettingsField } from "../../../shared/settingWidgets";

export default function UserInfoSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const signIn = data?.signIn ?? {};
    const setSignIn = (patch: Record<string, any>) => onChange({ signIn: { ...signIn, ...patch } });
    const settings = data?.settings ?? {};
    const setSettings = (patch: Record<string, any>) => onChange({ settings: { ...settings, ...patch } });
    const gradient = data?.gradient ?? {};
    const setGradient = (patch: Record<string, any>) => onChange({ gradient: { ...gradient, ...patch } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="演示信息">
                <TextField data={data} onChange={onChange} field="nickname" label="昵称(演示)" />
                <TextField data={data} onChange={onChange} field="description" label="等级/描述(演示)" />
            </SettingGroup>
            <SettingGroup title="签到入口">
                <SwitchField data={signIn} onChange={setSignIn} field="show" label="显示签到" />
                <TextField data={signIn} onChange={setSignIn} field="title" label="签到文案" />
                <SinglePicField data={signIn} onChange={setSignIn} field="icon" label="签到图标" />
            </SettingGroup>
            <SettingGroup title="设置入口">
                <SwitchField data={settings} onChange={setSettings} field="show" label="显示设置" />
                <SinglePicField data={settings} onChange={setSettings} field="icon" label="设置图标" />
            </SettingGroup>
            <SettingGroup title="头部渐变">
                <ColorField data={gradient} onChange={setGradient} field="colorA" label="渐变色A" />
                <ColorField data={gradient} onChange={setGradient} field="colorB" label="渐变色B" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
