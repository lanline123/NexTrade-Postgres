<template>
    <div></div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive } from "vue";
import { commonLog } from "@/api/common";
// 海外演示站需要用户在 Cookie 同意中勾选「分析性Cookies」后才能上报埋点；国内站及非演示站不受限
import { isOverseas, isDemo } from "@/utils/util";

const props = defineProps({
    productId: {
        type: [Number, String],
        default: ""
    },
    shopId: {
        type: [Number, String],
        default: ""
    },
    shopCategoryId: {
        type: [Number, String],
        default: ""
    },
    immediateLog: {
        type: Boolean,
        default: true
    }
});

interface IFilterParams {
    productId?: string | number;
    shopId?: string | number;
    shopCategoryId?: string | number;
    isNew?: number;
    click?: number;
}
const filterParams = reactive<IFilterParams>({});

const { hasConsent } = useConsent();
const analyticsAllowed = () => (isOverseas() && isDemo() ? hasConsent("analytics") : true);

const __commonLog = async () => {
    if (!analyticsAllowed()) return;
    if (props.productId) {
        filterParams.productId = props.productId;
    }
    if (props.shopId) {
        filterParams.shopId = props.shopId;
    }
    if (props.shopCategoryId) {
        filterParams.shopCategoryId = props.shopCategoryId;
    }
    try {
        const result = await commonLog(filterParams);
    } catch (error) {
        console.error(error);
    }
};

const currentDate = computed(() => {
    const date = new Date();
    const year = date.getFullYear();
    const month = date.getMonth() + 1;
    const day = date.getDate();
    return `${year}-${month}-${day}`;
});

const diffDate = (date1: string, date2: string): boolean => {
    const d1 = new Date(date1);
    const d2 = new Date(date2);
    const diff = Math.abs(d1.getTime() - d2.getTime());
    const oneDay = 24 * 60 * 60 * 1000;
    return diff >= oneDay;
};

const updateVisitor = () => {
    if (!analyticsAllowed()) return;
    const lastDate = localStorage.getItem("lastDate");
    if (!lastDate || diffDate(currentDate.value, lastDate)) {
        filterParams.isNew = 1;
        localStorage.setItem("untranslatedCache", "");
    } else {
        filterParams.click = 1;
    }
    localStorage.setItem("lastDate", currentDate.value);
};
updateVisitor();

if (props.immediateLog) {
    __commonLog();
}

defineExpose({
    __commonLog
});
</script>

<style lang="less" scoped></style>
