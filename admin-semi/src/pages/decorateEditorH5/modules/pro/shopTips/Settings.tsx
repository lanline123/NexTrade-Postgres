/**
 * 商品提示 设置（对齐 view/admin mobilePro/shopTips/Settings.vue）
 *
 * 页签：提示设置 / 扩展设置。
 * 内容页：基本设置（PicList 最多 4 个，原版以 watch 截断，此处以 max=4 承接）
 * + 参数设置（每行显示/提示间距 max30/间隔竖线/标题颜色/提示背景色）；
 * 扩展页：ModuleStyleEdit（type="shopTips"，含模块左右外间距项）。
 * 注：原版「标题颜色」重置按钮写回 #333333（与默认值 #00C455 不同，为原版 @click 口径），此处照原版。
 */
import { SettingGroup, RadioRowField, RangeField, ColorField, PicListEditorField, SegmentedRadio, SettingRow } from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField } from "../../../shared/styleEdits";

export default function ShopTipsSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["提示设置", "扩展设置"]}>
                <>
                    <SettingGroup title="基本设置">
                        <div style={{ color: "#999", fontSize: 12, lineHeight: 1.6, padding: "0 16px 4px" }}>提示：您可以通过拖拽进行排序，最多添加4个</div>
                        <PicListEditorField data={data} onChange={onChange} field="picList" label="" max={4} addText="添加提示" />
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <SettingRow label="每行显示" block>
                            <SegmentedRadio
                                value={Number(data?.rowNum ?? 3)}
                                options={[1, 2, 3, 4].map((n) => ({ value: n, label: `${n}个` }))}
                                onChange={(v) => onChange({ rowNum: v })}
                            />
                        </SettingRow>
                        <RangeField data={data} onChange={onChange} field="itemGap" label="提示间距" max={30} />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="showDivider"
                            label="间隔竖线"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "隐藏" }
                            ]}
                        />
                        <ColorField data={data} onChange={onChange} field="titleColor" label="标题颜色" resetValue="#333333" />
                        <ColorField data={data} onChange={onChange} field="itemBackgroundColor" label="提示背景色" resetValue="" />
                    </SettingGroup>
                </>
                <ModuleStyleSettingsField data={data} onChange={onChange} type="shopTips" />
            </SettingsTabs>
        </div>
    );
}
