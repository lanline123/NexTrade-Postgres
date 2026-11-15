/**
 * 用户信息3 设置（对齐 view/admin userInfo3/Settings.vue）
 *
 * 页签 内容设置/扩展设置。内容：布局风格（1-4，切换联动同原版 watch：textColors 重置为新样式默认色、
 * 样式 2/3/4 渐变重置为该样式默认、样式四另重置 memberBar 粉渐变+圆角 10）→ 文字颜色 4 项
 * （统计两色仅样式一/二，重置回当前样式默认）→ 各样式专属组：
 *   样式一 会员等级条（开关/背景/上圆角/等级名/描述/查看链接 SelectLink 文本承接）；
 *   样式二 设置入口+图标、消息通知+图标；
 *   样式三 成长值条（开关/渐变色对/上圆角）；
 *   样式四 头像编辑图标开关、成长值卡（开关/渐变色对/圆角/按钮文案/背景图）；
 *   样式一/二 统计数据（总开关 + 逐项 checkbox 显示 + 名称输入）。
 * 扩展：样式一 背景色（重置 #57D45C）/背景图/弧底开关；样式 2/3/4 渐变色对 + 渐变方向；
 *   左右内边距滑杆（frame.innerPadding，max 30）；CommonFrameEdit 门控
 *   hideBg+hidePadding 恒开，hideBoxPadding/hideBoxRadius 仅样式 2/3/4。
 * 布尔字段（show/showEditIcon）写 boolean，同原版 el-switch/el-checkbox 语义。
 */
import { Input, InputNumber, Radio, RadioGroup, Slider, Switch, Checkbox } from "@douyinfe/semi-ui";
import AlphaColorInput from "../../../../../components/AlphaColorInput";
import { SettingGroup, SettingRow, SegmentedRadio, DecDivider, PicSelectBox, FrameSettingsField, RadioRowField } from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";

const TEXT_DEFAULTS: Record<number, Record<string, string>> = {
    1: { nameColor: "#ffffff", subColor: "#ffffff", statValueColor: "#ffffff", statLabelColor: "#ffffff" },
    2: { nameColor: "#333333", subColor: "#666666", statValueColor: "#333333", statLabelColor: "#666666" },
    3: { nameColor: "#333333", subColor: "#666666", statValueColor: "#333333", statLabelColor: "#666666" },
    4: { nameColor: "#333333", subColor: "#666666", statValueColor: "#333333", statLabelColor: "#666666" }
};
const GRADIENT_DEFAULTS: Record<number, { colorA: string; colorB: string; gradientType: string }> = {
    2: { colorA: "#E0FCED", colorB: "#FDF1F3", gradientType: "leftRight" },
    3: { colorA: "#F1EFE9", colorB: "#E7F0E6", gradientType: "leftRight" },
    4: { colorA: "#FBD9E1", colorB: "#FDEDEF", gradientType: "leftRight" }
};

function BoolRow({ label, value, onValue }: { label: string; value: boolean; onValue: (v: boolean) => void }) {
    return (
        <SettingRow label={label}>
            <Switch checked={value} onChange={(v: boolean) => onValue(!!v)} />
        </SettingRow>
    );
}
function ColorRow({ label, value, onValue, resetValue }: { label: string; value: string; onValue: (v: string) => void; resetValue?: string }) {
    return (
        <SettingRow label={label}>
            <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <AlphaColorInput value={value ?? ""} onChange={onValue} />
                {resetValue !== undefined ? (
                    <a
                        className="dec-color-reset"
                        onClick={(e) => {
                            e.preventDefault();
                            onValue(resetValue);
                        }}
                    >
                        重置
                    </a>
                ) : null}
            </div>
        </SettingRow>
    );
}
function RangeRow({ label, value, max, onValue }: { label: string; value: number; max: number; onValue: (v: number) => void }) {
    const v = Number(value ?? 0) || 0;
    return (
        <SettingRow label={label}>
            <div style={{ display: "flex", alignItems: "center", gap: 10, width: 240, maxWidth: "100%" }}>
                <Slider style={{ flex: 1 }} min={0} max={max} value={v} onChange={(n) => onValue(Number(n) || 0)} />
                <InputNumber style={{ width: 64 }} min={0} max={max} value={v} onChange={(n) => onValue(Number(n) || 0)} />
            </div>
        </SettingRow>
    );
}
function SegRow({
    label,
    value,
    options,
    onValue
}: {
    label: string;
    value: any;
    options: { value: any; label: string }[];
    onValue: (v: any) => void;
}) {
    return (
        <SettingRow label={label}>
            <RadioGroup type="button" value={value} onChange={(e: any) => onValue(e?.target?.value)}>
                {options.map((o) => (
                    <Radio key={String(o.value)} value={o.value}>
                        {o.label}
                    </Radio>
                ))}
            </RadioGroup>
        </SettingRow>
    );
}
function PicRow({ label, value, onValue }: { label: string; value: any; onValue: (v: any) => void }) {
    return (
        <SettingRow label={label}>
            <PicSelectBox pic={value} onSelect={onValue} onReset={() => onValue({})} />
        </SettingRow>
    );
}
function TextRow({
    label,
    value,
    onValue,
    placeholder,
    width = 240
}: {
    label: string;
    value: string;
    onValue: (v: string) => void;
    placeholder?: string;
    width?: number;
}) {
    return (
        <SettingRow label={label}>
            <Input value={value ?? ""} placeholder={placeholder} style={{ width, maxWidth: "100%" }} onChange={onValue} />
        </SettingRow>
    );
}

export default function UserInfo3Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const layoutStyle = data?.layoutStyle ?? 1;
    const nested = (key: string) => (patch: Record<string, any>) => onChange({ [key]: { ...(data?.[key] ?? {}), ...patch } });
    const setTextColor = (key: string, v: string) => nested("textColors")({ [key]: v });
    const textColor = (key: string) => data?.textColors?.[key] || TEXT_DEFAULTS[layoutStyle]?.[key] || "";
    const bar = data?.memberBar ?? {};
    const setBar = nested("memberBar");
    const stats = data?.stats ?? {};
    const statsItems: any[] = Array.isArray(stats.items) ? stats.items : [];
    const setStatsItem = (index: number, patch: Record<string, any>) =>
        onChange({ stats: { ...stats, items: statsItems.map((it, i) => (i === index ? { ...it, ...patch } : it)) } });

    const handleLayout = (patch: Record<string, any>) => {
        if (patch.layoutStyle && patch.layoutStyle !== layoutStyle) {
            const next = patch.layoutStyle;
            const extra: Record<string, any> = { textColors: { ...TEXT_DEFAULTS[next] } };
            if (next === 2 || next === 3 || next === 4) extra.gradient = { ...GRADIENT_DEFAULTS[next] };
            if (next === 4) extra.memberBar = { ...bar, backgroundColor: "#FF9EB5", backgroundColorB: "#FF6E97", borderRadius: 10 };
            onChange({ ...patch, ...extra });
            return;
        }
        onChange(patch);
    };

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <div>
                    <SettingGroup>
                        <RadioRowField
                            data={data}
                            onChange={handleLayout}
                            field="layoutStyle"
                            label="布局风格"
                            options={[
                                { value: 1, label: "样式一" },
                                { value: 2, label: "样式二" },
                                { value: 3, label: "样式三" },
                                { value: 4, label: "样式四" }
                            ]}
                        />
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup>
                        <ColorRow label="昵称颜色" value={textColor("nameColor")} onValue={(v) => setTextColor("nameColor", v)} resetValue={TEXT_DEFAULTS[layoutStyle]?.nameColor} />
                        <ColorRow label="副标题颜色" value={textColor("subColor")} onValue={(v) => setTextColor("subColor", v)} resetValue={TEXT_DEFAULTS[layoutStyle]?.subColor} />
                        {layoutStyle === 1 || layoutStyle === 2 ? (
                            <>
                                <ColorRow
                                    label="统计数值颜色"
                                    value={textColor("statValueColor")}
                                    onValue={(v) => setTextColor("statValueColor", v)}
                                    resetValue={TEXT_DEFAULTS[layoutStyle]?.statValueColor}
                                />
                                <ColorRow
                                    label="统计标签颜色"
                                    value={textColor("statLabelColor")}
                                    onValue={(v) => setTextColor("statLabelColor", v)}
                                    resetValue={TEXT_DEFAULTS[layoutStyle]?.statLabelColor}
                                />
                            </>
                        ) : null}
                    </SettingGroup>
                    <DecDivider />
                    {layoutStyle === 1 ? (
                        <SettingGroup>
                            <BoolRow label="会员等级条" value={bar.show !== false} onValue={(v) => setBar({ show: v })} />
                            {bar.show !== false ? (
                                <>
                                    <ColorRow label="等级条背景" value={bar.backgroundColor ?? ""} onValue={(v) => setBar({ backgroundColor: v })} resetValue="#333" />
                                    <RangeRow label="等级条上圆角" value={bar.borderRadius ?? 10} max={30} onValue={(v) => setBar({ borderRadius: v })} />
                                    <TextRow label="会员文案" value={bar.levelName ?? ""} onValue={(v) => setBar({ levelName: v })} placeholder="付费会员" />
                                    <TextRow label="描述文案" value={bar.description ?? ""} onValue={(v) => setBar({ description: v })} placeholder="购买会员，畅享更多专属权益" />
                                    <TextRow
                                        label="查看链接"
                                        value={bar.buttonLink?.link ?? ""}
                                        onValue={(v) => setBar({ buttonLink: { ...(bar.buttonLink ?? {}), link: v } })}
                                        placeholder="member/premiumMember/index"
                                    />
                                </>
                            ) : null}
                        </SettingGroup>
                    ) : null}
                    {layoutStyle === 2 ? (
                        <SettingGroup>
                            <BoolRow label="设置入口" value={data?.settings?.show !== false} onValue={(v) => nested("settings")({ show: v })} />
                            {data?.settings?.show !== false ? (
                                <PicRow label="设置图标" value={data?.settings?.icon} onValue={(v) => nested("settings")({ icon: v })} />
                            ) : null}
                            <BoolRow label="消息通知" value={data?.notification?.show !== false} onValue={(v) => nested("notification")({ show: v })} />
                            {data?.notification?.show !== false ? (
                                <PicRow label="通知图标" value={data?.notification?.icon} onValue={(v) => nested("notification")({ icon: v })} />
                            ) : null}
                        </SettingGroup>
                    ) : null}
                    {layoutStyle === 3 ? (
                        <SettingGroup>
                            <BoolRow label="成长值条" value={bar.show !== false} onValue={(v) => setBar({ show: v })} />
                            {bar.show !== false ? (
                                <>
                                    <SettingRow label="成长值条渐变色">
                                        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                                            <AlphaColorInput value={bar.backgroundColor || "#292929"} onChange={(v: string) => setBar({ backgroundColor: v })} />
                                            <span style={{ color: "#999", fontSize: 12 }}>→</span>
                                            <AlphaColorInput value={bar.backgroundColorB || "#525252"} onChange={(v: string) => setBar({ backgroundColorB: v })} />
                                            <a
                                                className="dec-color-reset"
                                                onClick={(e) => {
                                                    e.preventDefault();
                                                    setBar({ backgroundColor: "#292929", backgroundColorB: "#525252" });
                                                }}
                                            >
                                                重置
                                            </a>
                                        </div>
                                    </SettingRow>
                                    <RangeRow label="成长值条上圆角" value={bar.borderRadius ?? 10} max={30} onValue={(v) => setBar({ borderRadius: v })} />
                                </>
                            ) : null}
                        </SettingGroup>
                    ) : null}
                    {layoutStyle === 4 ? (
                        <SettingGroup>
                            <BoolRow label="头像编辑图标" value={data?.showEditIcon !== false} onValue={(v) => onChange({ showEditIcon: v })} />
                            <BoolRow label="成长值卡" value={bar.show !== false} onValue={(v) => setBar({ show: v })} />
                            {bar.show !== false ? (
                                <>
                                    <SettingRow label="成长值卡渐变色">
                                        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                                            <AlphaColorInput value={bar.backgroundColor || "#FF9EB5"} onChange={(v: string) => setBar({ backgroundColor: v })} />
                                            <span style={{ color: "#999", fontSize: 12 }}>→</span>
                                            <AlphaColorInput value={bar.backgroundColorB || "#FF6E97"} onChange={(v: string) => setBar({ backgroundColorB: v })} />
                                        </div>
                                    </SettingRow>
                                    <RangeRow label="成长值卡圆角" value={bar.borderRadius ?? 10} max={30} onValue={(v) => setBar({ borderRadius: v })} />
                                    <TextRow label="按钮文案" value={bar.buttonText ?? ""} onValue={(v) => setBar({ buttonText: v })} placeholder="查看" width={160} />
                                    <PicRow label="成长值卡背景图" value={bar.waveImage} onValue={(v) => setBar({ waveImage: v })} />
                                </>
                            ) : null}
                        </SettingGroup>
                    ) : null}
                    {layoutStyle === 1 || layoutStyle === 2 ? (
                        <>
                            <DecDivider />
                            <SettingGroup>
                                <BoolRow label="统计数据" value={stats.show !== false} onValue={(v) => onChange({ stats: { ...stats, show: v } })} />
                                {stats.show !== false
                                    ? statsItems.map((item, index) => (
                                          <SettingRow key={item.key ?? index} label={item.label}>
                                              <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                                                  <Checkbox checked={item.show !== false} onChange={(e: any) => setStatsItem(index, { show: !!e?.target?.checked })}>
                                                      显示
                                                  </Checkbox>
                                                  <Input
                                                      value={item.label ?? ""}
                                                      placeholder="统计项名称"
                                                      style={{ width: 160 }}
                                                      onChange={(v: string) => setStatsItem(index, { label: v })}
                                                  />
                                              </div>
                                          </SettingRow>
                                      ))
                                    : null}
                            </SettingGroup>
                        </>
                    ) : null}
                </div>
                <div>
                    {layoutStyle === 1 ? (
                        <>
                            <SettingGroup>
                                <ColorRow label="背景色" value={data?.bgColor ?? ""} onValue={(v) => onChange({ bgColor: v })} resetValue="#57D45C" />
                                <PicRow label="背景图" value={data?.backgroundImage} onValue={(v) => onChange({ backgroundImage: v })} />
                            </SettingGroup>
                            <DecDivider />
                            <SettingGroup>
                                <BoolRow label="弧底效果" value={data?.bottomArc?.show !== false} onValue={(v) => nested("bottomArc")({ show: v })} />
                            </SettingGroup>
                            <DecDivider />
                        </>
                    ) : (
                        <>
                            <SettingGroup>
                                <SettingRow label={layoutStyle === 4 ? "顶部背景渐变色" : "背景渐变色"}>
                                    <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                                        <AlphaColorInput
                                            value={data?.gradient?.colorA || GRADIENT_DEFAULTS[layoutStyle]?.colorA || ""}
                                            onChange={(v: string) => nested("gradient")({ colorA: v })}
                                        />
                                        <span style={{ color: "#999", fontSize: 12 }}>-</span>
                                        <AlphaColorInput
                                            value={data?.gradient?.colorB || GRADIENT_DEFAULTS[layoutStyle]?.colorB || ""}
                                            onChange={(v: string) => nested("gradient")({ colorB: v })}
                                        />
                                    </div>
                                </SettingRow>
                                <SegRow
                                    label={layoutStyle === 4 ? "顶部背景渐变方向" : "背景渐变方向"}
                                    value={data?.gradient?.gradientType || "leftRight"}
                                    options={[
                                        { value: "upDown", label: "上下" },
                                        { value: "leftRight", label: "左右" },
                                        { value: "diagonal", label: "斜向" }
                                    ]}
                                    onValue={(v) => nested("gradient")({ gradientType: v })}
                                />
                            </SettingGroup>
                            <DecDivider />
                        </>
                    )}
                    <SettingGroup>
                        <RangeRow
                            label="左右内边距"
                            value={data?.frame?.innerPadding ?? 16}
                            max={30}
                            onValue={(v) => onChange({ frame: { ...(data?.frame ?? {}), innerPadding: v } })}
                        />
                    </SettingGroup>
                    <FrameSettingsField
                        data={data}
                        onChange={onChange}
                        hideBg
                        hidePadding
                        hideBoxPadding={layoutStyle === 2 || layoutStyle === 3 || layoutStyle === 4}
                        hideBoxRadius={layoutStyle === 2 || layoutStyle === 3 || layoutStyle === 4}
                    />
                </div>
            </SettingsTabs>
        </div>
    );
}
