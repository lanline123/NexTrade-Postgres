/**
 * 分类中心V2 设置（字段对齐 view/admin cateCenterV2/Settings.vue）
 *
 * 内容设置：展示布局 / 二级分类标签风格 / 文本对齐 / 文字粗细 / 商品名称行数 /
 * 显示商品名称 / 购买按钮样式（原版为标题在上的整宽块 + 三列等宽单选组）；
 * 颜色设置：14 色（留空跟随主题，重置回空串）。
 */
import { SettingGroup, SettingRow, SegmentedRadio, RadioRowField, ColorField } from "../../../shared/settingWidgets";

const COLOR_FIELDS: Array<{ field: string; label: string }> = [
    { field: "themeColor", label: "主题色（价格、按钮、选中态）" },
    { field: "sideBgColor", label: "一级分类栏背景色" },
    { field: "sideTextColor", label: "一级分类文字色" },
    { field: "sideActiveTextColor", label: "一级分类选中文字色" },
    { field: "sideActiveBgColor", label: "一级分类选中背景色" },
    { field: "sideIndicatorColor", label: "一级分类选中指示条颜色" },
    { field: "subTabTextColor", label: "二级标签文字色" },
    { field: "subTabBgColor", label: "二级标签背景色" },
    { field: "subTabActiveTextColor", label: "二级标签选中文字色" },
    { field: "subTabActiveBgColor", label: "二级标签选中背景色" },
    { field: "goodsListBgColor", label: "商品区背景色" },
    { field: "productTitleColor", label: "商品标题色" },
    { field: "priceColor", label: "商品价格色" },
    { field: "buyBtnColor", label: "购买按钮色" }
];

const BUY_BTN_OPTIONS = [
    { value: 0, label: "不显示" },
    { value: 1, label: "样式一" },
    { value: 2, label: "样式二" },
    { value: 3, label: "样式三" },
    { value: 4, label: "样式四" },
    { value: 5, label: "样式五" },
    { value: 6, label: "样式六" },
    { value: 7, label: "样式七" },
    { value: 8, label: "样式八" }
];

export default function CateCenterV2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="内容设置">
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="styleType"
                    label="展示布局"
                    options={[
                        { value: 2, label: "双列网格" },
                        { value: 3, label: "图文列表" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="subTabStyle"
                    label="二级分类标签风格"
                    options={[
                        { value: 1, label: "实心高亮" },
                        { value: 2, label: "描边高亮" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="textAlign"
                    label="文本对齐"
                    options={[
                        { value: 1, label: "左对齐" },
                        { value: 2, label: "居中对齐" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="textWeight"
                    label="文字粗细"
                    options={[
                        { value: 1, label: "常规体" },
                        { value: 2, label: "加粗体" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="goodsNameRow"
                    label="商品名称行数"
                    options={[
                        { value: 1, label: "1行" },
                        { value: 2, label: "2行" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="showName"
                    label="显示商品名称"
                    options={[
                        { value: 1, label: "显示" },
                        { value: 0, label: "不显示" }
                    ]}
                />
                <SettingRow label="购买按钮样式" block>
                    <SegmentedRadio value={data?.buyBtnStyle ?? 1} options={BUY_BTN_OPTIONS} onChange={(v) => onChange({ buyBtnStyle: v })} />
                </SettingRow>
            </SettingGroup>
            <SettingGroup title="颜色设置">
                {COLOR_FIELDS.map((item) => (
                    <ColorField key={item.field} data={data} onChange={onChange} field={item.field} label={item.label} />
                ))}
            </SettingGroup>
        </div>
    );
}
