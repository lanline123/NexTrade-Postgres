<template>
    <div class="from">
        <div class="check-box flex align-center">
            <Checkbox :checked="check"></Checkbox>
            <p>{{ $t(`使用${commonStore.integralName}`) }}</p>
            <span>{{ $t(`当前可用${commonStore.integralName}`) }}：<span class="font-color">0</span></span>
        </div>
    </div>
    <div class="from_list" v-if="check">
        <div class="arrow"></div>
        <div class="integral flex">
            <div class="tit">{{ $t(`使用${commonStore.integralName}`) }}：</div>
            <div class="inp_box">
                <div class="inp">
                    <input type="number" value="0" />
                    <span class="blue">【{{ $t(`如何获得${commonStore.integralName}`) }}？】</span>
                </div>
                <div class="tips">{{ $t(`该订单可用${commonStore.integralName}`) }}：0 {{ commonStore.integralName }}</div>
            </div>
        </div>
    </div>
    <div class="from">
        <div class="check-box flex align-center">
            <Checkbox :checked="check2"></Checkbox>
            <p>{{ $t("使用余额") }}</p>
            <span
                >{{ $t("当前账户余额") }} <span class="font-color">{{ balance }} </span></span
            >
        </div>
    </div>
</template>
<script lang="ts" setup>
import { Checkbox } from "ant-design-vue";
import { useCommonStore } from "~/store/common";

const commonStore = useCommonStore();

const check = ref(true);
const check2 = ref(true);
const props = defineProps({
    keyword: { type: String, default: "" }
});
const keyword = ref(props.keyword);
</script>
<style lang="less" scoped></style>
