/**
 * 用户信息 设置（对齐 view/admin userInfo/Settings.vue）
 *
 * 页签 内容设置/扩展设置。内容：签到入口（开关/自定义图标/入口名称）、设置入口（开关/自定义图标）；
 * 扩展：背景渐变色（双色 + 重置）/ 内边距（滑杆 max 30）/ CommonFrameEdit（hide-bg、hide-padding）。
 * signIn/settings 的 show 为 boolean 契约，用 Semi Switch 写 boolean。
 */
import { Switch } from "@douyinfe/semi-ui";
import AlphaColorInput from "../../../../../components/AlphaColorInput";
import {
    SettingRow,
    SettingGroup,
    TextField,
    PicSelectBox,
    RangeField,
    FrameSettingsField
} from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import { defaultGradient } from "./index";
import { useI18n } from "../../../../../i18n";

type Patch = Record<string, any>;
interface Props {
    data: Record<string, any>;
    onChange: (patch: Patch) => void;
}

/** 嵌套对象适配：把顶层 data/onChange 转成某子对象的 data/onChange */
function nested(data: Record<string, any>, onChange: (patch: Patch) => void, key: string) {
    const sub = (data?.[key] ?? {}) as Record<string, any>;
    return { data: sub, onChange: (patch: Patch) => onChange({ [key]: { ...sub, ...patch } }) };
}

function BoolRow({ data, onChange, field, label }: Props & { field: string; label: string }) {
    return (
        <SettingRow label={label}>
            <Switch checked={data?.[field] !== false} onChange={(v: boolean) => onChange({ [field]: v })} />
        </SettingRow>
    );
}

function IconRow({ data, onChange, field, label, defaultPic }: Props & { field: string; label: string; defaultPic: string }) {
    const pic = data?.[field] && typeof data[field] === "object" ? data[field] : { picUrl: defaultPic, picThumb: defaultPic };
    return (
        <SettingRow label={label}>
            <PicSelectBox
                pic={pic}
                onSelect={(p) => onChange({ [field]: p })}
                onReset={() => onChange({ [field]: { picUrl: defaultPic, picThumb: defaultPic } })}
            />
        </SettingRow>
    );
}

export default function UserInfoSettings({ data, onChange }: Props) {
    const { t } = useI18n();
    const signIn = nested(data, onChange, "signIn");
    const settings = nested(data, onChange, "settings");
    const gradient = nested(data, onChange, "gradient");
    const frame = nested(data, onChange, "frame");
    const defaultSignIn = { picUrl: "https://oss.tigshop.com/img/gallery/202605/1779691601SIdmgIcby2Mii610eQ.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200" };
    const defaultSettings = { picUrl: "https://oss.tigshop.com/img/gallery/202605/1779679345kBAM5aU9K9EuPRQFfo.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200" };

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <div>
                    <SettingGroup>
                        <BoolRow {...signIn} field="show" label="签到入口" />
                        <IconRow {...signIn} field="icon" label="自定义图标" defaultPic={defaultSignIn.picUrl} />
                        <TextField {...signIn} field="title" label="入口名称" placeholder="请输入入口名称" />
                    </SettingGroup>
                    <SettingGroup>
                        <BoolRow {...settings} field="show" label="设置入口" />
                        <IconRow {...settings} field="icon" label="自定义图标" defaultPic={defaultSettings.picUrl} />
                    </SettingGroup>
                </div>
                <div>
                    <SettingRow label="背景渐变色">
                        <div className="dec-color-group">
                            <div className="dec-color-button">
                                <a
                                    className="dec-color-reset"
                                    onClick={() => onChange({ gradient: { ...defaultGradient } })}
                                >
                                    {t("重置")}
                                </a>
                                <span className="dec-color-trigger">
                                    <AlphaColorInput
                                        value={String(gradient.data.colorA ?? defaultGradient.colorA)}
                                        onChange={(v) => gradient.onChange({ colorA: v })}
                                    />
                                </span>
                                <span style={{ margin: "0 10px" }}>-</span>
                                <span className="dec-color-trigger">
                                    <AlphaColorInput
                                        value={String(gradient.data.colorB ?? defaultGradient.colorB)}
                                        onChange={(v) => gradient.onChange({ colorB: v })}
                                    />
                                </span>
                            </div>
                        </div>
                    </SettingRow>
                    <RangeField {...frame} field="innerPadding" label="内边距" max={30} />
                    <FrameSettingsField data={data} onChange={onChange} hideBg hidePadding />
                </div>
            </SettingsTabs>
        </div>
    );
}
