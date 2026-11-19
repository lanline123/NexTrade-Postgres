import { Component, type ReactNode} from "react";
import { View} from "@tarojs/components";

import "./ModuleErrorBoundary.scss";

interface Props {
/** 模块类型（仅用于报错日志定位） */
moduleType?: string;
/** 模块在列表中的下标（仅用于报错日志定位） */
moduleIndex?: number;
/**
* 复位键（传模块数据对象本身）：编辑器里用户改好出错模块的数据后，数据
* 对象换新引用即自动重试渲染，不必刷新 iframe；引用不变则保持降级态。
*/
resetKey?: unknown;
/**
* 占位模式：true 时 fallback 渲染朴素提示块（编辑器嵌入预览用，文案与
* admin 兼容渲染同口径）；默认 false 只渲染空 View（线上口径，不给顾客看报错文案）。
*/
placeholder?: boolean;
children: ReactNode;
}

interface State {
hasError: boolean;
prevResetKey?: unknown;
}

/**
* 模块级错误隔离（2026-11-19）：单个装修模块渲染抛错时只降级该
* 模块，不再拖垮整页（此前 shopInfo 无 shopInfo 传参抛错曾让编辑器真实渲染
* 整页空白）。fallback 恒渲染一个 View 占位，保证 decoratePreview 的布局
* 槽位（按容器 children 下标对齐）不位移、decorate-rendered 仍可发出。
*/
export default class ModuleErrorBoundary extends Component<Props, State> {
state: State = { hasError: false, prevResetKey: this.props.resetKey };

static getDerivedStateFromProps(props: Props, state: State): Partial<State> | null {
if (props.resetKey !== state.prevResetKey) {
return { hasError: false, prevResetKey: props.resetKey };
}
return null;
}

static getDerivedStateFromError(): Partial<State> {
return { hasError: true };
}

componentDidCatch(error: Error) {
console.error(
`[ModuleErrorBoundary] 模块渲染失败，已跳过 type=${this.props.moduleType?? "?"} index=${this.props.moduleIndex?? "?"}`,
error
);
}

render() {
if (this.state.hasError) {
return this.props.placeholder? (
<View className="module-error-boundary__placeholder">该模块渲染失败，已跳过</View>
): (
<View className="module-error-boundary__empty" />
);
}
return this.props.children;
}
}
