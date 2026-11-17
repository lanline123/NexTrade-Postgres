/**
 * 空白辅助 设置（字段对齐 view/admin whiteBlank/Settings.vue）
 */
import { ColorField, NumberField } from "../../../shared/settingWidgets";
import { defaultFrame } from "../../../contract/sharedDefaults";

export default function WhiteBlankSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            {/* 原版背景颜色带「重置」链、回 defaultFrame.backgroundColor（#ffffff）；
                此前 allowEmpty={false} 把重置链误隐藏且无重置目标值，已按原版补正 */}
            <ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" resetValue={defaultFrame.backgroundColor} />
            <NumberField data={data} onChange={onChange} field="blankHeight" label="空白高度" suffix="px" max={100} />
        </div>
    );
}
