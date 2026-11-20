/**
 * 标准左侧模块库（2026-11-17 抽取，Step 2）
 *
 * 形态对齐 PC 编辑器 .pc-decorate-lib：295px 白底、分组段头可折叠、
 * 图标卡片三列网格（hover 主题色底白字），样式见 editor-shell.css 的
 * .dec-lib* 段。调用方只给分组与条目数据（图标为 ReactNode），
 * 添加动作走 onAdd 回调（各端自己的 store/校验）。
 */
import { useState, type ReactNode } from "react";
import { IconChevronDown } from "@douyinfe/semi-icons";
import "./editor-shell.css";

export interface LibraryItem {
    key: string;
    label: ReactNode;
    icon?: ReactNode;
    /** 悬停提示（标签截断时看全名），可选 */
    title?: string;
}

export interface LibraryGroup {
    key: string;
    title: ReactNode;
    items: LibraryItem[];
}

export interface LibraryPanelProps {
    groups: LibraryGroup[];
    /** 条目点击添加；第三参为原生点击事件（H5 编辑器据 detail 区分双击连点用，PC 编辑器可忽略） */
    onAdd: (groupKey: string, item: LibraryItem, e?: React.MouseEvent) => void;
    /** 段头上方内容（如页签），可选 */
    header?: ReactNode;
    /** 紧凑模式：去掉分组区 35px 顶距（上方已有页签等头部时用） */
    flush?: boolean;
    /** 提供时条目可拖拽（PC 编辑器拖拽入画布用）：由调用方写 dataTransfer */
    onDragStartItem?: (groupKey: string, item: LibraryItem, e: React.DragEvent) => void;
    /** 提供时条目按下回调（H5 编辑器 pointer 长按拖拽入画布用，2026-11-18）：
        只挂 pointerdown，后续 move/up 由调用方的拖拽控制器接管，不影响 click 添加 */
    onItemPointerDown?: (groupKey: string, item: LibraryItem, e: React.PointerEvent) => void;
}

export default function LibraryPanel({ groups, onAdd, header, flush, onDragStartItem, onItemPointerDown }: LibraryPanelProps) {
    const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
    return (
        <div className="dec-lib">
            {header}
            <div className="dec-lib__inside" style={flush ? { marginTop: 0 } : undefined}>
                {groups.map((group) => {
                    const isCollapsed = !!collapsed[group.key];
                    return (
                        <div key={group.key}>
                            <div
                                className="dec-lib__head"
                                onClick={() => setCollapsed((prev) => ({ ...prev, [group.key]: !prev[group.key] }))}
                            >
                                <span>{group.title}</span>
                                <span className={`dec-lib__arrow${isCollapsed ? "" : " dec-lib__arrow--open"}`}>
                                    <IconChevronDown />
                                </span>
                            </div>
                            {!isCollapsed ? (
                                <div className="dec-lib__list">
                                    {group.items.map((item) => (
                                        <div
                                            key={item.key}
                                            className="dec-lib__item"
                                            onClick={(e) => onAdd(group.key, item, e)}
                                            draggable={!!onDragStartItem}
                                            onDragStart={onDragStartItem ? (e) => onDragStartItem(group.key, item, e) : undefined}
                                            onPointerDown={onItemPointerDown ? (e) => onItemPointerDown(group.key, item, e) : undefined}
                                        >
                                            <span className="dec-lib__pic">{item.icon}</span>
                                            <span className="dec-lib__content" title={item.title}>{item.label}</span>
                                        </div>
                                    ))}
                                </div>
                            ) : null}
                        </div>
                    );
                })}
            </div>
        </div>
    );
}
