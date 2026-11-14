/**
 * 视频号 设置（字段覆盖渲染端 videoNumber/index.tsx 读取的可配置字段）
 */
import { SettingGroup, RadioRowField, TextField, ColorField, PicListField, FrameSettingsField } from "../../../shared/settingWidgets";

export default function VideoNumberSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const isLive = data?.contentType === "live";
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="视频号">
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="contentType"
                    label="内容类型"
                    options={[
                        { value: "video", label: "视频" },
                        { value: "live", label: "直播" }
                    ]}
                />
                <TextField data={data} onChange={onChange} field="videoNumberId" label="视频号ID" placeholder="finderUserName，如 sphxxxx" />
                <TextField data={data} onChange={onChange} field="videoId" label={isLive ? "直播ID" : "视频ID"} placeholder="feedId" />
                {isLive && (
                    <>
                        <TextField data={data} onChange={onChange} field="liveTitle" label="直播标题" placeholder="这场直播特别有料" />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="titleColorType"
                            label="标题颜色"
                            options={[
                                { value: "default", label: "默认" },
                                { value: "theme", label: "主题色" },
                                { value: "custom", label: "自定义" }
                            ]}
                        />
                        {data?.titleColorType === "custom" && <ColorField data={data} onChange={onChange} field="titleColor" label="标题颜色值" />}
                        <PicListField data={data} onChange={onChange} field="productImages" label="直播商品图（最多展示3张）" max={3} />
                    </>
                )}
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} label="边框设置" />
        </div>
    );
}
