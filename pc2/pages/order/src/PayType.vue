<template>
    <div class="form_item">
        <div class="top_row">
            <div class="tit">{{ $t("确认支付方式") }}</div>
        </div>
        <div class="pay_type">
            <template v-if="paymentTypeList.length > 0">
                <el-radio-group v-model="paytId" @change="onChangePayType" style="display: grid;">
                    <template v-for="item in paymentTypeList" :key="item.typeId">
                        <el-radio  :label="item.typeId" :disabled="item.disabled" v-if="item?.isShow">
                            {{ item.typeName }}
                        </el-radio>
                    </template>
                </el-radio-group>
            </template>

            <span class="errInfo red" v-if="payTypeId === 0" style="display: inline; margin: 0; line-height: 15px">
                {{ $t("请选择支付方式") }}
            </span>
        </div>
    </div>
</template>

<script setup lang="ts">
import type { PaymentTypeItem } from "@/types/order/check";

const props = defineProps({
    payTypeId: {
        type: Number,
        required: true,
        default: 0
    },
    paymentTypeList: {
        type: Array as PropType<PaymentTypeItem[]>,
        required: true,
        default: () => []
    }
});
const emit = defineEmits(["changePayType"]);

const paytId = ref(props.payTypeId);
watch(
    () => props.payTypeId,
    (val) => {
        paytId.value = val;
    }
);

const onChangePayType = (id: any) => {
    emit("changePayType", id);
};
</script>

<style lang="less" scoped></style>
