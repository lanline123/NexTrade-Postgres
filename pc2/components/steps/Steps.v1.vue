<template>
    <div class="steps-row flex align-center hand-pointer">
        <div class="col flex align-center" v-for="(item, index) in stepsList" :class="props.current >= index ? 'active' : ''">
            <p>{{ $t(item) }}</p>
            <div class="o_rol">
                {{ index + 1 }}
            </div>
            <div class="o_line" v-if="index != stepsList.length - 1" :class="props.current > index ? 'o_line_red' : ''"></div>
        </div>
    </div>
</template>
<script setup lang="ts">
import { toRefs, ref, computed, reactive } from "vue";
const props = defineProps({
    current: { type: Number, default: 0 }
});
const stepsList = reactive(["查看购物车", "填写购买信息", "订单支付", "订单完成"]);
</script>
<style lang="less" scoped>
.steps-row {
    .col {
        position: relative;
        padding-top: 25px;
        p {
            color: #999;
            position: absolute;
            width: 100px;
            text-align: center;
            top: 0px;
            left: -40px;
        }
        .o_rol {
            background: none repeat scroll 0 0 #ddd;
            border-radius: 999px;
            color: #fff;
            display: block;
            font-size: 13px;
            height: 22px;
            width: 22px;
            line-height: 22px;
            text-align: center;
            z-index: 10;
            font-weight: bold;
        }
        .o_line {
            background: none repeat scroll 0 0 #ddd;
            height: 4px;
            width: 110px;
            overflow: hidden;
            z-index: 9;
        }
        .o_line_red {
            background-color: var(--general);
        }
    }
    .active {
        p {
            color: var(--general);
        }
        .o_rol {
            background: none repeat scroll 0 0 var(--general);
            border-color: #fff;
            color: #fff;
        }
    }
}
</style>
