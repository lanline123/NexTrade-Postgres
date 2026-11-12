<template>
    <div v-popover="popoverRef" class="dropdown">
        <span class="button"> {{ $t(selectList.find((item) => item.value === selectId)?.label) }}</span>
        <span :class="isShow ? 'icon-shangjiantou' : 'icon-xiajiantou'" class="iconfont-pc" style="font-size: 12px"></span>
    </div>
    <el-popover
        :show-arrow="false"
        :hide-after="0"
        transition=""
        ref="popoverRef"
        persistent
        trigger="click"
        v-model:visible="isShow"
        virtual-triggering
        @before-enter="isShow = true"
        @before-leave="isShow = false"
    >
        <div v-for="(item, index) in selectList" :key="index" class="dropdown-item" @click="selectItem(item)">
            {{ $t(item.label) }}
        </div>
    </el-popover>
</template>
<script lang="ts" setup>
import { ref } from "vue";

const props = defineProps({
    selectList: {
        type: Array,
        default: []
    }
});
// 接收父组件传的方法
const emit = defineEmits<{
    (event: "loadFilter"): void;
}>();
const selectId = defineModel();
const isShow = ref(false);
const popoverRef = ref();

const selectItem = (item: any) => {
    selectId.value = item.value;
    isShow.value = false;
    emit("loadFilter");
};
</script>
<style lang="less" scoped>
.dropdown {
    cursor: pointer;

    .button {
        margin-right: 5px; // 根据需要增减边距
    }
}

.dropdown-item {
    line-height: 2.5;
    text-align: center;
    cursor: pointer;

    &:hover {
        background-color: #f0f0f0;
    }
}
</style>
