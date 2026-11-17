/**
 * 标准设置面板外壳（2026-11-17 抽取，Step 2）
 *
 * 结构对齐 PC 编辑器现行 .pc-module-settings：灰底面板（.dec-settings-scope）
 * + 白底标题区（标题 17px/650 + 可选说明）+ 可选胶囊页签 + 内容区。
 * 内容区由调用方放入 .dec-settings-body 白卡（或各端过渡期别名类），
 * 字段行/分组标题带等原语样式全部来自 editor-shell.css，勿另写。
 */
import type { ReactNode } from "react";
import "./editor-shell.css";

export interface DecSettingsTab {
    key: string;
    label: ReactNode;
}

export interface DecSettingsPanelProps {
    title: ReactNode;
    desc?: ReactNode;
    tabs?: DecSettingsTab[];
    activeTab?: string;
    onTabChange?: (key: string) => void;
    /** 标题行右侧附加内容（如 PC 公告模块的「全局重置」按钮），可选 */
    extra?: ReactNode;
    className?: string;
    children: ReactNode;
}

export default function DecSettingsPanel({
    title,
    desc,
    tabs,
    activeTab,
    onTabChange,
    extra,
    className,
    children
}: DecSettingsPanelProps) {
    return (
        <div className={`dec-settings-scope dec-settings-panel${className ? ` ${className}` : ""}`}>
            <div
                className="dec-edit-title"
                style={extra ? { display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 12 } : undefined}
            >
                <div>
                    <h3>{title}</h3>
                    {desc ? <div className="dec-edit-desc">{desc}</div> : null}
                </div>
                {extra}
            </div>
            {tabs && tabs.length > 0 ? (
                <div className="settings-tabs-nav">
                    {tabs.map((tab) => (
                        <div
                            key={tab.key}
                            className={`settings-tabs-item${tab.key === activeTab ? " active" : ""}`}
                            onClick={() => onTabChange?.(tab.key)}
                        >
                            {tab.label}
                        </div>
                    ))}
                </div>
            ) : null}
            {children}
        </div>
    );
}
