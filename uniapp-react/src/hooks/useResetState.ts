import { useCallback, useRef, useState } from "react";

/**
 * 可重置状态（自旧 hooks/useResetState.ts 等价移植）。
 * 旧版导出两个 Vue 工具：
 * - useResetReactive(value, clone?) => [reactiveState, reset]
 * - useResettableRef(value, clone?) => [ref, reset]
 * reset 把状态回滚到初始值的深克隆（默认 JSON 深克隆，与旧 defaultClone 一致）。
 * React 对应实现：
 * - useResetReactive → 基于 useState 的对象状态版，返回 [state, setState, reset]
 *   （多返回一个 setState 以便修改，reset 语义与旧版一致）
 * - useResettableRef → 基于 useRef，返回 [ref, reset]
 * 列表场景的 reset 已由 hooks/useList 的 resetState 承接，本文件补通用场景。
 */

export type CloneFn<T> = (value: T) => T;

export const defaultClone = <T,>(value: T): T => {
    if (value === undefined || value === null) return value;
    return JSON.parse(JSON.stringify(value)) as T;
};

export function useResetReactive<T extends object>(value: T, clone: CloneFn<T> = defaultClone) {
    const initialRef = useRef(clone(value));
    const [state, setState] = useState<T>(() => clone(value));
    const reset = useCallback(() => {
        setState(clone(initialRef.current));
    }, [clone]);
    return [state, setState, reset] as const;
}

export function useResettableRef<T>(value: T, clone: CloneFn<T> = defaultClone) {
    const initialRef = useRef(clone(value));
    const ref = useRef<T>(clone(value));
    const reset = useCallback(() => {
        ref.current = clone(initialRef.current);
    }, [clone]);
    return [ref, reset] as const;
}

/** 便捷版：单值状态 + reset（React 惯用形态）。 */
export function useResetState<T>(value: T, clone: CloneFn<T> = defaultClone) {
    const initialRef = useRef(clone(value));
    const [state, setState] = useState<T>(() => clone(value));
    const reset = useCallback(() => {
        setState(clone(initialRef.current));
    }, [clone]);
    return [state, setState, reset] as const;
}

export default useResetState;
