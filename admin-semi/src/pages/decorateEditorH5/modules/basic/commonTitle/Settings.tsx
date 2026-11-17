/**
 * 通用标题 设置（复用共享 TitleSettingsField 通用形态，对齐原版 commonTitle/Edit.vue else 分支：
 * 风格三/背景渐变色仅 product 形态、圆角 max 30、descColor/moreColor 重置回 defaultTitle 值、
 * moreLink 走 SelectLink 对象口径、各字段恒显示与 showTitle/showMore 无联动隐藏）
 *
 * 本模块数据本身即标题对象（位于模块数据根部），TitleSettingsField 按 field="title"
 * 读写嵌套对象，故此处以 { title: data } 适配、onChange 解包后回写根部。
 * moreLink 经 TitleSettingsField 内的 PicLinkField 选择（保留原对象其余键的合并口径）。
 */
import { TitleSettingsField } from "../../../shared/settingWidgets";

export default function CommonTitleSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <TitleSettingsField
                data={{ title: data }}
                onChange={(patch: Record<string, any>) => {
                    if (patch && typeof patch === "object" && patch.title && typeof patch.title === "object") {
                        onChange(patch.title);
                    }
                }}
            />
        </div>
    );
}
