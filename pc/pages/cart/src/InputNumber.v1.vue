<template>
    <div class="cart-num-box flex" :class="disabled ? 'disabled-box' : ''">
        <span class="minus hand-pointer" :class="cartNum < 2 ? 'disabled' : ''" @click="step('min')">－</span>
        <input type="text" :value="cartNum" @input="onInput" @change="onChange" :disabled="disabled" />
        <span class="add hand-pointer" @click="step('add')">+</span>
    </div>
</template>
<script lang="ts" setup>
const { t } = useI18n();

const emit = defineEmits(["change", "update:modelValue"]);
const props = defineProps({
    //是否显示尾部上半部分 1显示 2不显示
    modelValue: {
        type: Number,
        default: 1
    },
    disabled: {
        type: Boolean,
        default: false
    },
    max: {
        type: Number
    }
});

const cartNum = computed({
    get: () => props.modelValue,
    set: (value) => {
        emit("update:modelValue", value);
    }
});

const step = (data: string) => {
    if (props.disabled) return;
    if (data == "min" && cartNum.value > 1) {
        emit("update:modelValue", props.modelValue - 1);
        emit("change");
    }
    if (data == "add") {
        if (props.max && props.modelValue >= props.max) {
            emit("update:modelValue", props.max);
            message.error(t("库存不足"));
            emit("change");
            return;
        }
        emit("update:modelValue", props.modelValue + 1);
        emit("change");
    }
};

// 输入时过滤非数字字符
const onInput = (event: Event) => {
    const input = event.target as HTMLInputElement;
    input.value = input.value.replace(/\D/g, "");
    if (props.max && parseInt(input.value) > props.max) {
        input.value = props.max.toString();
        message.error(t("库存不足"));
    }
};

const onChange = (event: Event) => {
    const input = event.target as HTMLInputElement;
    let num = input.value;

    if (num === "" || num === null) {
        num = "1";
    } else {
        // 确保是整数
        num = parseInt(num, 10).toString();

        // 确保是正数且不小于1
        if (isNaN(parseInt(num)) || parseInt(num) < 1) {
            num = "1";
        }
    }

    // 更新输入框显示值
    input.value = num;

    // 向父组件发送更新事件
    emit("update:modelValue", parseInt(num, 10));
    emit("change");
};
</script>
<style lang="less" scoped>
.cart-num-box {
    background-color: #fff;
    border: 1px solid #dfdfdf;
    border-radius: 2px;
    height: 18px;
    line-height: 18px;
    padding: 3px 0;
    width: 80px;

    span {
        display: block;
        font-family: "Tahoma";
        font-size: 14px;
        overflow: hidden;
        cursor: pointer;
        text-align: center;
        width: 20px;
    }

    .disabled {
        color: #dfdfdf;
        cursor: default;
    }

    input {
        border: none;
        padding: 0;
        border-left: 1px solid #dfdfdf;
        border-right: 1px solid #dfdfdf;
        height: 18px;
        line-height: 18px;
        outline: medium none;
        text-align: center;
        width: 38px;
    }
}
.disabled-box {
    background-color: #eee;
}
</style>
