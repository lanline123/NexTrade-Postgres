/**
 * 模块级错误隔离（对齐 PC 编辑器 ModuleErrorBoundary 语义）：
 * 单个模块的预览/设置组件渲染抛错时只降级该模块，不拖垮整个编辑器。
 *
 * 自愈（2026-11-18）：此前 failed 状态整场会话不重置——一次偶发失败（如部署
 * 切换时分片 404）会让该模块设置面板整场卡在红字上、必须手动刷新。现：
 * - resetKey 变化（如切换到另一个模块）时自动重置；
 * - 降级块带「重试」按钮，点后重置并重新挂载子组件（配合 lazyModule 的
 *   缓存逐出，可重新发起分片加载）。
 */
import { Component, Fragment, type ReactNode } from "react";

interface Props {
    label?: string;
    /** 身份键（通常为 moduleIndex）：变化即重置失败状态 */
    resetKey?: string | number;
    children: ReactNode;
}

interface State {
    failed: boolean;
    resetKey?: string | number;
    retryNonce: number;
}

export default class ModuleErrorBoundary extends Component<Props, State> {
    state: State = { failed: false, resetKey: this.props.resetKey, retryNonce: 0 };

    static getDerivedStateFromError(): Partial<State> {
        return { failed: true };
    }

    static getDerivedStateFromProps(props: Props, state: State): Partial<State> | null {
        if (props.resetKey !== state.resetKey) {
            return { failed: false, resetKey: props.resetKey };
        }
        return null;
    }

    componentDidCatch(error: Error): void {
        console.error("[h5-decorate] module render failed:", this.props.label || "", error);
    }

    private handleRetry = (): void => {
        this.setState((s) => ({ failed: false, retryNonce: s.retryNonce + 1 }));
    };

    render(): ReactNode {
        if (this.state.failed) {
            return (
                <div style={{ padding: "18px 10px", textAlign: "center", background: "#fff5f5", color: "#c92a2a", fontSize: 13 }}>
                    <div>该模块渲染失败，已跳过（不影响其他模块）</div>
                    <button
                        type="button"
                        onClick={this.handleRetry}
                        style={{
                            marginTop: 10,
                            padding: "4px 14px",
                            fontSize: 12,
                            color: "#c92a2a",
                            background: "#fff",
                            border: "1px solid #ffc9c9",
                            borderRadius: 6,
                            cursor: "pointer"
                        }}
                    >
                        重试
                    </button>
                </div>
            );
        }
        // retryNonce 进 key：重试时强制子组件整树重挂，懒加载组件会重新发起加载
        return <Fragment key={this.state.retryNonce}>{this.props.children}</Fragment>;
    }
}
