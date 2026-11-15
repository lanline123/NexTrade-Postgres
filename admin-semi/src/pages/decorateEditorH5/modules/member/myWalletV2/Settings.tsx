/**
 * 我的钱包2 设置（对齐 view/admin myWalletV2/Settings.vue）
 *
 * 页签 内容设置/扩展设置。内容：按 items 逐项「<label>图标」PicSelect（写回 items[i].icon）+
 * 内容说明静态提示；扩展：CommonFrameEdit 全量。
 */
import { SettingGroup, SettingRow, PicSelectBox, FrameSettingsField } from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import { myWalletV2Defaults } from "./index";

export default function MyWalletV2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const items: any[] = Array.isArray(data?.items) && data.items.length ? data.items : myWalletV2Defaults.items;
    const setItemIcon = (index: number, icon: any) => {
        const next = items.map((it, i) => (i === index ? { ...it, icon } : it));
        onChange({ items: next });
    };
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <div>
                    <SettingGroup>
                        {items.map((item, index) => (
                            <SettingRow key={item.key ?? index} label={`${item.label}图标`}>
                                <PicSelectBox pic={item.icon} onSelect={(v) => setItemIcon(index, v)} onReset={() => setItemIcon(index, {})} />
                            </SettingRow>
                        ))}
                        <SettingRow label="内容说明" block>
                            <div style={{ color: "#999", fontSize: 12, lineHeight: 1.6 }}>
                                该模块展示用户的资产与行为数据，包括余额、优惠券、商品收藏、浏览记录，数据自动获取，无需手动配置。
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
