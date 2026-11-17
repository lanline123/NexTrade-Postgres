/**
 * 广告图 设置（对齐 view/admin adImage/Settings.vue）
 *
 * 页签 内容设置/扩展设置。内容：广告图片 PicSelect（重置回默认图 adImageDefaults.pic，
 * 同原版 :default-value）、图片链接 PicLinkField（原版 SelectLink，完整 LinkType 存
 * picLink {link,name,...}）；扩展：CommonFrameEdit 全量。
 */
import { SettingGroup, SettingRow, PicSelectField, PicLinkField, FrameSettingsField } from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import { adImageDefaults } from "./index";

export default function AdImageSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const picLink = data?.picLink && typeof data.picLink === "object" ? data.picLink : { link: "", name: "" };
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <div>
                    <SettingGroup>
                        <PicSelectField data={data} onChange={onChange} field="pic" label="广告图片" resetPic={adImageDefaults.pic} />
                        <SettingRow label="图片链接">
                            <PicLinkField value={picLink} onChange={(v) => onChange({ picLink: v })} />
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
