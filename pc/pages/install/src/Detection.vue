<template>
    <div class="detection-container">
        <h2 class="tit">系统环境检测</h2>
        <div class="content">
            <a-spin :spinning="loading">
                <List size="large" bordered :data-source="filterState">
                    <template #renderItem="{ item }">
                        <ListItem>
                            <div class="item">
                                <div class="txt ml10">{{ item.name }}</div>
                                <div class="flex flex-align-center">
                                    <div class="mr10">{{ item.value }}</div>
                                    <el-icon v-if="item.status == true" color="#87b848" size="18"><CircleCheckFilled /></el-icon>
                                    <el-icon v-else color="#ff4d4f" size="18"><CircleCloseFilled /></el-icon>
                                </div>
                            </div>
                        </ListItem>
                    </template>
                </List>
            </a-spin>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, onMounted } from "vue";
import { getDetection } from "@/api/install/install";
import { message } from "ant-design-vue";
import { List, ListItem } from "ant-design-vue";
import { CircleCheckFilled, CircleCloseFilled } from "@element-plus/icons-vue";
const emit = defineEmits(["onDisabled"]);
const filterState = ref([]);
const loading = ref(false);
const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getDetection();
        filterState.value = result;
        isTrue(result);
    } catch (error: any) {
        if (error.code === -1) {
            navigateTo("/404");
        }
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const isTrue = (arr: any) => {
    arr.forEach((item: any) => {
        if (item.status === true) {
            emit("onDisabled", false);
        } else {
            emit("onDisabled", true);
        }
    });
};
onMounted(() => {
    loadFilter();
    emit("onDisabled", true);
});
</script>
<style lang="less" scoped>
.detection-container {
    width: 1190px;
    .tit {
        padding: 30px 0;
        color: #333;
        font-size: 18px;
        text-align: center;
    }
    .content {
        font-size: 14px;
        padding: 0 15%;
        .item {
            width: 100%;
            display: flex;
            align-items: center;
            justify-content: space-between;
            font-size: 14px;
        }
    }
}
</style>
