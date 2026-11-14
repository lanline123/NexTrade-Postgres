/**
 * 分类中心 V2 设置（字段与渲染端 cateCenterV2Defaults 一一对应；颜色留空 = 跟随主题）
 */
import { SettingGroup, TextField, SwitchField, RadioRowField, NumberField, ColorField } from "../../../shared/settingWidgets";

export default function CateCenterV2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="基础">
                <TextField data={data} onChange={onChange} field="searchPlaceholder" label="搜索提示文字" />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="styleType"
                    label="商品样式"
                    options={[
                        { value: 2, label: "双列网格" },
                        { value: 3, label: "图文列表" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="subTabStyle"
                    label="二级标签风格"
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
                        { value: 1, label: "居左" },
                        { value: 2, label: "居中" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="textWeight"
                    label="文字粗细"
                    options={[
                        { value: 1, label: "常规" },
                        { value: 2, label: "加粗" }
                    ]}
                />
                <NumberField data={data} onChange={onChange} field="goodsNameRow" label="商品名称行数" min={1} max={2} />
                <SwitchField data={data} onChange={onChange} field="showName" label="显示商品名称" />
                <NumberField data={data} onChange={onChange} field="buyBtnStyle" label="购买按钮样式(0-8)" min={0} max={8} />
            </SettingGroup>
            <SettingGroup title="颜色">
                <ColorField data={data} onChange={onChange} field="themeColor" label="主题色" />
                <ColorField data={data} onChange={onChange} field="sideBgColor" label="左侧栏背景色" />
                <ColorField data={data} onChange={onChange} field="sideTextColor" label="左侧栏文字色" />
                <ColorField data={data} onChange={onChange} field="sideActiveTextColor" label="左侧选中文字色" />
                <ColorField data={data} onChange={onChange} field="sideActiveBgColor" label="左侧选中背景色" />
                <ColorField data={data} onChange={onChange} field="sideIndicatorColor" label="左侧指示条色" />
                <ColorField data={data} onChange={onChange} field="subTabTextColor" label="二级标签文字色" />
                <ColorField data={data} onChange={onChange} field="subTabBgColor" label="二级标签背景色" />
                <ColorField data={data} onChange={onChange} field="subTabActiveTextColor" label="标签选中文字色" />
                <ColorField data={data} onChange={onChange} field="subTabActiveBgColor" label="标签选中背景色" />
                <ColorField data={data} onChange={onChange} field="goodsListBgColor" label="商品区背景色" />
                <ColorField data={data} onChange={onChange} field="productTitleColor" label="商品标题色" />
                <ColorField data={data} onChange={onChange} field="priceColor" label="价格颜色" />
                <ColorField data={data} onChange={onChange} field="buyBtnColor" label="购买按钮色" />
            </SettingGroup>
        </div>
    );
}
