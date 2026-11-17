/**
 * 商品2 设置（对齐 view/admin editor/modules/mobile/product2/Settings.vue）
 *
 * 商品来源 + 图片背景颜色 / 商品卡圆角 / 内外边距。
 */
import { SettingGroup, ColorField, NumberField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";
import { defaultFrame } from "../../../contract/sharedDefaults";

export default function Product2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="商品2">
                <ProductsSourceField data={data} onChange={onChange} />
            </SettingGroup>
            <SettingGroup title="样式设置">
                {/* 原版图片背景颜色「重置」回 defaultFrame.backgroundColor（#ffffff），非清空 */}
                <ColorField data={data} onChange={onChange} field="imageBackgroundColor" label="图片背景颜色" resetValue={defaultFrame.backgroundColor} />
                <NumberField data={data} onChange={onChange} field="goodsRadius" label="商品卡圆角" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxRadius" label="圆角" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="innerPadding" label="内边距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxPadding" label="左右边距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxPaddingTop" label="上边距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxPaddingBottom" label="下边距" max={30} suffix="px" />
            </SettingGroup>
        </div>
    );
}
