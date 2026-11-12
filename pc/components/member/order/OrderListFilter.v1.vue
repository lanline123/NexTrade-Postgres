<!--
  订单列表 · 顶部筛选区
  - 订单状态 tab（全部/待支付/待发货/待收货/待评价）
  - 关键词搜索框
  - 数据加载、时间筛选（列头下拉）仍由父组件负责
-->
<template>
    <div v-if="modelValue" class="tag-and-input">
        <div class="tig-tabs">
            <div v-for="item in menuTypeList" :key="item.type" :class="menuType == item.type ? 'active' : ''" class="tab" @click="onChangeType(item.type)">
                <span v-if="item.num" :class="{ 'order-count-shift': item.num > 99 }" class="order-count">{{ formatOrderCount(item.num) }}</span>
                <p>{{ $t(item.value) }}</p>
            </div>
        </div>
        <div class="input">
            <el-input :model-value="keyword" :placeholder="$t('收货人名称或订单号')" type="text" @update:model-value="onKeyword" />
            <el-button class="ml10" @click="emit('search')">{{ $t("搜索") }}</el-button>
        </div>
    </div>
</template>

<script lang="ts" setup>
import type { PropType } from "vue";

interface MenuTypeItem {
    type: string;
    value: string;
    num: number;
}

defineProps({
    modelValue: {
        type: Number,
        default: 1
    },
    menuType: {
        type: String,
        default: "all"
    },
    menuTypeList: {
        type: Array as PropType<MenuTypeItem[]>,
        default: () => []
    },
    keyword: {
        type: String,
        default: ""
    }
});

const emit = defineEmits(["update:keyword", "change-type", "search"]);

const onChangeType = (type: string) => emit("change-type", type);
const onKeyword = (val: string) => emit("update:keyword", val);
const formatOrderCount = (count: number) => (count > 99 ? "99+" : count);
</script>

<style lang="less" scoped>
@import "/assets/css/member/member.v1";

.tag-and-input {
    display: flex;
    flex-direction: row;
    justify-content: space-between;

    .image {
        margin-bottom: 20px;
    }

    .input {
        display: flex;
        align-items: center;
    }

    .tig-tabs {
        .tab > .order-count-shift {
            right: -24px;
        }
    }
}
</style>
