/**
 * 模块级错误隔离（对齐 PC 编辑器 ModuleErrorBoundary 语义）：
 * 单个模块的预览/设置组件渲染抛错时只降级该模块，不拖垮整个编辑器。
 */
import { Component, type ReactNode } from "react";

interface Props {
    label?: string;
    children: ReactNode;
}

interface State {
    failed: boolean;
}

export default class ModuleErrorBoundary extends Component<Props, State> {
    state: State = { failed: false };

    static getDerivedStateFromError(): State {
        return { failed: true };
    }

    componentDidCatch(error: Error): void {
        console.error("[h5-decorate] module render failed:", this.props.label || "", error);
    }

    render(): ReactNode {
        if (this.state.failed) {
            return (
                <div style={{ padding: "18px 10px", textAlign: "center", background: "#fff5f5", color: "#c92a2a", fontSize: 13 }}>
                    该模块渲染失败，已跳过（不影响其他模块）
                </div>
            );
        }
        return this.props.children;
    }
}
