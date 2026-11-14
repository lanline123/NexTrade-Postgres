/**
 * 我的钱包 设置（覆盖渲染端读取的 layoutStyle/walletIcon/frame）
 */
import { SettingGroup, RadioRowField, SinglePicField, FrameSettingsField } from "../../../shared/settingWidgets";

export default function MyWalletSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="钱包">
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="layoutStyle"
                    label="布局样式"
                    options={[
                        { value: 1, label: "标题+数据" },
                        { value: 2, label: "图标+数据" }
                    ]}
                />
                <SinglePicField data={data} onChange={onChange} field="walletIcon" label="钱包图标(样式2)" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
