<template>
    <div class="cart-num-box flex" :class="disabled ? 'disabled-box' : ''">
        <template v-if="cartNum > 1">
            <span class="min" :class="{ disabled: cartNum <= 1 }" @click="step('min')"></span>
        </template>
        <template v-else>
            <span class="delete" @click="removeCartItem"> </span>
        </template>
        <input type="text" :value="cartNum" @input="onInput" @change="onChange" :disabled="disabled" style="background: #f7f8fc" />
        <span class="plus" :class="{ disabled: cartNum >= max }" @click="step('add')"></span>
    </div>
</template>
<script lang="ts" setup>
/**
 * 购物车数量输入组件
 * - 支持加减按钮和手动输入
 * - 数量为1时减号变为删除图标
 * - 支持库存上限和限购数量校验
 */
const { t } = useI18n();

const emit = defineEmits(["change", "update:modelValue", "removeCartItem"]);

const props = defineProps({
    modelValue: {
        type: Number,
        default: 1
    },
    disabled: {
        type: Boolean,
        default: false
    },
    max: {
        type: Number,
        default: 100
    },
    // 限购
    limitNumber: {
        type: Number
    }
});

const cartNum = computed({
    get: () => props.modelValue,
    set: (value) => {
        emit("update:modelValue", value);
    }
});

/**
 * 步进操作：增加或减少数量
 * - 减少时检查最小值（1）
 * - 增加时检查限购数和库存上限
 */
const step = (data: string) => {
    if (props.disabled) return;
    if (data == "min" && cartNum.value > 1) {
        emit("update:modelValue", props.modelValue - 1);
        emit("change");
    }
    if (data == "add") {
        if(props.limitNumber && cartNum.value >= props.limitNumber) {
            emit("update:modelValue", props.limitNumber);
            message.error(t(`限购${props.limitNumber}件，已达限购数`));
            emit("change");
            return;
        } else if (props.max && props.modelValue >= props.max) {
            emit("update:modelValue", props.max);
            message.error(t("库存不足"));
            emit("change");
            return;
        }
        emit("update:modelValue", props.modelValue + 1);
        emit("change");
    }
};
const removeCartItem = () => {
    emit("removeCartItem");
};

// 输入时过滤非数字字符
const onInput = (event: Event) => {
    const input = event.target as HTMLInputElement;
    input.value = input.value.replace(/\D/g, "");
    if (props.limitNumber && parseInt(input.value) > props.limitNumber) {
        input.value = props.limitNumber.toString();
        message.error(t(`限购${props.limitNumber}件，已达限购数`));
    } else if (props.max && parseInt(input.value) > props.max) {
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
        num = parseInt(num, 10).toString();

        if (isNaN(parseInt(num)) || parseInt(num) < 1) {
            num = "1";
        }
    }

    input.value = num;

    emit("update:modelValue", parseInt(num, 10));
    emit("change");
};
</script>
<style lang="less" scoped>
.cart-num-box {
    display: flex;
    background: #f7f8fc;
    border-radius: 4px;
    padding: 0 5px;
    height: 36px;

    .min {
        background: url("~/assets/images/common/min.png") no-repeat 50% / 16px 16px;

        &.disabled {
            background: url("~/assets/images/common/min-disabled.png") no-repeat 50% / 16px 16px;
        }
    }

    .plus {
        background: url("~/assets/images/common/plus.png") no-repeat 50% / 16px 16px;
        &.disabled {
            background: url("~/assets/images/common/plus-disabled.png") no-repeat 50% / 16px 16px;
        }
    }

    .delete {
        background: url("~/assets/images/common/delete.png") no-repeat 50% / 16px 16px;
    }

    span {
        display: block;
        overflow: hidden;
        cursor: pointer;
        text-align: center;
        width: 21px;
    }

    .disabled {
        color: #dfdfdf;
        cursor: default;
    }

    input {
        border: none;
        padding: 0;
        outline: medium none;
        text-align: center;
        width: 48px;
        font-size: 18px;
    }
}
.disabled-box {
    background-color: #eee;
}
</style>
