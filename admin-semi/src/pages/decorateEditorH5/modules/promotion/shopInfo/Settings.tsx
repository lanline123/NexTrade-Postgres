/**
 * 店铺信息 设置（字段覆盖渲染端 shopInfo/index.tsx 读取的可配置字段）
 */
import { SettingGroup, RadioRowField, NumberField, SinglePicField } from "../../../shared/settingWidgets";

export default function ShopInfoSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="店铺信息">
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="style"
                    label="展示风格"
                    options={[
                        { value: 1, label: "样式一" },
                        { value: 2, label: "样式二" },
                        { value: 3, label: "样式三" },
                        { value: 4, label: "样式四" }
                    ]}
                />
                <NumberField data={data} onChange={onChange} field="backgroundDefault" label="背景图序号" min={0} max={10} />
                <div style={{ color: "#646a73", fontSize: 12, marginBottom: 8 }}>0 表示使用下方自定义背景图；1~10 为系统内置背景图。</div>
                {Number(data?.backgroundDefault) === 0 && <SinglePicField data={data} onChange={onChange} field="customPic" label="自定义背景图" />}
            </SettingGroup>
        </div>
    );
}
