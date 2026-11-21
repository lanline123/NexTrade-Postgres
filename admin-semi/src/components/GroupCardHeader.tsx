/**
 * 组头控件（GroupCardHeader）——装修编辑器分组卡片的标题行通用组件（我方新增）。
 *
 * 2026-11-21 按用户指示抽取：此前分组卡的卡头行由 GroupCardList
 * （decorateEditorH5/shared/styleEdits）内联手写，删除是卡片右上角外挂的
 * 灰色徽标（-10px 悬出卡外，与上移/下移不在一行）；公告混排又另有一份
 * 手写克隆。现统一为本组件：
 * - 布局：标题在左；右侧依次并列「上移 / 下移 / 删除 ×」（删除在下移右边）；
 * - 删除 × 颜色跟随后台主题色：经 useBuyStyleThemeColor 取 themeStyle 的
 *   --general（会话级懒加载 /setting/config/themeStyleSettings，与购买按钮
 *   样式选择器 BuyBtnStylePicker 同源同机制；设置面板 DOM 无 --general
 *   变量祖先，故不走 CSS 变量），悬停时叠同色 8% 浅底；
 * - onMove / onRemove 不传则不渲染对应动作（无删除的固定分组、不可删列表
 *   沿用各自原语义）；onClick 承接卡头点击（折叠切换），动作区点击不冒泡。
 *
 * 契约只管呈现：标题文案、排序/删除/折叠的实际数据写回全在调用方，
 * 组对象结构与回调口径与抽取前逐项不变。
 */
import { useState } from "react";
import { useI18n } from "../i18n";
import { useBuyStyleThemeColor } from "./BuyBtnStylePicker";

export interface GroupCardHeaderProps {
    /** 卡头标题（调用方组装，如「第1组」或成员自带名称） */
    title: string;
    /** 是否首组（上移置灰） */
    isFirst?: boolean;
    /** 是否末组（下移置灰） */
    isLast?: boolean;
    /** 排序回调，delta=-1 上移 / +1 下移；不传不渲染排序链 */
    onMove?: (delta: number) => void;
    /** 删除回调；不传不渲染删除 × */
    onRemove?: () => void;
    /** 卡头点击（折叠切换等）；不传则默认光标 */
    onClick?: () => void;
}

export default function GroupCardHeader({ title, isFirst, isLast, onMove, onRemove, onClick }: GroupCardHeaderProps) {
    const { t } = useI18n();
    const themeColor = useBuyStyleThemeColor();
    const [removeHover, setRemoveHover] = useState(false);
    // 悬停浅底：十六进制主题色拼 8% alpha（与 BuyBtnStylePicker 选中浅底同法），非 hex 走兜底
    const removeHoverBg = /^#[0-9a-f]{6}$/i.test(themeColor) ? `${themeColor}14` : "rgba(101, 196, 170, 0.08)";
    return (
        <div
            style={{
                backgroundColor: "#f7f8fa",
                padding: "10px 12px",
                fontSize: 13,
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                cursor: onClick ? "pointer" : "default"
            }}
            onClick={onClick}
        >
            <span style={{ fontWeight: 600, color: "#1c1f23" }}>{title}</span>
            <span style={{ display: "flex", gap: 10, fontSize: 12, alignItems: "center" }} onClick={(e) => e.stopPropagation()}>
                {onMove ? (
                    <>
                        <a className={isFirst ? "is-disabled" : ""} onClick={() => onMove(-1)}>
                            {t("上移")}
                        </a>
                        <a className={isLast ? "is-disabled" : ""} onClick={() => onMove(1)}>
                            {t("下移")}
                        </a>
                    </>
                ) : null}
                {onRemove ? (
                    <span
                        title={t("删除")}
                        onClick={onRemove}
                        onMouseEnter={() => setRemoveHover(true)}
                        onMouseLeave={() => setRemoveHover(false)}
                        style={{
                            color: themeColor,
                            backgroundColor: removeHover ? removeHoverBg : "transparent",
                            fontSize: 15,
                            lineHeight: 1,
                            padding: "1px 3px",
                            cursor: "pointer",
                            boxSizing: "border-box"
                        }}
                    >
                        ×
                    </span>
                ) : null}
            </span>
        </div>
    );
}
