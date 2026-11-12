<template>
    <div style="background-color: #f2f3f5">
        <Title>{{ isProductOffline ? $t("商品已下架") : `404 - ${$t("页面未找到")}` }}</Title>
        <CommonPageHeader page="index"></CommonPageHeader>
        <CommonPageContainer>
            <div class="page-404">
                <img class="img-404" src="@/assets/images/common/404.png" />
                <div>
                    <h1 class="text-404-desc title">{{ isProductOffline ? $t("商品已下架") : `404 - ${$t("页面未找到")}~` }}</h1>
                    <p class="text-404-desc">
                        {{ isProductOffline ? $t("抱歉商品已下架，去查看其他商品") : `${$t("抱歉，您所请求的页面不存在")}。` }}
                    </p>
                    <div class="text-404-desc" style="margin-top: 20px">
                        <NuxtLink to="/" class="btn-404" replace
                            ><el-button type="primary">{{ $t("返回首页") }}</el-button></NuxtLink
                        >
                    </div>
                </div>
            </div>
        </CommonPageContainer>
    </div>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 404 页面 - 兜底路由
 * - 匹配所有未定义的路由路径
 * - 展示友好的 404 提示信息和返回首页按钮
 * - 商品下架跳到 /404?reason=offline 时，改用下架标题和副标题
 */
const route = useRoute();
const isProductOffline = computed(() => {
    const slug = route.params.slug;
    const path = Array.isArray(slug) ? slug.join("/") : String(slug ?? "");
    return path === "404" && route.query.reason === "offline";
});
const event = useRequestEvent();
if (event) {
    setResponseStatus(event, 404);
}
</script>
<style lang="less" scoped>
.page-404 {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 50px 0;
    .img-404 {
        width: 300px;
        margin-right: 30px;
    }
    .text-404-desc {
        margin-bottom: 10px;
        font-size: 12px;
        color: #999;
    }
    .title {
        font-size: 16px;
        font-weight: bold;
        color: #666;
    }
}
</style>
