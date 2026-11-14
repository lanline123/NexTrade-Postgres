import { Text, View } from "@tarojs/components";

import { t } from "@/utils/i18n";

import "./TigNumberStepper.scss";

interface Props {
    /** 当前值（旧 modelValue） */
    value?: number;
    disabled?: boolean;
    max?: number;
    min?: number;
    integer?: boolean;
    /** 减到 min 时显示删除（旧 isDelete，触发 onDelete） */
    isDelete?: boolean;
    onChange?: (value: number) => void;
    onUpdateValue?: (value: number) => void;
    onDelete?: () => void;
}

/**
 * 数字步进器（自旧 tig-number-stepper.vue 等价移植）。
 * 注意与 tig-number-box（带输入框，规格弹窗用）及购物车自研
 * pages/cart/src/NumberStepper（购物车专用：min 删除/max 库存 toast）
 * 均不同物：本组件是旧 tig-number-stepper 的通用版 props/emits：
 * modelValue/disabled/max/min/integer/isDelete，change/update:modelValue/delete。
 */
export default function TigNumberStepper({
    value = 1,
    disabled = false,
    max = 100,
    min = 1,
    integer = true,
    isDelete = false,
    onChange,
    onUpdateValue,
    onDelete
}: Props) {
    const emit = (next: number) => {
        const v = integer ? Math.round(next) : Math.round(next * 100) / 100;
        onChange?.(v);
        onUpdateValue?.(v);
    };

    const handleMinus = () => {
        if (disabled) return;
        if (value <= min) {
            if (isDelete) onDelete?.();
            return;
        }
        emit(Math.max(min, value - 1));
    };

    const handlePlus = () => {
        if (disabled) return;
        if (value >= max) return;
        emit(Math.min(max, value + 1));
    };

    return (
        <View className={`tig-number-stepper${disabled ? " is-disabled" : ""}`}>
            <View className="tig-number-stepper__btn" onClick={handleMinus}>
                <Text>{isDelete && value <= min ? t("删除") : "−"}</Text>
            </View>
            <View className="tig-number-stepper__value">
                <Text>{value}</Text>
            </View>
            <View className="tig-number-stepper__btn" onClick={handlePlus}>
                <Text>+</Text>
            </View>
        </View>
    );
}
