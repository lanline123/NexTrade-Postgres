/**
 * 分类推荐 设置（对齐 view/admin mobilePro/categoryRec/Settings.vue）
 *
 * 页签：导航内容 / 扩展设置。
 * 内容页：内容设置（CategoryGroupEdit 折叠卡：分类图片/标题/链接/标题颜色/背景颜色）
 * + 参数设置（缓动效果/分类内部间距/分类圆角大小/分类标题/分类背景颜色/分类边框组）；
 * 扩展页：ModuleStyleEdit（type="categoryRec", topPaddingMax=100）+ ContentStyleEdit（type="categoryA1"）。
 * 注：链接原版为 SelectLink 选择器（未移植），此处以文本输入写回同一 link 结构。
 */
import { Input } from "@douyinfe/semi-ui";
import { SettingGroup, SettingRow, RangeField, ColorField, RadioRowField, TextField, SinglePicField, DecDivider } from "../../../shared/settingWidgets";
import {
    SettingsTabs,
    ModuleStyleSettingsField,
    ContentStyleSettingsField,
    AnimationSettingsField,
    SubTitleSettingsField,
    GroupCardList
} from "../../../shared/styleEdits";

import { useI18n } from '../../../../../i18n';
const EMPTY_LINK = { link: { link: "", title: "" }, linkTitle: "" };

function CategoryGroupEditor({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const { t } = useI18n();
    const list: Array<Record<string, any>> = Array.isArray(data?.categoryList) ? data.categoryList : [];
    const setList = (next: Array<Record<string, any>>) => onChange({ categoryList: next });
    const patchItem = (index: number, patch: Record<string, any>) => setList(list.map((it, i) => (i === index ? { ...it, ...patch } : it)));
    const moveItem = (index: number, delta: number) => {
        const target = index + delta;
        if (target < 0 || target >= list.length) return;
        const next = [...list];
        const [it] = next.splice(index, 1);
        next.splice(target, 0, it);
        setList(next);
    };
    return (
        <GroupCardList
            items={list}
            addText="添加一组"
            onAdd={() => setList([...list, { title: "", link: JSON.parse(JSON.stringify(EMPTY_LINK)), titleColor: "#ffffff", backgroundColor: "#ffffff", catPic: {} }])}
            onRemove={(index) => setList(list.filter((_, i) => i !== index))}
            onMove={moveItem}
            renderItem={(item, index) => {
                const itemProps = { data: item, onChange: (patch: Record<string, any>) => patchItem(index, patch) };
                const linkUrl = typeof item.link === "string" ? item.link : item.link?.link?.link ?? "";
                return (
                    <>
                        <SinglePicField {...itemProps} field="catPic" label="分类图片" />
                        <TextField {...itemProps} field="title" label="标题" placeholder="请输入标题内容" />
                        <SettingRow label="链接">
                            <Input
                                value={linkUrl}
                                placeholder={t('跳转链接，如 /pages/index/index')}
                                style={{ width: 240, maxWidth: "100%" }}
                                onChange={(v: string) =>
                                    patchItem(index, {
                                        link:
                                            item.link && typeof item.link === "object"
                                                ? { ...item.link, link: { ...(item.link.link || {}), link: v } }
                                                : { link: { link: v, title: "" }, linkTitle: "" }
                                    })
                                }
                            />
                        </SettingRow>
                        <ColorField {...itemProps} field="titleColor" label="标题颜色" resetValue="" />
                        <ColorField {...itemProps} field="backgroundColor" label="背景颜色" resetValue="" />
                    </>
                );
            }}
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
