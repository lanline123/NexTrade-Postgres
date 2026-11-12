<template>
    <div class="tab-list">
        <div v-for="(tab, index) in tabs" :key="index" :class="{ active: activeTab === index }" class="tab-item" @mouseenter="setActiveTab(index)">
            {{ $t(tab) }}
        </div>
    </div>
</template>

<script lang="ts" setup>
import { ref } from "vue";

const tabs = ref(["热销", "新品", "精品"]); // Tab标签数组
const activeTab = ref(0); // 默认激活的Tab索引
const setActiveTab = (index) => {
    activeTab.value = index; // 设置当前激活的Tab
};
</script>

<style scoped>
.tab-list {
    display: block;
    width: 100%;
    overflow-x: auto;
    white-space: nowrap;
    cursor: pointer;

    .tab-item {
        border: 1px solid #eeeeee;
        border-right: 1px solid transparent;
        padding: 4px 16px;
        display: inline-block;
        white-space: nowrap;

        &:last-child {
            border-right: 1px solid #eeeeee;
        }

        &:hover,
        &.active {
            border-color: #e4393c;
            color: #e4393c;
        }
    }
}
</style>
