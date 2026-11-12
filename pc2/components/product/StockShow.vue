<template>
    <div style="display: inline-flex">
        {{ $t("库存") }}：
        <em id="storage" class="storage">
            <template v-if="showSeckillText">
                <span v-if="type == 2" style="color: red">{{ $t("限购") }} {{ maxProductNumber }} {{ $t("件") }} </span>
            </template>
            <template v-else>
                <span v-if="type == 1" style="color: red">{{ $t("缺货") }}</span>
                <span v-if="type == 2" style="color: red">{{ $t("仅剩") }} {{ modelValue }} {{ $t("件") }} </span>
                <span v-if="type == 3">{{ $t("充足") }}</span>
            </template>
        </em>
    </div>
</template>

<script setup>
import { computed, ref } from "vue";
const props = defineProps({
    modelValue: {
        type: Number,
        default: 0
    },
    isSeckill: {
        type: Number,
        default: 0
    },
    maxProductNumber: {
        type: Number,
        default: 0
    }
});
const type = computed(() => {
    if (props.modelValue == 0) {
        return 1;
    } else if (props.modelValue > 0 && props.modelValue <= 20) {
        return 2;
    } else {
        return 3;
    }
});

const showSeckillText = computed(() => {
    return props.isSeckill > 0 && props.maxProductNumber > 0 ? true : false;
});
</script>

<style lang="less" scoped></style>
