/**
 * 点单卡片 设置（对齐 view/admin mobilePro/storeOrderCard/Settings.vue）
 *
 * 页签：内容设置 / 扩展设置。
 * - 选择模板（picType 图标 radio）以等值文字钮承接；切换模板按 LENGTH_MAP 增删 cardList
 *   （补空卡/截断）并重置选中卡，与原版 onPicTypeChange 一致；
 * - 卡片选择：原版为布局缩略图点选，本实现以等价的卡片选择条承接（点选切换当前卡，
 *   缩略图布局预览未 1:1 复刻，小结注明）；当前卡编辑：链接（SelectLink 文本输入写回
 *   link.link.link 结构）、标题（maxlength 10)/描述（maxlength 20，图文模式）、
 *   图片（PicSelectBox 写回 picUrl/picThumb；原版 DialogForm 图库回填的 picId/picName
 *   在本承接下保持原值）；
 * - 未配置卡片提示（hasEmptyPicCard）与原版同口径；
 * - 扩展页 CommonFrameEdit 以 FrameSettingsField hideBg + splitInnerPadding 承接
 *   （原版 allow-negative-top 上边距负值未承接：共享 RangeField 下限为 0，小结注明），
 *   图片边距三项挂 frame 本地 RangeField（max 30）。
 */
import { useState } from "react";
import { Input } from "@douyinfe/semi-ui";
import {
    SettingGroup,
    SettingRow,
    RadioRowField,
    RadioField,
    RangeField,
    ColorField,
    PicSelectBox,
    FrameSettingsField,
    DecDivider
} from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import { pickPic } from "../../../shared/format";
import { LENGTH_MAP, buildDefaultCard } from "./index";
import { useI18n } from "../../../../../i18n";

type Patch = Record<string, any>;
interface Props {
    data: Record<string, any>;
    onChange: (patch: Patch) => void;
}

const PIC_TYPE_OPTIONS = [
    { value: 1, label: "一行一个" },
    { value: 2, label: "一行两个" },
    { value: 3, label: "左一右二" },
    { value: 4, label: "一行三个" },
    { value: 5, label: "一行四个" },
    { value: 6, label: "两行五个" }
];

export default function StoreOrderCardSettings({ data, onChange }: Props) {
    const { t } = useI18n();
    const [picIndex, setPicIndex] = useState(0);
    const picType = Number(data?.picType) || 2;
    const cardList: Array<Record<string, any>> = Array.isArray(data?.cardList) ? data.cardList : [];
    const current = cardList[picIndex];
    const frame = data?.frame && typeof data.frame === "object" ? data.frame : {};
    const frameProps: Props = { data: frame, onChange: (patch: Patch) => onChange({ frame: { ...frame, ...patch } }) };

    const changePicType = (next: number) => {
        const target = LENGTH_MAP[next] || 2;
        const list = [...cardList];
        while (list.length < target) list.push(buildDefaultCard());
        if (list.length > target) list.length = target;
        setPicIndex(0);
        onChange({ picType: next, cardList: list });
    };
    const patchCard = (patch: Patch) => {
        onChange({ cardList: cardList.map((c, i) => (i === picIndex ? { ...c, ...patch } : c)) });
    };
    const hasEmptyPicCard =
        Number(data?.cardStyle) === 2
            ? cardList.some((c) => !c?.picUrl && !c?.picThumb)
            : cardList.some((c) => !c?.picUrl && !c?.picThumb && !c?.title && !c?.desc);
    const linkUrl = current?.link?.link?.link ?? (typeof current?.link?.link === "string" ? current.link.link : "");

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <>
                    <SettingGroup title="模块样式">
                        <SettingRow label="选择模板" block>
                            <RadioField
                                data={data}
                                onChange={(patch: Patch) => {
                                    if (patch.picType !== undefined) changePicType(Number(patch.picType));
                                }}
                                field="picType"
                                options={PIC_TYPE_OPTIONS}
                            />
                        </SettingRow>
                    </SettingGroup>
                    <DecDivider />
                    <RadioRowField
                        data={data}
                        onChange={onChange}
                        field="cardStyle"
                        label="卡片风格"
                        options={[
                            { value: 1, label: "图文模式" },
                            { value: 2, label: "图片模式" }
                        ]}
                    />
                    <SettingRow label="选择卡片" block>
                        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                            {cardList.map((card, index) => (
                                <button
                                    key={index}
                                    type="button"
                                    onClick={() => setPicIndex(index)}
                                    style={{
                                        border: index === picIndex ? "1px solid #155bd4" : "1px solid #e4e7ed",
                                        backgroundColor: index === picIndex ? "#eef4ff" : "#fff",
                                        borderRadius: 4,
                                        padding: "4px 8px",
                                        cursor: "pointer",
                                        display: "flex",
                                        alignItems: "center",
                                        gap: 6,
                                        fontSize: 12
                                    }}
                                >
                                    {pickPic(card) ? <img src={pickPic(card)} alt="" style={{ width: 20, height: 20, objectFit: "cover", borderRadius: 2 }} /> : null}
                                    卡片{index + 1}
                                    {card.title ? ` · ${card.title}` : ""}
                                </button>
                            ))}
                        </div>
                    </SettingRow>
                    {current ? (
                        <SettingGroup title={`卡片${picIndex + 1}设置`}>
                            <SettingRow label="链接">
                                <Input
                                    value={linkUrl}
                                    placeholder={t("跳转链接，如 /pages/product/index?id=1")}
                                    style={{ width: 240, maxWidth: "100%" }}
                                    onChange={(v: string) =>
                                        patchCard({
                                            link: { ...(current.link || {}), link: { ...(current.link?.link && typeof current.link.link === "object" ? current.link.link : {}), link: v } }
                                        })
                                    }
                                />
                            </SettingRow>
                            {Number(data?.cardStyle) === 1 ? (
                                <>
                                    <SettingRow label="标题">
                                        <Input
                                            value={current.title ?? ""}
                                            maxLength={10}
                                            placeholder={t("请输入标题")}
                                            style={{ width: 240, maxWidth: "100%" }}
                                            onChange={(v: string) => patchCard({ title: v })}
                                        />
                                    </SettingRow>
                                    <SettingRow label="描述">
                                        <Input
                                            value={current.desc ?? ""}
                                            maxLength={20}
                                            placeholder={t("请输入描述")}
                                            style={{ width: 240, maxWidth: "100%" }}
                                            onChange={(v: string) => patchCard({ desc: v })}
                                        />
                                    </SettingRow>
                                </>
                            ) : null}
                            <SettingRow label="图片">
                                <span>
                                    <PicSelectBox pic={current} onSelect={(pic) => patchCard({ picUrl: pic.picUrl, picThumb: pic.picThumb })} />
                                    <span style={{ display: "block", color: "#999", fontSize: 12, marginTop: 4 }}>{t("建议尺寸 160*160 像素")}</span>
                                </span>
                            </SettingRow>
                        </SettingGroup>
                    ) : null}
                    {hasEmptyPicCard ? (
                        <div style={{ margin: "0 16px 10px", padding: "8px 10px", backgroundColor: "#fdf6ec", color: "#e6a23c", fontSize: 12, borderRadius: 4 }}>存在未配置的点单卡片</div>
                    ) : null}
                    <DecDivider />
                    <SettingRow label="卡片样式" block>
                        <RadioField
                            data={data}
                            onChange={onChange}
                            field="boxStyle"
                            options={[
                                { value: 1, label: "无边白底" },
                                { value: 2, label: "卡片投影" },
                                { value: 3, label: "描边白底" },
                                { value: 4, label: "无边透明底" }
                            ]}
                        />
                    </SettingRow>
                    <ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" resetValue="" />
                    {![1, 4].includes(Number(data?.boxStyle)) ? (
                        <ColorField data={data} onChange={onChange} field="cardBackground" label="卡片背景" resetValue="#ffffff" />
                    ) : null}
                    {picType !== 1 ? <RangeField data={data} onChange={onChange} field="cardGap" label="卡片间距" max={30} /> : null}
                </>
                <>
                    <FrameSettingsField data={data} onChange={onChange} hideBg splitInnerPadding />
                    <RangeField {...frameProps} field="imagePaddingTop" label="图片上边距" max={30} />
                    <RangeField {...frameProps} field="imagePaddingBottom" label="图片下边距" max={30} />
                    <RangeField {...frameProps} field="imagePaddingLeftRight" label="图片左右边距" max={30} />
                </>
            </SettingsTabs>
        </div>
    );
}
