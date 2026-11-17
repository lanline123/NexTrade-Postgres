/**
 * 标准装修编辑器壳（2026-11-17 抽取，Step 2）
 *
 * 布局：顶栏槽 + 三栏（左库 295px / 画布自适应 / 右栏 380px），尺寸与
 * 底色见 editor-shell.css 的 .dec-shell* 段（唯一标准，两端共用）。
 * PC 与 H5 编辑器只注入各自的顶栏、模块库、画布与设置面板内容，
 * 不再各写一套壳样式。preview=true 时隐藏左右两栏（预览态）。
 */
import type { ReactNode } from "react";
import "./editor-shell.css";

export interface EditorShellProps {
    /** 顶栏（各端自带内容：标题编辑、语言、保存/发布按钮组等） */
    topbar?: ReactNode;
    /** 左侧模块库（一般放 LibraryPanel） */
    library?: ReactNode;
    /** 右侧设置面板（一般放 DecSettingsPanel 或各端既有面板根） */
    settings?: ReactNode;
    /** 中间画布 */
    children: ReactNode;
    /** 预览态：隐藏左右两栏 */
    preview?: boolean;
    /** 壳根附加类名（各端过渡期的页面级修饰类挂这里） */
    className?: string;
}

export default function EditorShell({ topbar, library, settings, children, preview, className }: EditorShellProps) {
    return (
        <div className={`dec-shell-root${className ? ` ${className}` : ""}`}>
            {topbar}
            <div className="dec-shell">
                {!preview && library ? <div className="dec-shell__lib">{library}</div> : null}
                <div className="dec-shell__canvas">{children}</div>
                {!preview && settings ? <div className="dec-shell__props">{settings}</div> : null}
            </div>
        </div>
    );
}
