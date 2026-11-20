/**
 * 分类推荐 设置（对齐 view/admin mobilePro/categoryRec/Settings.vue）
 *
 * 页签：导航内容 / 扩展设置。
 * 内容页：内容设置（CategoryGroupEdit 折叠卡：分类图片/标题/链接/标题颜色/背景颜色）
 * + 参数设置（缓动效果/分类内部间距/分类圆角大小/分类标题/分类背景颜色/分类边框组）；
 * 扩展页：ModuleStyleEdit（type="categoryRec", topPaddingMax=100）+ ContentStyleEdit（type="categoryA1"）。
 * 链接已恢复原版选链方式：PicLinkField（原版 CategoryGroupEdit 以 SelectLink 绑定
 * element.link 整体，选中即回写完整 LinkType；默认占位 { link:{link,title}, linkTitle } 保留）。
 */
import { SettingGroup, SettingRow, RangeField, ColorField, RadioRowField, TextField, SinglePicField, DecDivider, PicLinkField } from "../../../shared/settingWidgets";
import {
    SettingsTabs,
    ModuleStyleSettingsField,
    ContentStyleSettingsField,
    AnimationSettingsField,
    SubTitleSettingsField
} from "../../../shared/styleEdits";
import GroupListEditor from "../../../shared/GroupListEditor";

const EMPTY_LINK = { link: { link: "", title: "" }, linkTitle: "" };

function CategoryGroupEditor({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <GroupListEditor
            data={data}
            onChange={onChange}
            field="categoryList"
            addText="添加一组"
            createItem={() => ({ title: "", link: JSON.parse(JSON.stringify(EMPTY_LINK)), titleColor: "#ffffff", backgroundColor: "#ffffff", catPic: {} })}
            renderItem={(item, _index, itemProps) => (
                <>
                    <SinglePicField {...itemProps} field="catPic" label="分类图片" />
                    <TextField {...itemProps} field="title" label="标题" placeholder="请输入标题内容" />
                    <SettingRow label="链接">
                        <PicLinkField
                            value={typeof item.link === "string" ? { link: item.link, title: "" } : item.link ?? null}
                            onChange={(v) => itemProps.onChange({ link: v })}
                        />
                    </SettingRow>
                    <ColorField {...itemProps} field="titleColor" label="标题颜色" resetValue="" />
                    <ColorField {...itemProps} field="backgroundColor" label="背景颜色" resetValue="" />
                </>
            )}
        />
    );
}

export default function CategoryRecSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["导航内容", "扩展设置"]}>
                <>
                    <SettingGroup title="内容设置">
                        <CategoryGroupEditor data={data} onChange={onChange} />
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <AnimationSettingsField data={data} onChange={onChange} type="categoryA1" />
                        <DecDivider />
                        <RangeField data={data} onChange={onChange} field="categoryPadding" label="分类内部间距" max={10} />
                        <RangeField data={data} onChange={onChange} field="radius" label="分类圆角大小" max={25} />
                        <SubTitleSettingsField data={data} onChange={onChange} title="分类标题" type="categoryRec" />
                        <ColorField data={data} onChange={onChange} field="backgroundColor" label="分类背景颜色" resetValue="#ffffff" />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="catBorder"
                            label="分类边框设置"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "不显示" },
                            ]}
                        />
                        {Number(data?.catBorder) === 1 ? (
                            <>
                                <RangeField data={data} onChange={onChange} field="catBorderWidth" label="分类边框粗细" max={25} />
                                <ColorField data={data} onChange={onChange} field="catBorderColor" label="分类边框颜色" resetValue="#ffffff" />
                            </>
                        ) : null}
                    </SettingGroup>
                </>
                <>
                    <ModuleStyleSettingsField data={data} onChange={onChange} type="categoryRec" topPaddingMax={100} />
                    <ContentStyleSettingsField data={data} onChange={onChange} type="categoryA1" />
                </>
            </SettingsTabs>
        </div>
    );
}
