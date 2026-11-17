/**
 * 我的钱包 设置（对齐 view/admin myWallet/Settings.vue）
 *
 * 页签 内容设置/扩展设置。内容：布局样式（样式一/样式二）→ 钱包图标 PicSelect（仅样式二，
 * 重置回默认图标 myWalletDefaults.walletIcon，同原版 :default-value）→
 * 内容说明静态提示；扩展：CommonFrameEdit 全量。
 */
import { SettingGroup, SettingRow, SegmentedRadio, PicSelectField, FrameSettingsField, RadioRowField } from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import { myWalletDefaults } from "./index";

export default function MyWalletSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const layoutStyle = data?.layoutStyle ?? 1;
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <div>
                    <SettingGroup>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="layoutStyle"
                            label="布局样式"
                            options={[
                                { value: 1, label: "样式一" },
                                { value: 2, label: "样式二" }
                            ]}
                        />
                        {layoutStyle === 2 ? (
                            <PicSelectField data={data} onChange={onChange} field="walletIcon" label="钱包图标" resetPic={myWalletDefaults.walletIcon} />
                        ) : null}
                        <SettingRow label="内容说明" block>
                            <div style={{ color: "#999", fontSize: 12, lineHeight: 1.6 }}>
                                该模块展示用户的资产信息，包括余额、优惠券、积分等，数据自动获取，无需手动配置。
                            </div>
                        </SettingRow>
                    </SettingGroup>
                </div>
                <div>
                    <FrameSettingsField data={data} onChange={onChange} />
                </div>
            </SettingsTabs>
        </div>
    );
}
