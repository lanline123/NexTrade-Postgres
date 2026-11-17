/**
 * 组合推荐 设置（对齐 view/admin mobilePro/comboSuggestions/Settings.vue）
 *
 * 页签：内容设置 / 扩展设置。
 * 内容页：ComboGroupEdit 分组卡（标题头图/大标题/小标题/描述/颜色/查看更多链接/分组内容
 * 图片|商品；图片组走 PicList，商品组 pruductList 为 ProductInfoSelect 数据选择器，未接入，
 * 按口径保留只读提示）+ 商品参数设置（常规价格颜色/价格背景/价格圆角 max30/价格字号 10-24/商品圆角 max20）。
 * 扩展页：样式设置（渐变方向/渐变色对/分组大标题色/轮播图标题色/指示数颜色/内容圆角 max10/内容投影）
 * + ModuleStyleEdit（type="ComboSuggestionsEdit"）+ 自动轮播/轮播间隔 max10。
 * 注：原版 Settings 中活动价标签/活动价/售罄图标/轮播方向等字段在源码中已被注释掉（不渲染），
 * 此处同样不提供编辑入口，默认值保留在 defaults 中。
 */
import AlphaColorInput from "../../../../../components/AlphaColorInput";
import { SettingGroup, SettingRow, RadioRowField, RangeField, ColorField, TextField, PicListField, PicSelectField, PicLinkField } from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField, GroupCardList } from "../../../shared/styleEdits";

const EMPTY_GROUP = {
    titleImage: { picUrl: "", picThumb: "" },
    groupTitle: "",
    groupSubTitle: "",
    groupDesc: "",
    groupColor: "",
    groupType: "image",
    picList: [],
    pruductList: [],
    moreLink: { link: { link: "", title: "" }, linkTitle: "" }
};

function ComboGroupEditor({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const list: Array<Record<string, any>> = Array.isArray(data?.comboGroups) ? data.comboGroups : [];
    const setList = (next: Array<Record<string, any>>) => onChange({ comboGroups: next });
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
            onAdd={() => setList([...list, JSON.parse(JSON.stringify(EMPTY_GROUP))])}
            onRemove={(index) => setList(list.filter((_, i) => i !== index))}
            onMove={moveItem}
            renderItem={(item, index) => {
                const itemProps = { data: item, onChange: (patch: Record<string, any>) => patchItem(index, patch) };
                return (
                    <>
                        <PicSelectField {...itemProps} field="titleImage" label="标题头图：宽150内,高30" />
                        <TextField {...itemProps} field="groupTitle" label="分组大标题" placeholder="请输入标题内容" />
                        <TextField {...itemProps} field="groupSubTitle" label="分组小标题" placeholder="请输入标题内容" />
                        <TextField {...itemProps} field="groupDesc" label="分组描述" placeholder="请输入描述内容" />
                        <ColorField {...itemProps} field="groupColor" label="分组颜色" resetValue="" />
                        <SettingRow label="查看更多链接">
                            {/* 原版 comboGroup/Edit 以 SelectLink 绑定 element.moreLink 整体，选中即回写完整 LinkType */}
                            <PicLinkField
                                value={typeof item.moreLink === "string" ? { link: item.moreLink, title: "" } : item.moreLink ?? null}
                                onChange={(v) => patchItem(index, { moreLink: v })}
                            />
                        </SettingRow>
                        <RadioRowField
                            {...itemProps}
                            field="groupType"
                            label="分组内容"
                            options={[
                                { value: "image", label: "图片" },
                                { value: "product", label: "商品" }
                            ]}
                        />
                        {item.groupType === "product" ? (
                            <div style={{ color: "#646a73", fontSize: 12, lineHeight: 1.6, padding: "4px 16px 10px" }}>
                                商品选择器（ProductInfoSelect 数据选择组件）尚未接入本编辑器，已选商品（{(item.pruductList || []).length} 个）数据保持不变。
                            </div>
                        ) : (
                            <PicListField {...itemProps} field="picList" label="" max={20} addText="添加一组" />
                        )}
                    </>
                );
            }}
        />
    );
}

export default function ComboSuggestionsSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <>
                    <ComboGroupEditor data={data} onChange={onChange} />
                    <SettingGroup title="商品参数设置">
                        <ColorField data={data} onChange={onChange} field="regularPriceColor" label="常规价格颜色" resetValue="" />
                        <ColorField data={data} onChange={onChange} field="regularPriceBgColor" label="价格背景颜色" resetValue="" />
                        <RangeField data={data} onChange={onChange} field="regularPriceRadius" label="价格圆角半径" max={30} />
                        <RangeField data={data} onChange={onChange} field="regularPriceSize" label="价格文字大小" min={10} max={24} />
                        <RangeField data={data} onChange={onChange} field="imageRadius" label="商品圆角半径" max={20} />
                    </SettingGroup>
                </>
                <>
                    <SettingGroup title="样式设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="backgroundGradientType"
                            label="背景渐变方向"
                            options={[
                                { value: "upDown", label: "上下" },
                                { value: "leftRight", label: "左右" },
                                { value: "diagonal", label: "斜向" },
                                { value: "purity", label: "单色" }
                            ]}
                        />
                        <SettingRow label="背景颜色">
                            <div className="dec-color-group">
                                <div className="dec-color-button">
                                    <span className="dec-color-trigger">
                                        <AlphaColorInput value={String(data?.gradientColorA ?? "")} onChange={(v) => onChange({ gradientColorA: v })} />
                                    </span>
                                    <span style={{ margin: "0 10px" }}>-</span>
                                    <span className="dec-color-trigger">
                                        <AlphaColorInput value={String(data?.gradientColorB ?? "")} onChange={(v) => onChange({ gradientColorB: v })} />
                                    </span>
                                </div>
                            </div>
                        </SettingRow>
                        <ColorField data={data} onChange={onChange} field="groupTitleColor" label="分组大标题颜色" resetValue="" />
                        <ColorField data={data} onChange={onChange} field="bannerTittleColor" label="轮播图标题颜色" resetValue="#ffffff" />
                        <ColorField data={data} onChange={onChange} field="bannerDotColor" label="轮播图指示数颜色" resetValue="#ffffff" />
                        <RangeField data={data} onChange={onChange} field="groupRadius" label="内容圆角" max={10} />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="groupShadow"
                            label="内容投影"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "不显示" }
                            ]}
                        />
                    </SettingGroup>
                    <ModuleStyleSettingsField data={data} onChange={onChange} type="ComboSuggestionsEdit" />
                    <SettingGroup title="轮播设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="autoPlay"
                            label="自动轮播"
                            options={[
                                { value: 1, label: "开启" },
                                { value: 0, label: "关闭" }
                            ]}
                        />
                        <RangeField data={data} onChange={onChange} field="interval" label="轮播间隔" max={10} />
                    </SettingGroup>
                </>
            </SettingsTabs>
        </div>
    );
}
