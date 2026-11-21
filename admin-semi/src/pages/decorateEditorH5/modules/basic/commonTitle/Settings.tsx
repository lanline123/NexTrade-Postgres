/**
 * 贴附标题条 设置（复用共享 TitleSettingsField 通用形态，对齐原版 commonTitle/Edit.vue else 分支：
 * 风格三/背景渐变色仅 product 形态、圆角 max 30、descColor/moreColor 重置回 defaultTitle 值、
 * moreLink 走 SelectLink 对象口径、各字段恒显示与 showTitle/showMore 无联动隐藏）
 *
 * 本模块数据本身即标题对象（位于模块数据根部），TitleSettingsField 按 field="title"
 * 读写嵌套对象，故此处以 { title: data } 适配、onChange 解包后回写根部。
 * moreLink 经 TitleSettingsField 内的 PicLinkField 选择（保留原对象其余键的合并口径）。
 *
 * 2026-11-21 用户拍板（通用标题三项口径）：独立形态额外放开标题位置/标题粗细
 * （enableAlignWeight，仅本模块传；嵌套形态门控不动），顶部加一行与「文本标题」
 * 的区别说明（与模块库悬停共用 commonTitleDesc）。
 */
import { TitleSettingsField } from "../../../shared/settingWidgets";
import { useI18n } from "../../../../../i18n";
import { commonTitleDesc } from "./index";

export default function CommonTitleSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const { t } = useI18n();
    return (
        <div className="h5-editor-settings-body">
            <div style={{ fontSize: 12, color: "#999", padding: "0 16px", marginBottom: 4 }}>{t(commonTitleDesc)}</div>
            <TitleSettingsField
                data={{ title: data }}
                enableAlignWeight
                onChange={(patch: Record<string, any>) => {
                    if (patch && typeof patch === "object" && patch.title && typeof patch.title === "object") {
                        onChange(patch.title);
                    }
                }}
            />
        </div>
    );
}
