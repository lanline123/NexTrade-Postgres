<template>
    <div class="small-personal_center">
        <div class="uphoto-wrap">
            <NuxtLink to="/member/">
                <img v-if="userStore.userInfo.avatar" style="width: 64px; height: 64px" :src="imageFormat(userStore.userInfo.avatar)" alt="" />
                <img v-else-if="commonStore.defaultAvatar" style="width: 64px; height: 64px" :src="imageFormat(commonStore.defaultAvatar)" />
            </NuxtLink>
        </div>
        <div class="login-box" v-if="userStore.userInfo && userStore.userInfo.userId > 0">
            <div class="acea-row row-center-wrapper">
                <div>
                    {{ $t("你好") }},
                    <NuxtLink to="/member/">{{ userStore.userInfo.nickname ? userStore.userInfo.nickname : userStore.userInfo.dimUsername }}</NuxtLink>
                </div>
                <img v-if="userStore.userInfo.rankLogo" class="ranktag" :src="imageFormat(userStore.userInfo.rankLogo)" />
            </div>

            <div class="mt10 acea-row" style="cursor: pointer">
                <NuxtLink to="/member/point/list" class="flex1">
                    <div class="tab-color f14 fw700">{{ userStore.userInfo.points }}</div>
                    <div class="gray9">{{ commonStore.integralName }}</div>
                </NuxtLink>
                <NuxtLink to="/member/coupon/list" class="flex1">
                    <div class="tab-color f14 fw700">{{ userStore.userInfo.coupon }}</div>
                    <div class="gray9">{{ $t("优惠券") }}</div>
                </NuxtLink>
            </div>
        </div>
        <div class="login-box" v-if="!userStore.isAuthenticated">
            <div>{{ $t("你好") }} ~</div>
            <!-- <div class="login-wel">{{$t('欢迎光临NexTrade商城')}}！</div> -->
            <div class="login-wel">{{ $t("欢迎光临{0}商城", [commonStore.shopName]) }}！</div>
            <div class="mt10">
                <NuxtLink to="/member/login/" class="login_btn">{{ $t("登录") }}</NuxtLink>
                <NuxtLink v-if="commonStore.shopRegClosed === 0" to="/member/register/" class="login_btn">{{ $t("注册") }}</NuxtLink>
            </div>
        </div>
        <div class="vip-list acea-row">
            <div class="lysy" v-for="(item, index) in value.picList3" :key="index">
                <NuxtLink target="_blank" :to="item.picLink?.link">
                    <div class="">
                        <img style="width: 32px; height: 32px" :src="imageFormat(item.picUrl)" />
                    </div>
                    <p>{{ item.picTitle }}</p>
                </NuxtLink>
            </div>
        </div>
        <div class="notice-list">
            <template v-for="(item, index) in filterState" :key="index">
                <NuxtLink v-if="index < 6" class="item" :to="'/article/news/list?articleId=' + item.articleId" target="_blank">
                    <span class="tab-color">【{{ item.articleTag }}】</span>
                    <span class="line1">{{ item.articleTitle }}</span>
                </NuxtLink>
            </template>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, reactive } from "vue";
import { useUserStore } from "~/store/user";
import { imageFormat } from "~/utils/format";
import type { ArticleFilterParams, ArticleFilterState } from "~/types/article/article";
import { getArticleList } from "~/api/article/article";
import { useCommonStore } from "~/store/common";

const commonStore = useCommonStore();
const userStore = useUserStore();
const props = defineProps({
    value: {
        type: Object,
        default: {}
    }
});

const filterState = ref(<ArticleFilterState[]>[]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<ArticleFilterParams>({
    //初使化用于查询的参数
    page: 1,
    size: 9,
    categorySn: "bzgg"
});

const loadFilter = async () => {
    try {
        loading.value = true;
        const result = await getArticleList({ ...filterParams });
        filterState.value = result.records;
        total.value = result.total;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
loadFilter();
</script>
<style lang="less" scoped>
.acea-row {
    display: flex;
    flex-wrap: wrap;
}
.acea-row.row-middle {
    align-items: center;
}
.acea-row.row-right {
    justify-content: flex-end;
}
.acea-row.row-center-wrapper {
    align-items: center;
    justify-content: center;
}
.acea-row.row-between-wrapper {
    align-items: center;
    justify-content: space-between;
}
.acea-row .flex1 {
    flex: 1;
}
.mt10 {
    margin-top: 10px;
}
.mr10 {
    margin-right: 10px;
}
.f14 {
    font-size: 14px;
}
.tab-color {
    color: var(--general);
}
.gray9 {
    color: #999;
}
.line1 {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.small-personal_center {
    width: 190px;
    height: 460px;
    background-color: #fff;
    background-image: url("/assets/images/index/vip_bg2.png");
    background-repeat: no-repeat;
    overflow: hidden;
    padding-top: 10px;
}
.small-personal_center .uphoto-wrap {
    width: 60px;
    height: 60px;
    padding: 4px;
    border-radius: 50%;
    background-color: rgba(0, 0, 0, 0.1);
    margin: auto;
}
.small-personal_center .uphoto-wrap a {
    display: block;
    width: 60px;
    height: 60px;
    overflow: hidden;
    border-radius: 50%;
    background: url("/assets/images/index/globalhead_sprite.png") no-repeat 0 -150px;
}
.small-personal_center .login-box {
    text-align: center;
    color: #666;
    line-height: 20px;
    padding-top: 10px;
    height: 76px;
    .ranktag {
        margin-left: 3px;
        height: 16px;
        width: 16px;
    }
}
.small-personal_center .login-box .login-wel {
    height: 20px;
    line-height: 20px;
    overflow: hidden;
    padding: 0 5px;
}
.small-personal_center .login-box a.login_btn,
.small-personal_center .login-box a.regist_btn {
    display: inline-block;
    width: 68px;
    height: 24px;
    margin: 0 5px;
    border-radius: 13px;
    border: 1px solid var(--general);
    background-color: #fff;
    color: var(--general);
    line-height: 24px;
    text-align: center;
}
.small-personal_center .login-box a.login_btn:hover,
.small-personal_center .login-box a.regist_btn:hover {
    color: #fff !important;
    background-color: var(--general);
}
.small-personal_center .vip-list {
    width: 190px;
    border-top: 1px solid #fafafa;
    border-bottom: 1px dashed #e5e5e5;
    display: flex;
    align-items: center;
    justify-content: center;
    padding-top: 10px;
    text-align: center;
}
.small-personal_center .vip-list .lysy {
    width: 62px;
    padding-bottom: 10px;
    color: #666;
}
.small-personal_center .vip-list .lysy p {
    height: 20px;
    line-height: 20px;
    overflow: hidden;
}
.small-personal_center .notice-list {
    padding: 8px 10px;
    overflow: hidden;
}
.small-personal_center .notice-list .item {
    display: flex;
    align-items: center;
    height: 24px;
    overflow: hidden;
    color: #666;
    line-height: 24px;
    white-space: nowrap;
    cursor: pointer;
}
.small-personal_center .notice-list .item:hover {
    color: var(--general);
}
</style>
