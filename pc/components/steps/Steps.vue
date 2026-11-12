<template>
    <div class="steps-row flex align-center hand-pointer">
        <div class="col flex align-center" v-for="(item, index) in stepsList">
            <div class="flex pg_8">
                <SvgIcon v-if="props.sudden > item.suden" class="o_rolConter" name="conter"></SvgIcon>
                <div v-else :class="props.sudden >= item.suden ? 'o_rolOne' : 'o_rol'">
                    {{ index + 1 }}
                </div>
                <div class="stepsTitle">{{ $t(item.lable) }}</div>
            </div>
            <div class="o_line" v-if="index != stepsList.length - 1" :class="props.sudden === index ? 'o_line_red' : ''"></div>
        </div>
    </div>
</template>
<script setup lang="ts">
import { toRefs, ref, computed, reactive } from "vue";
const props = defineProps({
    current: { type: Number, default: 0 },
    sudden: { type: Number, default: 0 }
});
const stepsList = reactive([
    {
        lable: "购物车",
        suden: 7
    },
    {
        lable: "结账",
        suden: 8
    },
    {
        lable: "下单",
        suden: 9
    }
]);
</script>
<style lang="less" scoped>
.flex {
    display: flex;
    align-items: center;
}
.steps-row {
    .col {
        position: relative;
        padding-top: 25px;
        .stepsTitle {
            color: #000000;
            text-align: center;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 14px;
            font-weight: 500;
            padding-left: 8px;
        }
        .o_rolOne {
            background: none repeat scroll 0 0 var(--tag-text);
            border-radius: 999px;
            color: #fff;
            display: block;
            font-size: 13px;
            height: 24px;
            width: 24px;
            line-height: 24px;
            text-align: center;
            z-index: 10;
            font-weight: bold;
        }
        .o_rol {
            background: none repeat scroll 0 0 #98989b;
            border-radius: 999px;
            color: #fff;
            display: block;
            font-size: 13px;
            height: 24px;
            width: 24px;
            line-height: 24px;
            text-align: center;
            z-index: 10;
            font-weight: bold;
        }
        .o_rolConter {
            height: 24px;
            width: 24px;
            color: var(--general);
        }
        .o_line {
            background: none repeat scroll 0 0 #ddd;
            height: 1px;
            width: 315px;
            overflow: hidden;
            z-index: 9;
            margin-left: 5px;
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
.pg_8 {
    padding: 0 8px;
}
</style>
