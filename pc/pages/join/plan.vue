<template>
    <div style="background-color: #fff">
        <CommonHeader title="招商计划"></CommonHeader>
        <JoinHeader></JoinHeader>
        <div class="standard__wrap">
            <div v-for="item in joinStore.module2" class="container">
                <h3>{{ item.title }}</h3>
                <div class="model-body">
                    <span class="content" v-html="item.content"></span>
                </div>
            </div>
        </div>
    </div>
    <CommonPageFooter :mt30="false"></CommonPageFooter>
</template>
<script lang="ts" setup>
import { ref } from "vue";
import JoinHeader from "~/pages/join/joinHeader.vue";
import { useJoinStore } from "~/store/join";
import { getMobileNav } from "~/api/home/home";
const joinStore = useJoinStore();
const getBackgroundPicture = async () => {
    try {
        const result = await getMobileNav({ decorateSn: "investmentPlan" });
        Object.assign(joinStore.module2, result?.data);
    } catch (error) {
        console.log(error);
    } finally {
        joinStore.module2Loaded = true;
    }
};
if (joinStore.module2Loaded === false) {
    getBackgroundPicture();
}
</script>
<style lang="less" scoped>
.standard__wrap {
    background-color: #f5f5f5;
    padding: 10px 0 30px;
    h3 {
        padding: 10px;
        text-align: center;
        color: #333;
        font-size: 26px;
        font-weight: 400;
        margin-top: 20px;
        margin-bottom: 10px;
    }
    .model-body {
        background: #fff;
        padding: 35px 30px 40px;
        p {
            line-height: 2.5;
            font-size: 14px;
        }
    }
}
.content {
    box-sizing: border-box;
    font-size: 14px;
    line-height: 2.5;
}
</style>
