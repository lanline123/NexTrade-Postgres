<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="我的订单"></CommonHeader>
    <TopJumpBar tltle="我的订单"></TopJumpBar>
    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div data-list-rev="bTIyODJzdno" class="page-info-body">
                <MemberOrderList></MemberOrderList>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";

definePageMeta({
    middleware: "auth"
});
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";
</style>
