/**
 * 模块操作条（画布右侧外 −40px 白底竖排：上移/下移/显隐/复制/删除）
 *
 * 兼容渲染（Canvas 的 ModuleItem）与真实渲染（IframeCanvas 的覆盖层槽位）共用，
 * 保证两种模式的操作口径一致（原版 .module-operate，样式在 decorate.css）。
 * fixed 模块隐藏显隐/复制/删除三项（与原版一致）。
 */
import { Toast } from "@douyinfe/semi-ui";
import { IconChevronDown, IconChevronUp, IconCopy, IconDelete, IconEyeClosed, IconEyeOpened } from "@douyinfe/semi-icons";
import { editorStore } from "../store/editorStore";
import type { DecorateModuleBase } from "../contract/types";
import { useI18n } from "../../../i18n";

export function tryMoveModule(from: number, to: number, t: (key: string) => string) {
    if (to < 0) return;
    const state = editorStore.getState();
    if (to >= state.moduleList.length) return;
    const result: any = editorStore.moveModule(from, to);
    if (result !== true) Toast.warning(t(String(result?.reason || "")));
}

export default function ModuleOperateBar({ moduleData, index }: { moduleData: DecorateModuleBase; index: number }) {
    const { t } = useI18n();
    const isHidden = moduleData.isShow === false;

    const operateItems: Array<{ key: string; tip: string; icon: React.ReactNode; onClick: () => void; hidden?: boolean }> = [
        { key: "up", tip: t("上移"), icon: <IconChevronUp />, onClick: () => tryMoveModule(index, index - 1, t) },
        { key: "down", tip: t("下移"), icon: <IconChevronDown />, onClick: () => tryMoveModule(index, index + 1, t) },
        {
            key: "visibility",
            tip: isHidden ? t("显示") : t("隐藏"),
            icon: isHidden ? <IconEyeClosed /> : <IconEyeOpened />,
            onClick: () => editorStore.toggleModuleVisibility(index),
            hidden: !!moduleData.fixed
        },
        { key: "copy", tip: t("复制"), icon: <IconCopy />, onClick: () => editorStore.copyModule(index), hidden: !!moduleData.fixed },
        { key: "delete", tip: t("删除"), icon: <IconDelete />, onClick: () => editorStore.removeModule(index), hidden: !!moduleData.fixed }
    ];

    return (
        <div
            className="module-operate"
            onClick={(e) => e.stopPropagation()}
            onPointerDown={(e) => e.stopPropagation()}
        >
            {operateItems
                .filter((item) => !item.hidden)
                .map((item) => (
                    <div
                        key={item.key}
                        className="module-operate-item"
                        onClick={(e) => {
                            e.stopPropagation();
                            item.onClick();
                        }}
                    >
                        {item.icon}
                        <div className="opt-tip">{item.tip}</div>
                    </div>
                ))}
        </div>
    );
}
