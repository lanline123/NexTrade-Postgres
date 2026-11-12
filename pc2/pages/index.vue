<template>
    <div class="main-box">
        <div data-layout-rev="Y3RtMjI4Mmg" class="right-main-box">
            <template v-if="homeStore.previewId > 0">
                <div class="preview-box">
                    <div class="container">
                        {{ $t("预览") }}：&emsp;<span style="">{{ $t("PC端") }}</span>
                    </div>
                </div>
            </template>
            <div :style="pageBgStyle">
                <CommonPageHeader page="index" :showService="false"></CommonPageHeader>
                <!-- 新版：kebab 模块走注册表驱动渲染器 -->
                <div v-if="isNew" class="home-new-warp">
                    <DecorateRenderer :modules="homeStore.moduleList" :critical-module-count="4" />
                </div>
                <!-- 旧版：camelCase 模块保留原硬编码链路 -->
                <template v-else>
                <div v-if="homeStore.moduleList.length > 0" style="margin-bottom: 20px">
                    <template v-for="(item, index) in homeStore.moduleList" :key="index">
                        <HomeBanner v-if="item.type == 'pcBanner' && item.isShow" v-model="item.module" :class="{ container: item.module.bannerStyle == 1 }" />
                        <HomeBannerNew v-if="item.type == 'pcBanner2' && item.isShow" v-model="item.module" />
                    </template>
                </div>
                <div class="container">
                    <div v-if="homeStore.moduleList.length > 0" class="home-warp">
                        <template v-for="(item, index) in homeStore.moduleList" :key="index">
                            <HomeCatWide :moduleIndex="item.moduleIndex"  v-if="item.type === 'pcCatWide' && item.isShow"/>
                            <HomeSeckillNew v-if="isOverseas() && item.type == 'pcSeckill' && item.isShow" v-model="item.module" />
                            <pcGroupon v-if="item.type == 'pcGroupon' && item?.isShow" :module="item.module" />
                            <HomeCategoryNew v-if="item.type == 'pcCategoryNav' && item.isShow" v-model="item.module" />
                            <HomeCategoryNew3 v-if="item.type == 'pcCategoryNav3' && item.isShow" v-model="item.module" />
                            <HomeGroupBanner v-if="item.type == 'pcGroupBanner' && item.isShow" v-model="item.module" />
                            <HomeCoupon v-if="item.type == 'pcCoupon' && item.isShow" v-model="item.module" />
                            <HomeCatProductNormal v-if="item.type == 'pcCatProductNormal' && item.isShow" v-model="item.module" />
                            <HomeCatProductWide v-if="item.type == 'pcCatProductWide' && item.isShow" v-model="item.module" />
                            <HomeCatProductSimple v-if="item.type === 'pcCatProductSimple' && item.isShow" v-model="item.module" :moduleIndex="item.moduleIndex" />
                            <HomeCustomAd1 v-if="item.type == 'pcCustomAd1' && item.isShow" v-model="item.module" />
                            <HomeCustomAd2 v-if="item.type == 'pcCustomAd2' && item.isShow" v-model="item.module" />
                            <HomeImageAd v-if="item.type == 'pcImageAd' && item.isShow" v-model="item.module" />
                            <HomeRecommendProduct
                                v-if="item.type == 'pcRecommendProduct' && item.isShow"
                                v-model="item.module"
                                :decorateId="homeStore.decorateId"
                                :moduleIndex="item.moduleIndex"
                            />
                        </template>
                    </div>
                </div>
                </template>
                <div v-if="homeStore.moduleList.length > 0" class="home-elevator">
                    <HomeElevatorList class="home-elevator-list"></HomeElevatorList>
                </div>
                <div class="placeholder-box"></div>
            </div>
            <CommonPageFooter :showFriendly="true" :mt30="false"></CommonPageFooter>
        </div>
    </div>
</template>
<script lang="ts" setup>
/**
 * O2O商城首页
 * - 支持装修模块动态渲染（Banner、秒杀、优惠券、商品分类等）
 * - 支持预览模式（通过previewId参数）
 * - 模块列表根据后台装修配置动态加载
 * - 包含楼层电梯导航组件
 */
import { getHomeData } from "@/api/home/home";
import { useHomeStore } from "@/store/home";
import { useCommonStore } from "@/store/common";
import { useDecorateRuntimeStore } from "@/store/decorateRuntime";
import { handleDecorateLoadError } from "@/utils/handleDecorateLoadError";
import pcGroupon from "~/pages/activity/pcGroupon/index.vue";
import HomeCatWide from "@/components/home/CatWide.vue"
import DecorateRenderer from "~/decorate/components/DecorateRenderer.vue";
import { isNewHome } from "~/decorate/isNewHome";
import { isOverseas } from "~/utils/util";

const route = useRoute();

const homeStore = useHomeStore();
const decorateRuntimeStore = useDecorateRuntimeStore();

// 新旧分流：homeStyleVersion==='new' 或模块为 kebab → 走注册表渲染器，否则走旧链
const isNew = computed(() => isNewHome(homeStore.pageModule, homeStore.moduleList));

// 新版首页页面背景：让“页面设置”的背景配置在前台生效；未配置/旧版保留 o2o 默认底色 #f2f3f5
const pageBgStyle = computed<Record<string, string>>(() => {
    const style: Record<string, string> = { backgroundColor: "#f2f3f5" };
    if (!isNew.value) return style;
    const pm = (homeStore.pageModule ?? {}) as {
        backgroundColor?: string;
        backgroundImage?: { picUrl?: string };
        backgroundRepeat?: number | string;
        backgroundSize?: number | string;
    };
    if (pm.backgroundColor) style.backgroundColor = pm.backgroundColor;
    const picUrl = pm.backgroundImage?.picUrl;
    if (picUrl) {
        const repeatMap: Record<number, string> = { 1: "no-repeat", 2: "repeat", 3: "repeat-y", 4: "repeat-x", 5: "no-repeat" };
        const sizeMap: Record<number, string> = { 1: "100% auto", 2: "auto 100%", 3: "100% 100%", 4: "100% auto" };
        style.backgroundImage = `url(${picUrl})`;
        style.backgroundPosition = "top center";
        style.backgroundRepeat = repeatMap[Number(pm.backgroundRepeat)] || "no-repeat";
        style.backgroundSize = sizeMap[Number(pm.backgroundSize)] || "100% auto";
    }
    return style;
});

const commonStore = useCommonStore();

useSeo({
    title: computed(() => commonStore.shopTitle || commonStore.shopName),
    description: computed(() => commonStore.shopDesc),
    keywords: computed(() => commonStore.shopKeywords),
    canonical: "/",
    appendSuffix: false
});

// 预览ID 随 query 变化，computed 让 SSR / 客户端都保持响应式
const previewId = computed(() => Number(route.query?.previewId) || 0);

/**
 * 拉取首页装修数据
 * - 用 useAsyncData 包裹，SSR 阶段会真正等到响应再输出 HTML，避免 watch fire-and-forget 导致首屏无模块
 * - previewId 变化自动重取，覆盖原来"清空 + 重拉"的预览逻辑
 * - 命中"模板不存在"错误时跳 404，其余错误吞掉以保持页面可用
 */
const {
    data: homeData,
    error: homeError,
    status: homeStatus
} = await useAsyncData("o2o-pc-home-decorate", () => getHomeData({ previewId: previewId.value }), {
    watch: [previewId]
});

if (homeError.value) {
    await handleDecorateLoadError(homeError.value);
}

// previewId 变化触发 useAsyncData 重取后，homeError 会更新；监听它保证每次重取都能命中 404 处理
watch(homeError, (error) => {
    if (error) {
        handleDecorateLoadError(error);
    }
});

// 数据到位信号：与「已提交到 store」解耦，供演示站风格切换判断何时可以整体落地
// 只认 success：请求失败时不给信号，让风格切换走 waitUntil 的超时兜底，而不是落地一个空首页
watchEffect(() => {
    if (homeStatus.value !== "success") return;
    homeStore.moduleReady = true;
});

// 同步进 store 供旧链路组件（CatWide 等）读取；每次挂载都重写一遍，
// 保证首页不会渲染到其它页面写进 homeStore 的装修数据
watchEffect(() => {
    // 重取期间与失败时都保持旧数据（error 态 homeData 为空，提交下去等于把已渲染内容清成白屏）；
    // 风格切换挂起时等头部装修一起提交，避免"旧风格模块套新风格宽度"的中间态
    if (homeStatus.value !== "success" || decorateRuntimeStore.styleSwitching) return;
    homeStore.previewId = previewId.value;
    homeStore.moduleList = homeData.value?.moduleList ?? [];
    homeStore.decorateId = homeData.value?.decorateId ?? 0;
    homeStore.pageModule = homeData.value?.pageModule ?? null;
    homeStore.moduleLoaded = true;
});

</script>
<style lang="less" scoped>
// 新版首页容器：对齐 NexTrade-Pc（max 1600 / min 1264 / 两侧 38px）
.home-new-warp {
    width: 100%;
    max-width: 1600px;
    min-width: 1264px;
    margin: 0 auto;
    padding: 0 38px;
    box-sizing: border-box;
}
.main-box {
    width: 100%;
    display: flex;
    flex-direction: row;
    justify-content: center;
    position: relative;
    .ChooseStyle-class {
        position: sticky;
        z-index: 100;
        left: 0;
        top: 0;
    }

    .right-main-box {
        width: 100%;
        position: relative;

        .thematic-style {
            position: absolute;
            top: 60px;
            z-index: 20;
            left: 0;
            width: 70px;
            background-color: white;
            cursor: pointer;
            gap: 4px;

            font-size: 16px;
            display: flex;
            align-items: center;
            justify-content: center;

            box-sizing: border-box;
            padding: 0 10px;
            transition: height 0.5s ease; /* 平滑过渡效果 */
            line-height: 1.5;
            height: 120px;
            box-shadow: 0px 4px 8px 0px #dcdcdc;
            border-top-right-radius: 6px;
            border-bottom-right-radius: 6px;
            .div-to-rotate {
                /* 使用transform属性来旋转元素 */
                transform: rotate(-90deg);
                /* 或者使用弧度值 */
                /* transform: rotate(1.5708rad); */
            }
        }

        .home-warp {
            display: flex;
            flex-wrap: wrap;
            justify-content: space-between;
            position: relative;
        }

        .home-elevator {
            position: fixed;
            bottom: 100px;
            right: 0;
            z-index: 1100;
            height: fit-content;
        }

        .skeleton {
            display: flex;
            flex-direction: column;
            gap: 20px;

            .sck1 {
                display: flex;
                margin-top: 10px;
                gap: 10px;

                & > div {
                    display: flex;
                    gap: 10px;
                    flex-direction: column;
                }
            }
        }
        .preview-box {
            padding: 20px;
            background-color: #fff;
            border-bottom: 1px solid #ebeef5;
        }
        .preview-box span {
            position: relative;
            display: inline-block;
            font-size: 14px;
        }
        .preview-box span::after {
            content: "";
            position: absolute;
            top: 25px;
            left: 0;
            width: 100%;
            height: 2px;
            background-color: #155bd4;
        }
    }
}

.home-elevator-list {
    width: 60px;
    z-index: 9999;
}

.not-zh {
    .home-elevator-list {
        width: 68px;
    }
}
.placeholder-box {
    height: 30px;
}
</style>
