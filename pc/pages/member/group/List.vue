<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="我的订单"></CommonHeader>
    <TopJumpBar tltle="我的拼团"></TopJumpBar>
    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="page-info-body">
                <JoiningOrderList></JoiningOrderList>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 我的拼团订单列表页
 * - 展示用户参与的拼团订单
 * - 支持按状态筛选（全部/待成团/已成团/已失败）
 */
import JoiningOrderList from "~/components/member/joining/JoiningOrderList.vue";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";

definePageMeta({
    middleware: "auth"
});
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";
</style>
