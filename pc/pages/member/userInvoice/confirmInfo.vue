<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="增票资质确认书"></CommonHeader>
    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="confirm-info-content">
                <div class="tips">
                    <p class="title">{{ $t("申请开具增值税专用发票确认书") }}</p>
                    <p>{{ $t("根据国家税法及发票管理相关规定，任何单位和个人不得要求他人为自己开具与实际经营业务情况不符的增值税专用发票包括并不限于") }}</p>
                    <p>1）{{ $t("在没有货物采购或者没有接受应税劳务的情况下要求他人为自己开具增值税专用发票") }}；</p>
                    <p>
                        2）{{ $t("虽有货物采购或者接受应税劳务但要求他人为自己开具数量或金额与实际情况不符的增值税专用发票，否则属于“虚开增值税专用发票”") }}。
                    </p>
                    <p>{{ $t("我已充分了解上述各项相关国家税法和发票管理规定，并确认仅就我司实际购买商品或服务索取发票") }}。</p>
                    <p>{{ $t("如我司未按国家相关规定申请开具或使用增值税专用发票，由我司自行承担相应法律后果") }}。</p>
                </div>
            </div>
        </div>
    </CommonPageContainer>

    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 发票申请确认页
 * - 展示发票申请提交成功的确认信息
 */
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";
.container {
    margin-top: 32px;
}
.confirm-info-content {
    background: #fff;
    border: 0;
    padding: 20px;
    margin-bottom: 10px;
    line-height: 1.5;

    .tips {
        line-height: 3;
        color: #666;

        .title {
            font-size: 18px;
            width: 100%;
            text-align: center;
            font-weight: bold;
        }
    }
}
</style>
