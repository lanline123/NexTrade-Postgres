/**
 * 商城公告2 设置（对齐 view/admin mobilePro/noticeA2/Settings.vue）
 *
 * 页签：内容设置 / 扩展设置。
 * 内容页：公告内容（textarea 3 行）+ 公告跳转链接 + 参数设置（文字效果/对齐或
 * 滚动耗时/图标类型/文字颜色/关闭按钮）；扩展页 ModuleStyleEdit + ContentStyleEdit
 * （type="noticeA2"）。图标类型切换时按原版 watch 行为同步 iconPic 默认图。
 * 注：公告跳转链接原版为 SelectLink（未移植），以文本输入写回同一 nociceLink 结构。
 */
import { TextArea, Input } from "@douyinfe/semi-ui";
import { SettingGroup, SettingRow, SegmentedRadio, RangeField, ColorField, RadioRowField, SinglePicField } from "../../../shared/settingWidgets";
import { SettingsTabs, ExtensionStyleSettings } from "../../../shared/styleEdits";

import { useI18n } from '../../../../../i18n';
/** 原版 Settings.vue 内置的两张图标（切换 noticeIconType 时烧录） */
const CUSTOM_ICON = {
    picUrl: "https://oss.tigshop.com/img/gallery/202411/17315607538zqwwuwCIA2X97Slx5.png",
    picThumb: "https://oss.tigshop.com/img/gallery/202411/17315607538zqwwuwCIA2X97Slx5.png?x-oss-process=image/resize,m_pad,h_200,h_200",
    picId: 1447,
    picName: "FmNDutABVdVhDqScSHImoufkXdBL"
};
const DEFAULT_ICON = {
    picUrl: "https://oss.tigshop.com/img/gallery/202411/1731560722X2AIPCOsyzKVyTB3ZD.png",
    picThumb: "https://oss.tigshop.com/img/gallery/202411/1731560722X2AIPCOsyzKVyTB3ZD.png?x-oss-process=image/resize,m_pad,h_200,h_200",
    picId: 1446,
    picName: "通知"
};

export default function NoticeA2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const { t } = useI18n();
    const linkUrl = typeof data?.nociceLink === "string" ? data.nociceLink : data?.nociceLink?.link?.link ?? "";
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <>
                    <SettingGroup title="公告内容">
                        <div className="dec-edit-group dec-edit-group-block">
                            <div className="dec-edit-group-con">
                                <TextArea
                                    value={data?.noticeContent ?? ""}
                                    rows={3}
                                    placeholder={t('请输入公告内容')}
                                    style={{ width: "100%" }}
                                    onChange={(v: string) => onChange({ noticeContent: v })}
                                />
                            </div>
                        </div>
                        <SettingRow label="公告跳转链接">
                            <Input
                                value={linkUrl}
                                placeholder={t('跳转链接，如 /pages/index/index')}
                                style={{ width: 240, maxWidth: "100%" }}
                                onChange={(v: string) =>
                                    onChange({
                                        nociceLink:
                                            data?.nociceLink && typeof data.nociceLink === "object"
                                                ? { ...data.nociceLink, link: { ...(data.nociceLink.link || {}), link: v } }
                                                : { link: { link: v, title: "" } }
                                    })
                                }
                            />
                        </SettingRow>
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="nociceEffect"
                            label="公告文字效果"
                            options={[
                                { value: "scroll", label: "滚动" },
                                { value: "still", label: "静止" },
                            ]}
                        />
                        {data?.nociceEffect === "still" ? (
                            <RadioRowField
                                data={data}
                                onChange={onChange}
                                field="textAlignment"
                                label="标题对齐方式"
                                options={[
                                    { value: "left", label: "左对齐" },
                                    { value: "center", label: "居中对齐" },
                                    { value: "right", label: "右对齐" },
                                ]}
                            />
                        ) : (
                            <RangeField data={data} onChange={onChange} field="scrollSpeed" label="滚动耗时" min={5} max={40} />
                        )}
                        <SettingRow label="公告图标类型">
                            <SegmentedRadio
                                value={data?.noticeIconType ?? "default"}
                                options={[
                                    { value: "default", label: "默认" },
                                    { value: "custom", label: "自定义" },
                                    { value: "hide", label: "隐藏" },
                                ]}
                                onChange={(v) => onChange({ noticeIconType: v, iconPic: v === "custom" ? { ...CUSTOM_ICON } : { ...DEFAULT_ICON } })}
                            />
                        </SettingRow>
                        {data?.noticeIconType === "custom" && (
                            <SinglePicField data={data} onChange={onChange} field="iconPic" label="公告图标，宽度不限，高60" />
                        )}
                        <ColorField data={data} onChange={onChange} field="contentColor" label="公告文字颜色" resetValue="#333333" />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="closeIcon"
                            label="关闭按钮设置"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "不显示" },
                            ]}
                        />
                    </SettingGroup>
                </>
                <ExtensionStyleSettings data={data} onChange={onChange} type="noticeA2" />
            </SettingsTabs>
        </div>
    );
}
