/**
 * 分类中心V1 设置（字段对齐 view/admin cateCenterV1/Settings.vue）
 *
 * 内容设置：搜索框占位文字 / 展示分类名称 / 显示"推荐"入口 / "推荐"文案；
 * 颜色设置：6 色（留空跟随主题，重置回空串）。
 * 注：showCategoryName / showRecommend 在契约中为 boolean，
 * 故不用数值型 SwitchField（写 1/0），用 SettingRow + Semi Switch 写 boolean。
 */
import { Switch, TextArea } from "@douyinfe/semi-ui";
import { SettingGroup, SettingRow, TextField, ColorField } from "../../../shared/settingWidgets";
import { useI18n } from "../../../../../i18n";

const COLOR_FIELDS: Array<{ field: string; label: string }> = [
    { field: "sideBgColor", label: "一级分类栏背景色" },
    { field: "sideTextColor", label: "一级分类文字色" },
    { field: "sideActiveTextColor", label: "一级分类选中文字色" },
    { field: "sideActiveBgColor", label: "一级分类选中背景色" },
    { field: "groupTitleColor", label: "分组标题色" },
    { field: "categoryNameColor", label: "分类名称色" }
];

function BoolSwitchField({
    data,
    onChange,
    field,
    label
}: {
    data: Record<string, any>;
    onChange: (patch: Record<string, any>) => void;
    field: string;
    label: string;
}) {
    return (
        <SettingRow label={label}>
            <Switch checked={data?.[field] !== false} onChange={(v: boolean) => onChange({ [field]: v })} />
        </SettingRow>
    );
}

export default function CateCenterV1Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const { t } = useI18n();
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="内容设置">
                <TextField data={data} onChange={onChange} field="searchPlaceholder" label="搜索框占位文字" placeholder="请输入搜索框占位文字" />
                <BoolSwitchField data={data} onChange={onChange} field="showCategoryName" label="展示分类名称" />
                <BoolSwitchField data={data} onChange={onChange} field="showRecommend" label={'显示"推荐"入口'} />
                {data?.showRecommend !== false ? (
                    <SettingRow label={'"推荐"文案'} block>
                        <TextArea
                            value={data?.recommendText ?? ""}
                            rows={2}
                            placeholder={t("请输入推荐入口文案")}
                            style={{ width: "100%" }}
                            onChange={(v: string) => onChange({ recommendText: v })}
                        />
                    </SettingRow>
                ) : null}
            </SettingGroup>
            <SettingGroup title="颜色设置">
                {COLOR_FIELDS.map((item) => (
                    <ColorField key={item.field} data={data} onChange={onChange} field={item.field} label={item.label} />
                ))}
            </SettingGroup>
        </div>
    );
}
