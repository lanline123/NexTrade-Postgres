/**
 * 分类中心 V1 设置（字段与渲染端 cateCenterV1Defaults 一一对应；颜色留空 = 跟随主题）
 */
import { SettingGroup, TextField, SwitchField, ColorField } from "../../../shared/settingWidgets";

export default function CateCenterV1Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="搜索与入口">
                <TextField data={data} onChange={onChange} field="searchPlaceholder" label="搜索提示文字" />
                <SwitchField data={data} onChange={onChange} field="showRecommend" label="显示推荐入口" />
                <TextField data={data} onChange={onChange} field="recommendText" label="推荐文案" />
                <SwitchField data={data} onChange={onChange} field="showCategoryName" label="显示分类名称" />
            </SettingGroup>
            <SettingGroup title="颜色">
                <ColorField data={data} onChange={onChange} field="themeColor" label="主题色" />
                <ColorField data={data} onChange={onChange} field="sideBgColor" label="左侧栏背景色" />
                <ColorField data={data} onChange={onChange} field="sideTextColor" label="左侧栏文字色" />
                <ColorField data={data} onChange={onChange} field="sideActiveTextColor" label="左侧选中文字色" />
                <ColorField data={data} onChange={onChange} field="sideActiveBgColor" label="左侧选中背景色" />
                <ColorField data={data} onChange={onChange} field="groupTitleColor" label="分组标题色" />
                <ColorField data={data} onChange={onChange} field="categoryNameColor" label="分类名称色" />
            </SettingGroup>
        </div>
    );
}
