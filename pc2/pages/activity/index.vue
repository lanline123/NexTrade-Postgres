<template>
    <CommonPageHeader :shopId="shopId" :showDefNav="false"></CommonPageHeader>
    <div class="module-container">
        <template v-for="(module, index) in modules" :key="module.moduleIndex">
            <template v-if="module.type == 'pcImageAd' && module.isShow">
                <imageAd :module="module.module" />
            </template>
            <template v-if="module.type == 'pcImageSquareAd' && module.isShow">
                <imageSquareAd :module="module.module" />
            </template>
            <template v-if="module.type == 'pcImageHotarea' && module.isShow">
                <imageHotarea :module="module.module" />
            </template>
            <template v-if="module.type == 'pcProduct' && module.isShow">
                <product :module="module.module" :moduleIndex="index" />
            </template>
            <template v-if="module.type == 'pcWhiteLine' && module.isShow">
                <whiteLine :module="module.module" />
            </template>
            <template v-if="module.type == 'pcWhiteBlank' && module.isShow">
                <whiteBlank :module="module.module" />
            </template>
            <template v-if="module.type == 'pcImageNav' && module.isShow">
                <imageNav :module="module.module" />
            </template>
            <template v-if="module.type == 'pcTitleAd' && module.isShow">
                <titleAd :module="module.module" />
            </template>
            <template v-if="module.type == 'pcNotice' && module.isShow">
                <notice :module="module.module" />
            </template>
            <template v-if="module.type == 'pcCoupon' && module.isShow">
                <coupon :module="module.module" :shopId="shopId" />
            </template>
            <!--            秒杀-->
            <template v-if="module.type == 'pcSeckill' && module.isShow">
                <seckill :module="module.module" />
            </template>
            <!--            拼团-->
            <template v-if="module.type == 'pcGroupon' && module.isShow">
                <groupon :module="module.module" />
            </template>
        </template>
    </div>
    <div class="f-bottom">
        <div class="wrapper">
            <div class="copy">
                <p class="copyright">
                    <text>{{ commonStore.shopCompanyTxt }}｜</text>
                    <template v-if="commonStore.businessLicenseEnabled === 1 && commonStore.businessLicenseType?.includes('icp') && commonStore.businessLicense">
                        <text>{{ $t("增值电信业务经营许可证(ICP)") }}：{{ commonStore.businessLicense }}｜</text>
                    </template>
                    <template v-if="commonStore.businessLicenseEnabled === 1 && commonStore.businessLicenseType?.includes('edi') && commonStore.ediLicense">
                        <text>{{ $t("增值电信业务经营许可证(EDI)") }}：{{ commonStore.ediLicense }}｜</text>
                    </template>
                    <NuxtLink class="jj-link" rel="nofollow" target="_blank" :to="commonStore.shopIcpNoUrl">
                        {{ commonStore.shopIcpNo }}
                    </NuxtLink>
                    ｜
                    <NuxtLink class="jj-link" rel="nofollow" target="_blank" v-if="commonStore.shop110No" :to="commonStore.shop110Link"
                        ><img alt="police" :src="police" style="vertical-align: middle; width: 20px; height: 20px" />
                        {{ commonStore.shop110No }}
                    </NuxtLink>
                </p>
                <p class="copyright">
                    <text v-if="commonStore.kefuPhone">{{ $t("客服热线") }}：{{ commonStore.kefuPhone }}</text>
                    <text v-if="commonStore.kefuTime" class="ml10">{{ $t("客服时间") }}：{{ commonStore.kefuTime }}</text>
                    <text v-if="commonStore.kefuAddress" class="ml10">{{ $t("联系地址") }}：{{ commonStore.kefuAddress }}</text>
                </p>
            </div>
            <div class="bot-link"></div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * 活动专题页
 * - 支持通过 id 或 previewId 参数加载不同的装修模板
 * - 动态渲染多种装修组件（图片广告、热区图、商品、优惠券、秒杀、拼团等）
 * - 移动端访问自动跳转到 H5 版本（非 iframe 嵌入时）
 * - 支持后台装修预览模式
 */
import { onMounted, ref, watch, computed } from "vue";
import { getHomeData } from "@/api/home/home";
import type { ModuleListType } from "@/types/home/home.d";
import { useRoute } from "vue-router";
import imageAd from "./pcImageAd/index.vue";
import imageSquareAd from "./pcImageSquareAd/index.vue";
import imageHotarea from "./pcImageHotarea/index.vue";
import product from "./pcProduct/index.vue";
import whiteLine from "./pcWhiteLine/index.vue";
import whiteBlank from "./pcWhiteBlank/index.vue";
import imageNav from "./pcImageNav/index.vue";
import titleAd from "./pcTitleAd/index.vue";
import notice from "./pcNotice/index.vue";
import coupon from "./pcCoupon/index.vue";
import seckill from "./pcSeckill/index.vue";
import groupon from "./pcGroupon/index.vue";
import { useCommonStore } from "@/store/common";
import { handleDecorateLoadError } from "@/utils/handleDecorateLoadError";
import police from "assets/images/police.png";

const commonStore = useCommonStore();
const route = useRoute();
// 支持 id 和 previewId 两种参数
const id = route.query.id ? Number(route.query.id as string) : undefined;
const previewId = route.query.previewId ? Number(route.query.previewId as string) : undefined;

const modules = ref<ModuleListType[]>([]);
const shopId = ref(-1);
const title = ref("");
const description = ref("");
const keywords = ref("");

useSeo({ title, description, keywords, appendPoweredBy: false });

/**
 * 获取装修模块列表
 * - 优先使用 id 参数，其次使用 previewId
 * - 如果返回 bindDecorateId，表示需要跳转到 H5 版本
 */
const getModuleList = async () => {
    try {
        // 根据传入的参数类型构建请求参数
        const params: { id?: number; previewId?: number } = {};
        if (id !== undefined) {
            params.id = id;
        } else if (previewId !== undefined) {
            params.previewId = previewId;
        }
        const result: any = await getHomeData(params);
        if (result.bindDecorateId) {
            await handlerToH5(result.bindDecorateId);
        }
        modules.value = result?.moduleList ?? [];
        title.value = result?.pageModule?.title ?? "";
        description.value = result?.pageModule?.description ?? "";
        keywords.value = result?.pageModule?.keywords ?? "";
    } catch (error) {
        if (await handleDecorateLoadError(error)) {
            return;
        }
        console.log(error, "error");
    }
};

/**
 * 判断当前页面是否在 iframe 中
 * - 用于决定是否自动跳转 H5 版本
 * - iframe 嵌入时不跳转，避免破坏后台预览功能
 */
const isInIframe = () => {
    try {
        return window.self !== window.top;
    } catch {
        return true;
    }
};

/**
 * 移动端自动跳转 H5 版本
 * - 检测 userAgent 判断是否为移动设备
 * - 使用 replace 跳转避免产生历史记录
 */
const handlerToH5 = async (id: number) => {
    if (isInIframe()) {
        return;
    }

    const userAgent = navigator.userAgent;

    const isH5 = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(userAgent);
    if (isH5) {
        let url = `${commonStore.baseInfo.h5Domain ? commonStore.baseInfo.h5Domain : window.location.origin}/pages/activity/index?id=${id}`;
        window.location.replace(url);
    }
};

await getModuleList();
</script>
<style scoped lang="less">
//控制盒子宽度为12000
.module-container {
    width: 1200px;
    margin: 0 auto;
}

.f-bottom {
    box-sizing: border-box;
    border-top: 1px solid #eee;

    .wrapper {
        .copy {
            color: #999;
            line-height: 25px;
            text-shadow: 0 1px 0 #fff;
            text-align: center;
        }
    }

    .bot-link {
        height: 40px;
        text-align: center;
        padding-bottom: 30px;
        width: 1190px;
    }
}
</style>
