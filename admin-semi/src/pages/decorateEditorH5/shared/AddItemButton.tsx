/**
 * 添加按钮（H5 装修编辑器共享，2026-11-21 样式统一，我方新增）
 *
 * 设置面板内一切「添加 X」列表追加按钮的唯一口径：整宽虚线直角钮
 * （+ 字形 + 文案），取值与分组滑动线 2026-11-21 对 GroupCardList
 * （./styleEdits）添加按钮的现代化改版逐项一致——全编辑器只许这一套：
 * marginTop 12、padding 9px 16px、1px dashed 边框（常态 #c6cbd4、
 * hover var(--semi-color-primary, #155bd4)）、borderRadius 0、白底、
 * 文案 13px/20px #1c1f23（hover 转主色）、+ 字形 15px、gap 6。
 *
 * 取代旧 .dec-pic-add-btn（实线蓝边 #155bd4 + 2px 圆角 + 14px 蓝字，
 * 定义在 styles/css/decorate.css 且作用域限定 .dec-pic-group 之内）：
 * 作用域外的调用点（noticeA3 添加公告、newcomerCombo 添加优惠券、
 * GroupCardList 添加一组）根本命中不了规则、渲染为无样式裸链接，
 * 作用域内（PicListField）则是过时的蓝框样式——即用户 2026-11-21
 * 指认的「添加公告 / 添加一组」同病。本组件以行内样式承接，不依赖
 * 任何 CSS 作用域。GroupCardList 的同值行内实现待两包合并后收口至此。
 */
import { useState, type CSSProperties, type ReactNode } from "react";

export default function AddItemButton({
    children,
    onClick,
    style
}: {
    children: ReactNode;
    onClick?: () => void;
    style?: CSSProperties;
}) {
    const [hover, setHover] = useState(false);
    return (
        <a
            onClick={onClick}
            onMouseEnter={() => setHover(true)}
            onMouseLeave={() => setHover(false)}
            style={{
                marginTop: 12,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: 6,
                padding: "9px 16px",
                border: `1px dashed ${hover ? "var(--semi-color-primary, #155bd4)" : "#c6cbd4"}`,
                borderRadius: 0,
                background: "#fff",
                color: hover ? "var(--semi-color-primary, #155bd4)" : "#1c1f23",
                fontSize: 13,
                lineHeight: "20px",
                cursor: "pointer",
                boxSizing: "border-box",
                ...style
            }}
        >
            <span style={{ fontSize: 15, lineHeight: 1 }}>+</span>
            {children}
        </a>
    );
}
