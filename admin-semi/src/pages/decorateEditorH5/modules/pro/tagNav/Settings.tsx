/**
 * 标签导航 设置（对齐 view/admin mobilePro/tagNav/Settings.vue）
 *
 * 页签：分组设置（基本设置 GroupListEdit + 参数设置）/ 扩展设置
 * （颜色设置：categoryColors 渐变色对 + 分组标题色；ModuleStyle type="categoryRec"）。
 * 分组链接 SelectLink 未移植，沿用文本输入写回同一嵌套结构（部分对齐）。
 */
import { Input } from "@douyinfe/semi-ui";
import { SettingGroup, SettingRow, RadioRowField, RangeField, ColorField, TextField } from "../../../shared/settingWidgets";
import { SettingsTabs, GroupCardList, ModuleStyleSettingsField } from "../../../shared/styleEdits";
import AlphaColorInput from "../../../../../components/AlphaColorInput";

const EMPTY_LINK = { link: { link: "", title: "" }, linkTitle: "" };

function GroupListEditor({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const list: Array<Record<string, any>> = Array.isArray(data?.groupList) ? data.groupList : [];
    const setList = (next: Array<Record<string, any>>) => onChange({ groupList: next });
    const patchItem = (index: number, patch: Record<string, any>) => setList(list.map((it, i) => (i === index ? { ...it, ...patch } : it)));
    const moveItem = (index: number, delta: number) => {
        const target = index + delta;
        if (target < 0 || target >= list.length) return;
        const next = [...list];
        [next[index], next[target]] = [next[target], next[index]];
        setList(next);
    };
    return (
        <GroupCardList
            items={list}
            addText="添加分组推荐"
            onAdd={() => setList([...list, { title: "", link: JSON.parse(JSON.stringify(EMPTY_LINK)) }])}
            onRemove={(index) => setList(list.filter((_, i) => i !== index))}
            onMove={moveItem}
            canRemove={list.length > 1}
            renderItem={(item, index) => {
                const itemProps = { data: item, onChange: (patch: Record<string, any>) => patchItem(index, patch) };
                const linkUrl = typeof item.link === "string" ? item.link : item.link?.link?.link ?? "";
                return (
                    <>
                        <TextField {...itemProps} field="title" label="标题" placeholder="请输入标题内容" />
                        <SettingRow label="链接">
                            <Input
                                value={linkUrl}
                                placeholder="跳转链接，如 /pages/index/index"
                                style={{ width: 240, maxWidth: "100%" }}
                                onChange={(v: string) =>
                                    patchItem(index, {
                                        link:
                                            item.link && typeof item.link === "object"
                                                ? { ...item.link, link: { ...(item.link.link || {}), link: v } }
                                                : { link: { link: v, title: "" }, linkTitle: "" },
                                    })
                                }
                            />
                        </SettingRow>
                    </>
                );
            }}
        />
    );
}

export default function TagNavSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const categoryColors: Array<{ colorA?: string; colorB?: string }> = Array.isArray(data?.categoryColors) ? data.categoryColors : [];
    const setColorPair = (index: number, patch: Record<string, any>) =>
        onChange({ categoryColors: categoryColors.map((c, i) => (i === index ? { ...c, ...patch } : c)) });

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["分组设置", "扩展设置"]}>
                <>
                    <SettingGroup title="基本设置">
                        <GroupListEditor data={data} onChange={onChange} />
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="textWeight"
                            label="文本样式"
                            options={[
                                { value: 400, label: "常规体" },
                                { value: 700, label: "加粗体" },
                            ]}
                        />
                        <RangeField data={data} onChange={onChange} field="groupGap" label="分组间距" max={30} />
                        <RangeField data={data} onChange={onChange} field="groupPadding" label="分组左右边距" max={30} />
                    </SettingGroup>
                </>
                <>
                    <SettingGroup title="颜色设置">
                        {categoryColors.map((c, index) => (
                            <SettingRow key={index} label={`第${index + 1}个分类：`}>
                                {/* 原版 .dec-color-gradient：两色 + 「~」分隔（margin 0 8px、#969799），无重置链 */}
                                <div style={{ display: "flex", alignItems: "center" }}>
                                    <AlphaColorInput value={c.colorA ?? ""} onChange={(v) => setColorPair(index, { colorA: v })} />
                                    <span style={{ margin: "0 8px", color: "#969799" }}>~</span>
                                    <AlphaColorInput value={c.colorB ?? ""} onChange={(v) => setColorPair(index, { colorB: v })} />
                                </div>
                            </SettingRow>
                        ))}
                        <ColorField data={data} onChange={onChange} field="titleColor" label="分组标题色" resetValue="#ffffff" />
                    </SettingGroup>
                    <ModuleStyleSettingsField data={data} onChange={onChange} type="categoryRec" />
                </>
            </SettingsTabs>
        </div>
    );
}
