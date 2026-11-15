/**
 * 分类推荐A1 设置（对齐 view/admin mobilePro/categoryRecA1/Settings.vue）
 *
 * 页签：内容设置 / 扩展设置。
 * 内容页：基本设置（PicList 最多 4 个，含标题/描述）+ 参数设置（图片填充/图片圆角 max40/分类标题/副标题颜色）；
 * 扩展页：ModuleStyleEdit（type="categoryRec"）+ ContentStyleEdit（type="categoryA1"）。
 */
import { SettingGroup, RangeField, ColorField, PicListField } from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField, ContentStyleSettingsField, SubTitleSettingsField, PicFillTypeField } from "../../../shared/styleEdits";

export default function CategoryRecA1Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <>
                    <SettingGroup title="基本设置">
                        <PicListField data={data} onChange={onChange} field="picList" label="" max={4} showDesc />
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <PicFillTypeField data={data} onChange={onChange} field="picFillType" />
                        <RangeField data={data} onChange={onChange} field="picRadius" label="图片圆角" max={40} />
                        <SubTitleSettingsField data={data} onChange={onChange} title="分类标题" />
                        <ColorField data={data} onChange={onChange} field="subTitleColor" label="副标题颜色" resetValue="#999999" />
                    </SettingGroup>
                </>
                <>
                    <ModuleStyleSettingsField data={data} onChange={onChange} type="categoryRec" />
                    <ContentStyleSettingsField data={data} onChange={onChange} type="categoryA1" />
                </>
            </SettingsTabs>
        </div>
    );
}
