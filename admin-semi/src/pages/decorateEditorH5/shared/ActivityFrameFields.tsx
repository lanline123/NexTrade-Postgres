/**
 * 活动商品框设置（移植自 view/admin src/modules/src/commonActivityFrame/Edit.vue，section=all、type 非 product）
 *
 * 字段全部位于模块顶层（原版 v-model 绑整个 module）：选择模板 style / 商品卡样式 goodsStyle /
 * 瀑布流 waterfall（原版带 v-if="frame.style === 2"，仅模板「一行两个」时显示）/ 商品卡边角 goodsRadioStyle（useGoodsRadiusSlider 时改为商品卡圆角 goodsRadius 滑杆）/
 * 文本对齐 textAlign（hideTextAlign 可隐）/ 文字粗细 textWeight / 商品名称行数 goodsNameRow /
 * 商品信息边距 goodsNamePadding / 显示商品名称 showName / 显示商品价格 showPrice /
 * 商品边距 goodsPadding（max 50）/ 购买按钮样式 buyBtnStyle（hideBuyBtnStyle 可隐）/
 * 容器：背景颜色 backgroundColor（重置 defaultFrame.backgroundColor，transparentBackground 时重置为空）/
 * 圆角 boxRadius / 内边距 innerPadding / 左右边距 boxPadding / 上边距 boxPaddingTop / 下边距 boxPaddingBottom。
 * 原版 selectLabel 中有 showBrief 但模板未渲染该组——此处同样不渲染（快照口径）。
 */
import { SettingGroup, SegmentedRadio, RangeField, ColorField, RadioRowField } from "./settingWidgets";

interface Props {
    data: Record<string, any>;
    onChange: (patch: Record<string, any>) => void;
    hideTextAlign?: boolean;
    hideBuyBtnStyle?: boolean;
    useGoodsRadiusSlider?: boolean;
    transparentBackground?: boolean;
    /** 隐藏「选择模板」（seckill2 的模板由其自身 Settings 固定 10 项横滑，不走此组） */
    hideStyle?: boolean;
    /** 隐藏瀑布流（seckill2 defaults 无 waterfall 字段） */
    hideWaterfall?: boolean;
}

const SHOW = [
    { value: 1, label: "显示" },
    { value: 0, label: "不显示" }
];

export default function ActivityFrameFields({
    data,
    onChange,
    hideTextAlign,
    hideBuyBtnStyle,
    useGoodsRadiusSlider,
    transparentBackground,
    hideStyle,
    hideWaterfall
}: Props) {
    return (
        <>
            <SettingGroup>
                {!hideStyle ? (
                    <RadioRowField
                        data={data}
                        onChange={onChange}
                        field="style"
                        label="选择模板"
                        options={[
                            { value: 1, label: "一行一个" },
                            { value: 2, label: "一行两个" },
                            { value: 5, label: "详细列表" },
                            { value: 6, label: "横向滑动" }
                        ]}
                    />
                ) : null}
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="goodsStyle"
                    label="商品卡样式"
                    options={[
                        { value: 1, label: "标准" },
                        { value: 2, label: "投影" },
                        { value: 3, label: "描边" },
                        { value: 4, label: "透明底" }
                    ]}
                />
                {/* 原版 CommonActivityFrameEdit：瀑布流组带 v-if="frame.style === 2"，仅一行两个时显示 */}
                {!hideWaterfall && Number(data?.style) === 2 ? (
                    <RadioRowField
                        data={data}
                        onChange={onChange}
                        field="waterfall"
                        label="瀑布流"
                        options={[
                            { value: 1, label: "启用" },
                            { value: 0, label: "否" }
                        ]}
                    />
                ) : null}
                {useGoodsRadiusSlider ? (
                    <RangeField data={data} onChange={onChange} field="goodsRadius" label="商品卡圆角" max={30} />
                ) : (
                    <RadioRowField
                        data={data}
                        onChange={onChange}
                        field="goodsRadioStyle"
                        label="商品卡边角"
                        options={[
                            { value: 1, label: "直角" },
                            { value: 2, label: "圆角" }
                        ]}
                    />
                )}
                {!hideTextAlign ? (
                    <RadioRowField
                        data={data}
                        onChange={onChange}
                        field="textAlign"
                        label="文本对齐"
                        options={[
                            { value: 1, label: "左对齐" },
                            { value: 2, label: "居中对齐" }
                        ]}
                    />
                ) : null}
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="textWeight"
                    label="文字粗细"
                    options={[
                        { value: 1, label: "常规体" },
                        { value: 2, label: "加粗体" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="goodsNameRow"
                    label="商品名称行数"
                    options={[
                        { value: 1, label: "1行" },
                        { value: 2, label: "2行" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="goodsNamePadding"
                    label="商品信息边距"
                    options={[
                        { value: 1, label: "有边距" },
                        { value: 0, label: "无边距" }
                    ]}
                />
                <RadioRowField data={data} onChange={onChange} field="showName" label="显示商品名称" options={SHOW} />
                <RadioRowField data={data} onChange={onChange} field="showPrice" label="显示商品价格" options={SHOW} />
                <RangeField data={data} onChange={onChange} field="goodsPadding" label="商品边距" max={50} />
                {!hideBuyBtnStyle ? (
                    <RadioRowField
                        data={data}
                        onChange={onChange}
                        field="buyBtnStyle"
                        label="购买按钮样式"
                        options={[
                            { value: 0, label: "不显示" },
                            { value: 1, label: "样式一" },
                            { value: 2, label: "样式二" },
                            { value: 3, label: "样式三" },
                            { value: 4, label: "样式四" },
                            { value: 5, label: "样式五" },
                            { value: 6, label: "样式六" },
                            { value: 7, label: "样式七" },
                            { value: 8, label: "样式八" }
                        ]}
                    />
                ) : null}
            </SettingGroup>
            <SettingGroup>
                <ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" resetValue={transparentBackground ? "" : "#ffffff"} />
                <RangeField data={data} onChange={onChange} field="boxRadius" label="圆角" max={30} />
                <RangeField data={data} onChange={onChange} field="innerPadding" label="内边距" max={30} />
                <RangeField data={data} onChange={onChange} field="boxPadding" label="左右边距" max={30} />
                <RangeField data={data} onChange={onChange} field="boxPaddingTop" label="上边距" max={30} />
                <RangeField data={data} onChange={onChange} field="boxPaddingBottom" label="下边距" max={30} />
            </SettingGroup>
        </>
    );
}
