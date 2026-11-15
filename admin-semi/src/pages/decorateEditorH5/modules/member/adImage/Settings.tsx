/**
 * 广告图 设置（对齐 view/admin adImage/Settings.vue）
 *
 * 页签 内容设置/扩展设置。内容：广告图片 PicSelect（重置回默认图）、图片链接
 * （原版 SelectLink 未移植，以文本输入承接，写回 picLink {link,name} 同一结构）；
 * 扩展：CommonFrameEdit 全量。
 */
import { Input } from "@douyinfe/semi-ui";
import { SettingGroup, SettingRow, PicSelectField, FrameSettingsField } from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import { useI18n } from "../../../../../i18n";

export default function AdImageSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const { t } = useI18n();
    const picLink = data?.picLink && typeof data.picLink === "object" ? data.picLink : { link: "", name: "" };
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <div>
                    <SettingGroup>
                        <PicSelectField data={data} onChange={onChange} field="pic" label="广告图片" />
                        <SettingRow label="图片链接">
                            <Input
                                value={picLink.link ?? ""}
                                placeholder={t("跳转链接，如 /pages/product/index?id=1")}
                                style={{ width: 240, maxWidth: "100%" }}
                                onChange={(v: string) => onChange({ picLink: { ...picLink, link: v } })}
                            />
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
