/**
 * 分类导航 设置（对齐 view/admin editor/modules/mobile/catNav/Settings.vue）
 *
 * 口径：原版 6 个数值字段均为 el-slider（itemWidth max 50 / logoHeight max 200 /
 * boxPadding·boxPaddingTop·boxPaddingBottom max 30 / itemRadius max 50），本实现 NumberField 即滑杆形态；
 * 「设置分类」为 router-link「前往设置」（新页打开 /decorate-info/mobile-cat-nav/list）。
 * 原版与本模块均无「搜索框高/高度」字段（logoHeight 的标签为「LOGO高度」），审计该条无对应字段可改。
 */
import { SettingGroup, SettingRow, ColorField, NumberField, RadioRowField, SinglePicField, TextField } from "../../../shared/settingWidgets";

export default function CatNavSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="分类导航">
                <NumberField data={data} onChange={onChange} field="itemWidth" label="分类名宽度(%)" max={50} suffix="%" />
                <ColorField data={data} onChange={onChange} field="textColor" label="文字颜色" />
                <ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" />
                <SinglePicField data={data} onChange={onChange} field="navBackgroundPic" label="背景图" />
                <SettingRow label="设置分类">
                    <a className="btn-link" href="/decorate-info/mobile-cat-nav/list" target="_blank" rel="noreferrer" style={{ color: '#155bd4', fontSize: 14 }}>
                        前往设置
                    </a>
                </SettingRow>
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="isGanged"
                    label="搜索联动"
                    options={[
                        { value: 1, label: '联动' },
                        { value: 0, label: '不联动' },
                    ]}
                />
                <div style={{ color: '#646a73', fontSize: 12, margin: '4px 0 8px' }}>
                    设置为联动时，顶部会替换为LOGO栏+搜索栏+分类导航。屏幕下滑时LOGO栏会消失，搜索栏会替换至LOGO栏位置（请不要在该组件前再添加其它组件）。
                </div>
                <SinglePicField data={data} onChange={onChange} field="logoPic" label="LOGO图" />
                <div style={{ color: '#646a73', fontSize: 12, margin: '4px 0 8px' }}>提示：LOGO图建议尺寸为 1374px*324px，否则可能显示异常。</div>
                <NumberField data={data} onChange={onChange} field="logoHeight" label="LOGO高度" max={200} suffix="px" />
                <TextField data={data} onChange={onChange} field="searchText" label="搜索框文字" />
                <NumberField data={data} onChange={onChange} field="boxPadding" label="搜索框边距" max={30} suffix="px" />
                <ColorField data={data} onChange={onChange} field="searchTextColor" label="搜索文字颜色" />
                <ColorField data={data} onChange={onChange} field="itemBackgroundColor" label="搜索组件背景" />
                <NumberField data={data} onChange={onChange} field="itemRadius" label="搜索组件圆角" max={50} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxPaddingTop" label="上边距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxPaddingBottom" label="下边距" max={30} suffix="px" />
            </SettingGroup>
        </div>
    );
}
