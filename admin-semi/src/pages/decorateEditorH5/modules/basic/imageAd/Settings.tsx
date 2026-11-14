/**
 * 图片广告1 设置（字段对齐 view/admin imageAd/Settings.vue）
 *
 * picType 说明文案逐字取自原版 Settings.vue「添加图片」组的 pic-change-desc（仅 picType 1~4 有文案，5~7 无）。
 */
import {
    SettingGroup,
    RadioRowField,
    NumberField,
    ColorField,
    PicListField,
    TitleSettingsField,
    FrameSettingsField
} from "../../../shared/settingWidgets";

/** 原版 picType 说明文案（逐字） */
const PIC_TYPE_DESC: Record<number, string> = {
    1: "提示：您可以通过拖拽进行图片排序，单图占屏幕宽为100%，建议宽750px，高度自适应",
    2: "提示：您可以通过拖拽进行图片排序，建议手机端尺寸750x350px，高度统一；PC端宽不小于1500px（PC端选择时轮播默认通宽）",
    3: "提示：您可以通过拖拽进行图片排序，单图占屏幕宽为80%，建议宽度600px，高度统一",
    4: "提示：您可以通过拖拽进行图片排序，单图占屏幕宽为40%，建议宽度300px，高度统一"
};

export default function ImageAdSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const picTypeDesc = PIC_TYPE_DESC[Number(data?.picType)];
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="模板样式">
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="picType"
                    label="选择模板"
                    options={[
                        { value: 1, label: "一行一个" },
                        { value: 2, label: "轮播海报" },
                        { value: 3, label: "横滑大图" },
                        { value: 4, label: "横滑小图" },
                        { value: 5, label: "一行两个" },
                        { value: 6, label: "一行三个" },
                        { value: 7, label: "一行四个" }
                    ]}
                />
                {Number(data?.picType) === 2 && (
                    <>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="swiperPreView"
                            label="轮播行显示数量"
                            options={[
                                { value: 1, label: "1个" },
                                { value: 2, label: "2个" },
                                { value: 3, label: "3个" }
                            ]}
                        />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="picPageType"
                            label="指示器样式"
                            options={[
                                { value: 1, label: "样式一" },
                                { value: 2, label: "样式二" },
                                { value: 3, label: "样式三" }
                            ]}
                        />
                        <ColorField data={data} onChange={onChange} field="swiperPageColor" label="指示器颜色" />
                    </>
                )}
                <NumberField data={data} onChange={onChange} field="imgPadding" label="图片边距" suffix="px" max={30} />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="picRadioStyle"
                    label="图片边角"
                    options={[
                        { value: 1, label: "直角" },
                        { value: 2, label: "圆角" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="isFluxWidth"
                    label="是否通宽排版(仅PC端有效)"
                    options={[
                        { value: 1, label: "是" },
                        { value: 0, label: "否" }
                    ]}
                />
            </SettingGroup>
            {picTypeDesc ? (
                <div className="pic-change-desc" style={{ color: "#999", fontSize: 12, lineHeight: "18px", margin: "4px 0 8px" }}>
                    {picTypeDesc}
                </div>
            ) : null}
            <PicListField data={data} onChange={onChange} field="picList" label="图片广告" />
            <TitleSettingsField data={data} onChange={onChange} />
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
