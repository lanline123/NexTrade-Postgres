/**
* 倒计时 设置（对齐 view/admin mobilePro/countdown/Settings.vue）
*
* 页签：商品设置 / 扩展设置。
* 商品设置：ProductInfoSelect（数据选择器未接入 → 只读提示）+ 模块颜色风格（8 色，
* 切换时按原版 watch 联动写回倒计时背景图/商品价格色/内容区渐变色，ModuleTheme 数据逐字取自原版 editIndex）
* + 模块标题设置（moduleTitle 嵌套）+ 参数设置（countdownData 嵌套：显示开关/倒计时图 PicSelect/
* 类型/每日时间/指定时间/显示内容多选/顶部距离 max100/上浮距离 max40 + 每屏数量/图片比例/填充/
* 对齐/商品间距 max10/商品圆角 max20/底部圆角/商品标题/商品价格/价格粗细/划线价）；
* 扩展设置：颜色设置（moduleColor 8 色）+ ModuleStyleEdit（type="categoryRec"）+ ContentStyleEdit（type="countdown"）。
* 注：时间选择器用 Semi TimePicker / DatePicker 承接，写回原版时间戳数值形状；跳转链接走共享 PicLinkField 写回 titleLink.link。
* 标题对齐/价格粗细原版为 iconfont-admin 图标钮，现走共享视觉选择器
* 。
*/
import { Checkbox, DatePicker, TimePicker} from "@douyinfe/semi-ui";
import {
SettingGroup,
SettingRow,
RadioRowField,
RangeField,
ColorField,
TextField,
PicSelectField,
PicLinkField,
DecDivider,
SegmentedRadio
} from "../../../shared/settingWidgets";
import { TemplateIconRadioRowField} from "../../../shared/templateIcon";
import { SettingsTabs, ModuleStyleSettingsField, ContentStyleSettingsField, PicFillTypeField} from "../../../shared/styleEdits";

type NestedProps = { data: Record<string, any>; onChange: (patch: Record<string, any>) => void};
function nested(data: Record<string, any>, onChange: (patch: Record<string, any>) => void, key: string): NestedProps {
const sub = (data?.[key]?? {}) as Record<string, any>;
return { data: sub, onChange: (patch: Record<string, any>) => onChange({ [key]: {...sub,...patch}})};
}

/** 原版 editIndex.ts ModuleTheme（逐字移植，含各风格倒计时背景图） */
const MODULE_THEME: Record<string, { textColor: string; gradientColorA: string; gradientColorB: string; countdownBackgroundPic: any}> = {
orange: {
textColor: "#FF3E02",
gradientColorA: "#ff3e02",
gradientColorB: "#ff7b19",
countdownBackgroundPic: { picId: 1480, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908843qTonhlo5Iw9kRJdLN1.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908843qTonhlo5Iw9kRJdLN1.png", picName: "orange"}
},
red: {
textColor: "#FF1513",
gradientColorA: "#ff1513",
gradientColorB: "#ff856c",
countdownBackgroundPic: { picId: 1487, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908847AN8bNRFsYO681hzol5.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908847AN8bNRFsYO681hzol5.png", picName: "red"}
},
blue: {
textColor: "#4499E6",
gradientColorA: "#4499e6",
gradientColorB: "#7faff5",
countdownBackgroundPic: { picId: 1484, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17319088431TGhqdiHX3OhNWUiJJ.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17319088431TGhqdiHX3OhNWUiJJ.png", picName: "blue"}
},
green: {
textColor: "#59c12d",
gradientColorA: "#59c12d",
gradientColorB: "#64d135",
countdownBackgroundPic: { picId: 1485, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908843bO258L7j1eq3dPeJTM.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908843bO258L7j1eq3dPeJTM.png", picName: "green"}
},
purple: {
textColor: "#7649d7",
gradientColorA: "#7649d7",
gradientColorB: "#8456e3",
countdownBackgroundPic: { picId: 1486, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908846cfUWqho3wrh0l9xD0A.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908846cfUWqho3wrh0l9xD0A.png", picName: "purple"}
},
grey: {
textColor: "#373737",
gradientColorA: "#373737",
gradientColorB: "#6d6d6d",
countdownBackgroundPic: { picId: 1481, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908843sz1aJohktlFTfhDGsY.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908843sz1aJohktlFTfhDGsY.png", picName: "grey"}
},
chocolate: {
textColor: "#4e362e",
gradientColorA: "#4e362e",
gradientColorB: "#603929",
countdownBackgroundPic: { picId: 1482, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908843dIqndLbjKR48vEYrS0.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908843dIqndLbjKR48vEYrS0.png", picName: "chocolate"}
},
brown: {
textColor: "#a67e5a",
gradientColorA: "#a67e5a",
gradientColorB: "#c19270",
countdownBackgroundPic: { picId: 1483, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908843tbYlP36vnX8NbNyvHG.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908843tbYlP36vnX8NbNyvHG.png", picName: "brown"}
}
};

export default function CountdownSettings({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
const mt = nested(data, onChange, "moduleTitle");
const cd = nested(data, onChange, "countdownData");
const mc = nested(data, onChange, "moduleColor");
const showCountdown = Number(cd.data.showCountdown) === 1;
const timeContent: string[] = Array.isArray(cd.data.timeContent)? cd.data.timeContent: [];
const toggleTimeContent = (label: string, checked: boolean) => {
const order = ["天", "时", "分", "秒"];
const next = checked? [...timeContent, label]: timeContent.filter((c) => c!== label);
next.sort((a, b) => order.indexOf(a) - order.indexOf(b));
cd.onChange({ timeContent: next});
};
// 原版 watch(moduleColorStyle, immediate)：切换风格联动三处，此处在用户切换时执行（不做挂载即覆盖，避免打开面板即改老数据）
const applyColorStyle = (style: string) => {
const theme = MODULE_THEME[style];
onChange({ moduleColorStyle: style});
if (!theme) return;
onChange({
countdownData: {...(data?.countdownData?? {}), countdownBackgroundPic: theme.countdownBackgroundPic},
moduleColor: {...(data?.moduleColor?? {}), productPriceColor: theme.textColor},
contentStyle: {...(data?.contentStyle?? {}), gradientColorA: theme.gradientColorA, gradientColorB: theme.gradientColorB}
});
};
const countdownPicObj =
cd.data.countdownBackgroundPic && typeof cd.data.countdownBackgroundPic === "object"
? cd.data.countdownBackgroundPic
: typeof cd.data.countdownBackgroundPic === "string" && cd.data.countdownBackgroundPic
? { picUrl: cd.data.countdownBackgroundPic, picThumb: cd.data.countdownBackgroundPic}
: { picUrl: "", picThumb: ""};

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["商品设置", "扩展设置"]}>
                <>
                    <SettingGroup title="请点击添加选择商品">
                        <div style={{ color: "#646a73", fontSize: 12, lineHeight: 1.6 }}>
                            商品选择器（ProductInfoSelect 数据选择组件）尚未接入本编辑器，已选商品数据保持不变。
                            <br />
                            提示：您可以通过拖拽进行商品排序
                        </div>
                    </SettingGroup>
                    <SettingRow label="模块颜色风格" block>
                        <SegmentedRadio
                            value={data?.moduleColorStyle ?? "orange"}
                            options={[
                                { value: "orange", label: "番茄橙" },
                                { value: "red", label: "中国红" },
                                { value: "blue", label: "天空蓝" },
                                { value: "green", label: "森林绿" },
                                { value: "purple", label: "紫罗兰" },
                                { value: "grey", label: "深灰色" },
                                { value: "chocolate", label: "巧克力" },
                                { value: "brown", label: "马鞍棕" }
                            ]}
                            onChange={(v) => applyColorStyle(String(v))}
                        />
                    </SettingRow>
                    <SettingGroup title="模块标题设置">
                        <TextField {...mt} field="title" label="模块大标题" placeholder="请输入标题内容" />
                        <TextField {...mt} field="subTitle" label="模块小标题" placeholder="请输入标题内容" />
                        <TextField {...mt} field="moreText" label="右侧提示信息" placeholder="请输入右侧提示信息" />
                        <SettingRow label="跳转链接">
                            <PicLinkField value={mt.data.titleLink ?? null} onChange={(v) => mt.onChange({ titleLink: v })} />
                        </SettingRow>
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <RadioRowField
                            {...cd}
                            field="showCountdown"
                            label="倒计时"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "不显示" }
                            ]}
                        />
                        {showCountdown ? (
                            <>
                                <SettingRow label="倒计时图片: 宽750, 高度不限" block>
                                    <div style={{ color: "#999", fontSize: 12 }}>通过下方图盒更换倒计时背景图</div>
                                </SettingRow>
                                <PicSelectField
                                    data={{ countdownBackgroundPic: countdownPicObj }}
                                    onChange={(patch) => cd.onChange(patch)}
                                    field="countdownBackgroundPic"
                                    label="倒计时图片"
                                />
                                <RadioRowField
                                    {...cd}
                                    field="countdownType"
                                    label="倒计时类型"
                                    options={[
                                        { value: "dailyCycle", label: "每日循环" },
                                        { value: "specifyTime", label: "指定时间" }
                                    ]}
                                />
                                {cd.data.countdownType === "dailyCycle" ? (
                                    <SettingRow label="每日循环时间">
                                        <TimePicker
                                            format="HH:mm"
                                            value={cd.data.dailyTime ? new Date(cd.data.dailyTime) : undefined}
                                            style={{ width: 240, maxWidth: "100%" }}
                                            onChange={(_time, timeString) => {
                                                const [h, m] = String(timeString || "").split(":").map(Number);
                                                if (Number.isFinite(h)) {
                                                    const d = new Date();
                                                    d.setHours(h, m || 0, 0, 0);
                                                    cd.onChange({ dailyTime: d.getTime() });
                                                }
                                            }}
                                        />
                                    </SettingRow>
                                ) : null}
                                {cd.data.countdownType === "specifyTime" ? (
                                    <SettingRow label="指定时间">
                                        <DatePicker
                                            type="dateTime"
                                            format="yyyy-MM-dd HH:mm"
                                            value={cd.data.specifyTime ? new Date(cd.data.specifyTime) : undefined}
                                            style={{ width: 240, maxWidth: "100%" }}
                                            onChange={(date) => {
                                                const ts = date ? new Date(date as Date).getTime() : NaN;
                                                if (Number.isFinite(ts)) cd.onChange({ specifyTime: ts });
                                            }}
                                        />
                                    </SettingRow>
                                ) : null}
                                <SettingRow label="选择时间显示内容" block>
                                    <div style={{ display: "flex", gap: 12 }}>
                                        {["天", "时", "分", "秒"].map((label) => (
                                            <Checkbox key={label} checked={timeContent.includes(label)} onChange={(e: any) => toggleTimeContent(label, !!e.target.checked)}>
                                                {label}
                                            </Checkbox>
                                        ))}
                                    </div>
                                </SettingRow>
                                <RangeField {...cd} field="marginTop" label="顶部距离" max={100} />
                                <RangeField {...cd} field="countdownPicBottom" label="倒计时图片上浮距离" max={40} />
                            </>
                        ) : null}
                        <DecDivider />
                        <SettingRow label="每屏显示数量" block>
                            <SegmentedRadio
                                value={Number(data?.rowNum ?? 3)}
                                options={[
                                    { value: 2, label: "2个" },
                                    { value: 2.5, label: "2个半" },
                                    { value: 3, label: "3个" },
                                    { value: 3.5, label: "3个半" }
                                ]}
                                onChange={(v) => onChange({ rowNum: v })}
                            />
                        </SettingRow>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="picScale"
                            label="图片比例"
                            options={[
                                { value: "3:2", label: "3:2" },
                                { value: "1:1", label: "1:1" },
                                { value: "3:4", label: "3:4" },
                                { value: "16:9", label: "16:9" }
                            ]}
                        />
                        <PicFillTypeField data={data} onChange={onChange} field="picFillType" />
                        <TemplateIconRadioRowField
                            data={data}
                            onChange={onChange}
                            field="textAlignment"
                            label="标题对齐方式"
                            options={[
                                { value: "left", label: "左对齐", icon: "align-left" },
                                { value: "center", label: "居中对齐", icon: "align-center" }
                            ]}
                        />
                        <RangeField data={data} onChange={onChange} field="productPadding" label="商品内部间距" max={10} />
                        <RangeField data={data} onChange={onChange} field="productRadius" label="商品圆角半径" max={20} />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="picBottomRadius"
                            label="图片底部圆角"
                            options={[
                                { value: 1, label: "直角" },
                                { value: 2, label: "圆角" }
                            ]}
                        />
                        <DecDivider />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="showProductName"
                            label="商品标题"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "不显示" }
                            ]}
                        />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="showProductPrice"
                            label="商品价格"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "不显示" }
                            ]}
                        />
                        {Number(data?.showProductPrice) === 1 ? (
                            <TemplateIconRadioRowField
                                data={data}
                                onChange={onChange}
                                field="priceWeight"
                                label="商品价格粗细"
                                options={[
                                    { value: 400, label: "常规体", aSize: 14 },
                                    { value: 700, label: "加粗体", aSize: 14, aBold: true }
                                ]}
                            />
                        ) : null}
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="crossedOutPrice"
                            label="商品划线价"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "不显示" }
                            ]}
                        />
                    </SettingGroup>
                </>
                <>
                    <SettingGroup title="颜色设置">
                        <ColorField {...mc} field="titleColor" label="模块大标题颜色" resetValue="" />
                        <ColorField {...mc} field="subTitleColor" label="模块小标题颜色" resetValue="" />
                        <ColorField {...mc} field="tipsColor" label="提示信息颜色" resetValue="" />
                        <DecDivider />
                        <ColorField {...mc} field="productBackgroundColor" label="商品背景颜色" resetValue="" />
                        {Number(data?.showProductName) === 1 ? <ColorField {...mc} field="productNameColor" label="商品标题颜色" resetValue="" /> : null}
                        <ColorField {...mc} field="productPriceColor" label="商品价格颜色" resetValue="" />
                        <ColorField {...mc} field="crossedOutPriceColor" label="商品划线价颜色" resetValue="" />
                        <DecDivider />
                        <ColorField {...mc} field="countdownColor" label="倒计时文本颜色" resetValue="" />
                    </SettingGroup>
                    <ModuleStyleSettingsField data={data} onChange={onChange} type="categoryRec" />
                    <ContentStyleSettingsField data={data} onChange={onChange} type="countdown" />
                </>
            </SettingsTabs>
        </div>
    );
}
