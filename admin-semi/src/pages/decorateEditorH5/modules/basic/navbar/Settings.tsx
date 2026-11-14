/**
 * 页面导航 设置（字段覆盖渲染端 navbar/index.tsx 读取的全部字段）
 */
import { SettingGroup, TextField, ColorField } from "../../../shared/settingWidgets";

export default function NavbarSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="页面导航">
                <TextField data={data} onChange={onChange} field="title" label="标题文字" />
                <ColorField data={data} onChange={onChange} field="titleColor" label="标题颜色" />
                <ColorField data={data} onChange={onChange} field="titleBackgroundColor" label="背景颜色" />
            </SettingGroup>
        </div>
    );
}
