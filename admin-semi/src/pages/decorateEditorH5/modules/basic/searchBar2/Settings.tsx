/**
 * 商品搜索2 设置（对齐 view/admin editor/modules/mobile/searchBar2/Settings.vue）
 *
 * 原版结构与 searchBar 完全一致：搜索框文字 + CommonFrameEdit 专属形态
 * （type=searchBar：文字颜色/组件背景/组件圆角 max50/组件高度 max50 + 背景颜色/左右边距/上边距/下边距）。
 * 口径（以源码为准）：原版 searchBar2 的 Settings.vue / 旧 Edit.vue / index.ts defaults 中
 * 均不存在「海外设置组」（搜索按钮背景/文字颜色/显示搜索按钮）——该组字段属 banner 的
 * BannerStyle（bannerStyle.showSearchBtn 等），searchBar2 无此数据键；按「新增字段必须
 * 原版同名同默认」约束不凭空添加，详见批次回报。
 */
import { SettingGroup, TextField, FrameSettingsField } from "../../../shared/settingWidgets";

export default function SearchBar2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="商品搜索2">
                <TextField data={data} onChange={onChange} field="text" label="搜索框文字" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} variant="searchBar" />
        </div>
    );
}
