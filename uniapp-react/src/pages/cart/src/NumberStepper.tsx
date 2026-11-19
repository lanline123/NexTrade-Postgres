import { useEffect, useRef, useState } from "react";
import { Input, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import { staticResource } from "@/utils";
import { t } from "@/utils/i18n";

import "./NumberStepper.scss";

interface Props {
    value?: number;
    disabled?: boolean;
    max?: number;
    min?: number;
    integer?: boolean;
    /** 减到最小值时改为触发删除确认（购物车行：减到 1 变删除图标） */
    isDelete?: boolean;
    /** 对应旧 @change，载荷 { value, type }（type: "" | "minus" | "add"） */
    onChange?: (e: { value: number; type: string }) => void;
    /** 对应旧 @delete */
    onDelete?: () => void;
}

/**
 * 数量步进器（自旧 components/tig/tig-number-stepper/tig-number-stepper.vue 逐行移植）：
 * - 减到 min 时若 isDelete 则走删除回调（购物车减到 1 出删除图标）
 * - 加到 max 时 toast「库存不足」
 * - 聚焦可清空，失焦回填并钳制到 [min, max]
 */
export default function NumberStepper({ value = 1, disabled = false, max = 100, min = 1, integer = true, isDelete = false, onChange, onDelete }: Props) {
    // 旧 currentValue 为非受控内部值（modelValue 变化时同步）；React 版用 ref 承载 + state 仅驱动渲染
    const currentValueRef = useRef(value);
    const [, setRenderTick] = useState(0);
    const [focused, setFocused] = useState(false);
    const [inputText, setInputText] = useState("");

    function format(input: string | number): number {
        const filtered = filter(input);
        const parsed = Number(filtered);
        return Number.isFinite(parsed) ? Math.max(Math.min(max, parsed), min) : min;
    }

    function filter(input: string | number) {
        let next = String(input).replace(/[^0-9.-]/g, "");
        if (integer && next.indexOf(".") !== -1) {
            next = next.split(".")[0];
        }
        return next;
    }

    // 对应旧 watch(() => props.modelValue)
    useEffect(() => {
        const next = value;
        if (next !== currentValueRef.current) {
            currentValueRef.current = next;
            setRenderTick((tick) => tick + 1);
        }
        if (focused) {
            const textNum = Number(inputText);
            if (inputText !== "" && Number.isFinite(textNum) && textNum === value) return;
            setInputText(String(currentValueRef.current));
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [value, max, min, disabled]);

    const step = (type: string) => {
        if (disabled) return;
        if (currentValueRef.current <= min && type === "minus") {
            if (isDelete) {
                onDelete?.();
            }
            return;
        }
        if (currentValueRef.current >= max && type === "add") {
            Taro.showToast({
                icon: "none",
                title: t("库存不足")
            });
            return;
        }

        const diff = type === "minus" ? -1 : 1;
        const next = format(Number(currentValueRef.current) + diff);
        emitChange(next, type);
    };

    const onFocus = () => {
        if (disabled) return;
        setFocused(true);
        setInputText(String(currentValueRef.current));
    };

    const onInput = (e: any) => {
        if (disabled) return;
        const next = String(e?.detail?.value ?? e?.target?.value ?? "");
        setInputText(next);

        if (next === "") return;

        const formatted = format(next);
        emitChange(formatted);
    };

    const onBlur = () => {
        setFocused(false);
        const current = currentValueRef.current;
        const next = inputText === "" ? min : format(inputText);
        setInputText("");
        if (disabled || inputText === String(current) || next === current) return;
        emitChange(next);
    };

    const emitChange = (next: number, type = "") => {
        currentValueRef.current = next;
        setRenderTick((tick) => tick + 1);
        onChange?.({ value: next, type });
    };

    const minusClass = currentValueRef.current <= 1 && isDelete ? " del" : currentValueRef.current <= 1 && !isDelete ? " disabled" : "";
    const plusClass = currentValueRef.current >= max ? " disabled" : "";

    // 原样式为 background-image（@/static/images/common/minus.png 等），React 版以内联 backgroundImage 等价平铺
    const minusBg = { backgroundImage: `url(${staticResource(currentValueRef.current <= 1 && isDelete ? "common/delete.png" : currentValueRef.current <= 1 ? "common/minus2.png" : "common/minus.png")})` };
    const plusBg = { backgroundImage: `url(${staticResource(currentValueRef.current >= max ? "common/plus2.png" : "common/plus.png")})` };

    return (
        <View className="tig-number-step flex stepper-shell-Y3RoeTIyODI">
            <View className={`minus btn flex align-center${minusClass}`} style={minusBg as any} onClick={() => step("minus")} />
            <View className="value flex align-center">
                <Input className="stepper-input" disabled={disabled} value={focused && !disabled ? inputText : String(currentValueRef.current)} type="number" placeholder="" onFocus={onFocus} onInput={onInput} onBlur={onBlur} />
            </View>
            <View className={`plus btn flex align-center${plusClass}`} style={plusBg as any} onClick={() => step("add")} />
        </View>
    );
}
