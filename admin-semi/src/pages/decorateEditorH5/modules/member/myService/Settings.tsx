/**
 * 我的服务 设置（对齐 view/admin myService/Settings.vue）
 *
 * 页签 内容设置/扩展设置。内容：模块标题（maxlength 10）→ 布局样式（列表版/九宫格）→
 * 图标圆角滑杆（max 50）→ 默认服务（接口 /decorate/decorateDiscrete/memberDecorateData 的 menus，
 * 与后台「会员中心装修」页同源；点击标签追加到 serviceList，同原版 addTag）→ 服务列表 PicList；
 * 扩展：CommonFrameEdit 全量。原版打开设置时 serviceList 为空自动以 menus 填充——本实现不静默改数据，未复刻（见摘要）。
 */
import { useEffect, useState } from "react";
import { Tag, Spin } from "@douyinfe/semi-ui";
import { apiGet } from "../../../../../api";
import { SettingGroup, TextField, SegmentedRadio, RangeField, DecDivider, PicListField, FrameSettingsField, RadioRowField } from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";

export default function MyServiceSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const [menus, setMenus] = useState<any[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        let alive = true;
        apiGet<any>("/decorate/decorateDiscrete/memberDecorateData")
            .then((res: any) => {
                if (!alive) return;
                const raw = res?.menus ?? res?.data?.menus ?? [];
                setMenus(Array.isArray(raw) ? raw : Object.values(raw || {}));
            })
            .catch(() => undefined)
            .finally(() => {
                if (alive) setLoading(false);
            });
        return () => {
            alive = false;
        };
    }, []);

    const addTag = (item: any) => {
        const list = Array.isArray(data?.serviceList) ? [...data.serviceList] : [];
        list.push({ ...item });
        onChange({ serviceList: list });
    };

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <div>
                    <SettingGroup>
                        <TextField data={data} onChange={onChange} field="moduleTitle" label="模块标题" placeholder="我的服务" />
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="layoutStyle"
                            label="布局样式"
                            options={[
                                { value: 1, label: "列表版" },
                                { value: 2, label: "九宫格" }
                            ]}
                        />
                        <RangeField data={data} onChange={onChange} field="iconRadius" label="图标圆角" max={50} />
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="默认服务">
                        <Spin spinning={loading}>
                            <div style={{ display: "flex", flexWrap: "wrap", gap: 6, padding: "4px 0" }}>
                                {menus.map((item: any, i: number) => (
                                    <Tag key={i} size="large" type="light" style={{ cursor: "pointer" }} onClick={() => addTag(item)}>
                                        {item.picTitle}
                                    </Tag>
                                ))}
                                {!loading && !menus.length ? <span style={{ color: "#999", fontSize: 12 }}>暂无默认服务数据</span> : null}
                            </div>
                        </Spin>
                    </SettingGroup>
                    <PicListField data={data} onChange={onChange} field="serviceList" label="服务列表" showText addText="添加服务项" />
                </div>
                <div>
                    <FrameSettingsField data={data} onChange={onChange} />
                </div>
            </SettingsTabs>
        </div>
    );
}
