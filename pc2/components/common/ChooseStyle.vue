<template>
    <div class="div-warp" :class="{ 'is-overseas': isOverseas() }" :style="{ left: open ? '0px' : `-${panelWidth}px` }">
        <div class="min-div">
            <div class="left-div div-s">
                <div class="title">{{ $t("主题风格") }}</div>
                <div class="image-list">
                    <div class="image-text-box" v-for="item in StyleList" :key="item.key">
                        <div class="shi-image" :style="{ borderColor: currentStyle == item.key ? 'var(--general)' : '' }" @click="checkStyle(item)">
                            <el-image class="image" :src="item.image" fit="cover"></el-image>
                            <div class="text">{{ $t(item.label) }}</div>
                        </div>
                    </div>
                </div>
            </div>
            <div class="right-div div-s">
                <div class="title">{{ $t("主题配色") }}</div>
                <div class="color-list">
                    <div
                        v-for="item in ColorList"
                        :style="{ border: item.check ? '1px solid var(--general)' : '1px solid transparent' }"
                        class="color-card"
                        @click="selectItem(item.themeId)"
                    >
                        <div :style="{ background: item['--main-bg'] }"></div>
                        <div :style="{ background: item['--vice-bg'] }"></div>
                    </div>
                </div>
                <div class="tips">{{ $t("提示：风格切换仅为演示站提供") }}</div>
            </div>
        </div>
        <div @click="checkStyleOpen" class="thematic-style">
            <div class="text">{{ $t("主题风格") }} <i :class="open ? 'icon-shouqi' : 'icon-zhankai'" class="iconfont-pc div-to-rotate"></i></div>
        </div>
    </div>
</template>
<script setup lang="ts">
import oldImage from "~/assets/images/index/old.jpg";
import newImage from "~/assets/images/index/new.jpg";
import classicImage from "~/assets/images/index/classic.jpg";
import overseasImage from "~/assets/images/index/overseas.jpg";
import { useCommonStore } from "~/store/common";
import { useThemeStore } from "~/store/theme";
import { useRoute, useRouter } from "vue-router";
import { ref } from "vue";
import { useHomeStore } from "@/store/home";
import { useDecorateRuntimeStore } from "@/store/decorateRuntime";
import { isOverseas } from "@/utils/util";
const router = useRouter();
const themeStore = useThemeStore();
const commonStore = useCommonStore();
const homeStore = useHomeStore();
const decorateRuntimeStore = useDecorateRuntimeStore();

await commonStore.loadNav();

const ChooseStyleOpen = useCookie<any>("ChooseStyleOpen");

const open = ref(ChooseStyleOpen.value ? true : false);
/** 跨境四套风格需要更宽侧栏，收起位移与面板宽度对齐 */
const panelWidth = isOverseas() ? 720 : 500;
const checkStyleOpen = () => {
    open.value = !open.value;
    ChooseStyleOpen.value = open.value;
};

interface PageStyle {
    key: string;
    label: string;
    image: string;
    /** 0 = 走后台默认装修（不带 previewId） */
    previewId: number;
    headerStyle: number;
}

/** 跨境包默认现代风格；其它包默认现代风格 */
const StyleList: PageStyle[] = isOverseas()
    ? [
          { key: "overseas", label: "跨境风格", image: overseasImage, previewId: 10000, headerStyle: commonStore.defaultHeaderStyle },
          { key: "modern", label: "现代风格", image: classicImage, previewId: 0, headerStyle: commonStore.defaultHeaderStyle },
          { key: "classic", label: "经典风格", image: oldImage, previewId: 9, headerStyle: 1 },
          { key: "simple", label: "简约风格", image: newImage, previewId: 91, headerStyle: 2 }
      ]
    : [
          { key: "modern", label: "现代风格", image: classicImage, previewId: 0, headerStyle: commonStore.defaultHeaderStyle },
          { key: "classic", label: "经典风格", image: oldImage, previewId: 81, headerStyle: 1 },
          { key: "simple", label: "简约风格", image: newImage, previewId: 91, headerStyle: 2 }
      ];

const route = useRoute();
const currentStyle = ref(StyleList.find((item) => item.previewId === (Number(route.query.previewId) || 0))?.key ?? StyleList[0].key);

/** 切换进行中：防连点，避免多套装修数据交叉提交 */
const switching = ref(false);

/** 等待条件成立，带超时兜底，避免装修接口异常时切换永远挂起 */
const waitUntil = (predicate: () => boolean, timeout = 6000) =>
    new Promise<void>((resolve) => {
        if (predicate()) {
            resolve();
            return;
        }
        const timer = setTimeout(() => {
            stop();
            resolve();
        }, timeout);
        const stop = watch(predicate, (ok) => {
            if (!ok) return;
            clearTimeout(timer);
            stop();
            resolve();
        });
    });

const checkStyle = async (item: PageStyle) => {
    if (switching.value) return;
    // 目标就是当前地址时不折腾，否则等不到首页装修重取会白等一个超时
    const onTargetRoute = route.path === "/" && (Number(route.query.previewId) || 0) === item.previewId;
    if (onTargetRoute) {
        currentStyle.value = item.key;
        return;
    }

    switching.value = true;
    currentStyle.value = item.key;
    homeStore.moduleLoaded = false;
    homeStore.moduleReady = false;
    // 挂起首页模块提交，切换期间页面维持当前风格不动
    decorateRuntimeStore.setStyleSwitching(true);

    const prevPageConfig = { ...commonStore.decoratePageConfig };
    let nextPageConfig = { ...prevPageConfig, headerStyle: item.headerStyle };
    try {
        const targetPreviewId = item.previewId > 0 ? item.previewId : undefined;
        if (!targetPreviewId) {
            // 默认风格要回到后台基础配置；loadBaseConfig 会直接改 store，先还原、等提交时再一起切
            await commonStore.loadBaseConfig();
            nextPageConfig = { ...commonStore.decoratePageConfig, headerStyle: item.headerStyle };
            commonStore.decoratePageConfig = prevPageConfig;
        }
        // 头部装修（全站宽度体系的信号源）预取但先不提交，与首页装修并行拉取
        const [commitPcHeader] = await Promise.all([
            decorateRuntimeStore.prepareGlobalPcHeaderConfig(targetPreviewId),
            router.replace(targetPreviewId ? `/?previewId=${targetPreviewId}` : "/")
        ]);
        await waitUntil(() => homeStore.moduleReady);
        // 三份状态在同一同步块提交 → 头部、全站宽度、首页模块同帧生效，无中间态
        commonStore.decoratePageConfig = nextPageConfig;
        commitPcHeader();
    } finally {
        decorateRuntimeStore.setStyleSwitching(false);
        switching.value = false;
    }
};

const formState = ref({
    themeId: 0
});

const ColorList: any = ref([
    {
        check: false,
        themeId: 1,
        "--general": "#4a90e2",
        "--main-bg": "#4a90e2",
        "--main-bg-gradient": "#4a90e2",
        "--main-text": "#ffffff",
        "--vice-bg": "#D6E9FC",
        "--vice-text": "#0080FF",
        "--icon": "#0080FF",
        "--price": "#0080FF",
        "--tag-text": "#0080FF",
        "--tag-bg": "#E5F2FF",
        "--primary-light-3": "#80b1eb",
        "--primary-light-5": "#a5c8f1",
        "--primary-light-7": "#c9def6",
        "--primary-light-8": "#dbe9f9",
        "--primary-light-9": "#edf4fc",
        "--primary-dark-2": "#3b73b5",
        "--ump-main-bg": "#4a90e2",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#D6E9FC",
        "--ump-vice-text": "#0080FF",
        "--ump-icon": "#0080FF",
        "--ump-price": "#0080FF",
        "--ump-tag-text": "#0080FF",
        "--ump-tag-bg": "#E5F2FF",
        "--ump-coupon-bg": "#F2F8FF",
        "--ump-border": "#CCE4FF",
        "--ump-start-bg": "#00B8FF",
        "--ump-end-bg": "#0078EF"
    },
    {
        check: false,
        themeId: 2,
        "--general": "#ff4444",
        "--main-bg": "#ff4444",
        "--main-bg-gradient": "#ff4444",
        "--main-text": "#ffffff",
        "--vice-bg": "#ff8855",
        "--vice-text": "#fff",
        "--icon": "#FC0000",
        "--price": "#FC0000",
        "--tag-text": "#CF0000",
        "--tag-bg": "#FFF2F2",
        "--primary-light-3": "#ff7c7c",
        "--primary-light-5": "#ffa2a2",
        "--primary-light-7": "#ffc7c7",
        "--primary-light-8": "#ffdada",
        "--primary-light-9": "#ffecec",
        "--primary-dark-2": "#cc3636",
        "--ump-main-bg": "#ff4444",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#ff8855",
        "--ump-vice-text": "#fff",
        "--ump-icon": "#FC0000",
        "--ump-price": "#FC0000",
        "--ump-tag-text": "#CF0000",
        "--ump-tag-bg": "#FFF2F2",
        "--ump-coupon-bg": "#FFF2F2",
        "--ump-border": "#FFCCCC",
        "--ump-start-bg": "#F86F30",
        "--ump-end-bg": "#E91717"
    },
    {
        check: false,
        themeId: 3,
        "--general": "#ff5e15",
        "--main-bg": "#ff5e15",
        "--main-bg-gradient": "#ff5e15",
        "--main-text": "#ffffff",
        "--vice-bg": "#FF9300",
        "--vice-text": "#ffffff",
        "--icon": "#ff5e15",
        "--price": "#ff5e15",
        "--tag-text": "#ff5e15",
        "--tag-bg": "#FFEDE6",
        "--primary-light-3": "#ff8e5b",
        "--primary-light-5": "#ffaf8a",
        "--primary-light-7": "#ffcfb9",
        "--primary-light-8": "#ffdfd0",
        "--primary-light-9": "#ffefe8",
        "--primary-dark-2": "#cc4b11",
        "--ump-main-bg": "#ff5e15",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#FF9300",
        "--ump-vice-text": "#ffffff",
        "--ump-icon": "#ff5e15",
        "--ump-price": "#ff5e15",
        "--ump-tag-text": "#ff5e15",
        "--ump-tag-bg": "#FFEDE6",
        "--ump-coupon-bg": "#FFF6F2",
        "--ump-border": "#FFDCCC",
        "--ump-start-bg": "#FF8C20",
        "--ump-end-bg": "#FF4300"
    },
    {
        check: false,
        themeId: 4,
        "--general": "#ff547b",
        "--main-bg": "#ff547b",
        "--main-bg-gradient": "#ff547b",
        "--main-text": "#ffffff",
        "--vice-bg": "#FFE6E8",
        "--vice-text": "#ff547b",
        "--icon": "#ff547b",
        "--price": "#ff547b",
        "--tag-text": "#ff547b",
        "--tag-bg": "#FFF2F5",
        "--primary-light-3": "#ff87a3",
        "--primary-light-5": "#ffaabd",
        "--primary-light-7": "#ffccd7",
        "--primary-light-8": "#ffdde5",
        "--primary-light-9": "#ffeef2",
        "--primary-dark-2": "#cc4362",
        "--ump-main-bg": "#ff547b",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#FFE6E8",
        "--ump-vice-text": "#ff547b",
        "--ump-icon": "#ff547b",
        "--ump-price": "#ff547b",
        "--ump-tag-text": "#ff547b",
        "--ump-tag-bg": "#FFF2F5",
        "--ump-coupon-bg": "#FFF2F5",
        "--ump-border": "#FFCCD8",
        "--ump-start-bg": "#FF44A3",
        "--ump-end-bg": "#FF4063"
    },
    {
        check: false,
        themeId: 5,
        "--general": "#FF4444",
        "--main-bg": "#FF4444",
        "--main-bg-gradient": "#FF4444",
        "--main-text": "#ffffff",
        "--vice-bg": "#555555",
        "--vice-text": "#ffffff",
        "--icon": "#FC0000",
        "--price": "#FC0000",
        "--tag-text": "#CF0000",
        "--tag-bg": "#FFF2F2",
        "--primary-light-3": "#ff7c7c",
        "--primary-light-5": "#ffa2a2",
        "--primary-light-7": "#ffc7c7",
        "--primary-light-8": "#ffdada",
        "--primary-light-9": "#ffecec",
        "--primary-dark-2": "#cc3636",
        "--ump-main-bg": "#FF4444",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#555555",
        "--ump-vice-text": "#ffffff",
        "--ump-icon": "#FC0000",
        "--ump-price": "#FC0000",
        "--ump-tag-text": "#CF0000",
        "--ump-tag-bg": "#FFF2F2",
        "--ump-coupon-bg": "#FFF2F2",
        "--ump-border": "#FFCCCC",
        "--ump-start-bg": "#FF8544",
        "--ump-end-bg": "#FF2E2E"
    },
    {
        check: false,
        themeId: 6,
        "--general": "#FCC600",
        "--main-bg": "#FCC600",
        "--main-bg-gradient": "#FCC600",
        "--main-text": "#ffffff",
        "--vice-bg": "#1D262E",
        "--vice-text": "#ffffff",
        "--icon": "#FCC600",
        "--price": "#FCC600",
        "--tag-text": "#FCC600",
        "--tag-bg": "#FFF4CD",
        "--primary-light-3": "#fdd74d",
        "--primary-light-5": "#fee380",
        "--primary-light-7": "#feeeb3",
        "--primary-light-8": "#fef4cc",
        "--primary-light-9": "#fff9e6",
        "--primary-dark-2": "#ca9e00",
        "--ump-main-bg": "#FCC600",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#1D262E",
        "--ump-vice-text": "#ffffff",
        "--ump-icon": "#FCC600",
        "--ump-price": "#FCC600",
        "--ump-tag-text": "#FCC600",
        "--ump-tag-bg": "#FFF4CD",
        "--ump-coupon-bg": "#FFFCF2",
        "--ump-border": "#FFF3CC",
        "--ump-start-bg": "#FF5B00",
        "--ump-end-bg": "#FFBF25"
    },
    {
        check: false,
        themeId: 7,
        "--general": "#65c4aa",
        "--main-bg": "#65c4aa",
        "--main-bg-gradient": "#65c4aa",
        "--main-text": "#ffffff",
        "--vice-bg": "#D9F6EF",
        "--vice-text": "#65c4aa",
        "--icon": "#65c4aa",
        "--price": "#65c4aa",
        "--tag-text": "#65c4aa",
        "--tag-bg": "#F2FFFC",
        "--primary-light-3": "#93d6c4",
        "--primary-light-5": "#b2e2d5",
        "--primary-light-7": "#d1ede6",
        "--primary-light-8": "#e0f3ee",
        "--primary-light-9": "#f0f9f7",
        "--primary-dark-2": "#519d88",
        "--ump-main-bg": "#65c4aa",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#D9F6EF",
        "--ump-vice-text": "#65c4aa",
        "--ump-icon": "#65c4aa",
        "--ump-price": "#65c4aa",
        "--ump-tag-text": "#65c4aa",
        "--ump-tag-bg": "#F2FFFC",
        "--ump-coupon-bg": "#FFFCF2",
        "--ump-border": "#CCFFF3",
        "--ump-start-bg": "#00D1C9",
        "--ump-end-bg": "#00BB87"
    },
    {
        check: false,
        themeId: 8,
        "--general": "#09bb07",
        "--main-bg": "#09bb07",
        "--main-bg-gradient": "#09bb07",
        "--main-text": "#ffffff",
        "--vice-bg": "#383838",
        "--vice-text": "#ffffff",
        "--icon": "#09BB07",
        "--price": "#09BB07",
        "--tag-text": "#09BB07",
        "--tag-bg": "#E6F8E6",
        "--primary-light-3": "#53cf51",
        "--primary-light-5": "#84dd83",
        "--primary-light-7": "#b5ebb5",
        "--primary-light-8": "#cef1cd",
        "--primary-light-9": "#e6f8e6",
        "--primary-dark-2": "#079606",
        "--ump-main-bg": "#09bb07",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#383838",
        "--ump-vice-text": "#ffffff",
        "--ump-icon": "#09BB07",
        "--ump-price": "#09BB07",
        "--ump-tag-text": "#09BB07",
        "--ump-tag-bg": "#E6F8E6",
        "--ump-coupon-bg": "#F2FFF2",
        "--ump-border": "#CCFFCC",
        "--ump-start-bg": "#10C80E",
        "--ump-end-bg": "#00A630"
    },
    {
        check: false,
        themeId: 9,
        "--general": "#63be72",
        "--main-bg": "#63be72",
        "--main-bg-gradient": "#63be72",
        "--main-text": "#ffffff",
        "--vice-bg": "#E1F4E3",
        "--vice-text": "#6CBE72",
        "--icon": "#6CBE72",
        "--price": "#6CBE72",
        "--tag-text": "#6CBE72",
        "--tag-bg": "#F0F8F0",
        "--primary-light-3": "#92d29c",
        "--primary-light-5": "#b1dfb9",
        "--primary-light-7": "#d0ecd5",
        "--primary-light-8": "#e0f2e3",
        "--primary-light-9": "#eff9f1",
        "--primary-dark-2": "#4f985b",
        "--ump-main-bg": "#63be72",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#E1F4E3",
        "--ump-vice-text": "#6CBE72",
        "--ump-icon": "#6CBE72",
        "--ump-price": "#6CBE72",
        "--ump-tag-text": "#6CBE72",
        "--ump-tag-bg": "#F0F8F0",
        "--ump-coupon-bg": "#F2FFF4",
        "--ump-border": "#CCFFD4",
        "--ump-start-bg": "#57D8AE",
        "--ump-end-bg": "#4CBA54"
    },
    {
        check: false,
        themeId: 10,
        "--general": "#c3a769",
        "--main-bg": "#c3a769",
        "--main-bg-gradient": "#c3a769",
        "--main-text": "#ffffff",
        "--vice-bg": "#F3EEE1",
        "--vice-text": "#C3A769",
        "--icon": "#C3A769",
        "--price": "#C3A769",
        "--tag-text": "#C3A769",
        "--tag-bg": "#F9F6F0",
        "--primary-light-3": "#d5c196",
        "--primary-light-5": "#e1d3b4",
        "--primary-light-7": "#ede5d2",
        "--primary-light-8": "#f3ede1",
        "--primary-light-9": "#f9f6f0",
        "--primary-dark-2": "#9c8654",
        "--ump-main-bg": "#c3a769",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#F3EEE1",
        "--ump-vice-text": "#C3A769",
        "--ump-icon": "#C3A769",
        "--ump-price": "#C3A769",
        "--ump-tag-text": "#C3A769",
        "--ump-tag-bg": "#F9F6F0",
        "--ump-coupon-bg": "#FFFBF2",
        "--ump-border": "#FFEFCC",
        "--ump-start-bg": "#B18836",
        "--ump-end-bg": "#DEB155"
    },
    {
        check: false,
        themeId: 11,
        "--general": "#2f2f34",
        "--main-bg": "#2f2f34",
        "--main-bg-gradient": "#2f2f34",
        "--main-text": "#ffffff",
        "--vice-bg": "#EBECF2",
        "--vice-text": "#2F2F34",
        "--icon": "#2F2F34",
        "--price": "#2F2F34",
        "--tag-text": "#2F2F34",
        "--tag-bg": "#EAEAEA",
        "--primary-light-3": "#6d6d71",
        "--primary-light-5": "#97979a",
        "--primary-light-7": "#c1c1c2",
        "--primary-light-8": "#d5d5d6",
        "--primary-light-9": "#eaeaeb",
        "--primary-dark-2": "#26262a",
        "--ump-main-bg": "#2f2f34",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#EBECF2",
        "--ump-vice-text": "#2F2F34",
        "--ump-icon": "#2F2F34",
        "--ump-price": "#2F2F34",
        "--ump-tag-text": "#2F2F34",
        "--ump-tag-bg": "#EAEAEA",
        "--ump-coupon-bg": "#F2F2FF",
        "--ump-border": "#CCCCFF",
        "--ump-start-bg": "#383D46",
        "--ump-end-bg": "#2E333D"
    },
    {
        check: false,
        themeId: 12,
        "--general": "#884cff",
        "--main-bg": "#884cff",
        "--main-bg-gradient": "#884cff",
        "--main-text": "#ffffff",
        "--vice-bg": "#EFE6FF",
        "--vice-text": "#884cff",
        "--icon": "#884cff",
        "--price": "#884cff",
        "--tag-text": "#884cff",
        "--tag-bg": "#F3EDFF",
        "--primary-light-3": "#ac82ff",
        "--primary-light-5": "#c4a6ff",
        "--primary-light-7": "#dbc9ff",
        "--primary-light-8": "#e7dbff",
        "--primary-light-9": "#f3edff",
        "--primary-dark-2": "#6d3dcc",
        "--ump-main-bg": "#884cff",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "#EFE6FF",
        "--ump-vice-text": "#884cff",
        "--ump-icon": "#884cff",
        "--ump-price": "#884cff",
        "--ump-tag-text": "#884cff",
        "--ump-tag-bg": "#F3EDFF",
        "--ump-coupon-bg": "#F7F2FF",
        "--ump-border": "#E0CCFF",
        "--ump-start-bg": "#CF4BF6",
        "--ump-end-bg": "#5F35EB"
    },
    {
        check: false,
        themeId: 13,
        "--general": "#EE0A24",
        "--main-bg": "#EE0A24",
        "--main-bg-gradient": "#EE0A24",
        "--main-text": "#ffffff",
        "--vice-bg": "#FF9518FF",
        "--vice-text": "#ffffff",
        "--icon": "#EE0A24",
        "--price": "#EE0A24",
        "--tag-text": "#EE0A24",
        "--tag-bg": "#FDE6E9",
        "--primary-light-3": "#f35466",
        "--primary-light-5": "#f78592",
        "--primary-light-7": "#fab6bd",
        "--primary-light-8": "#fcced3",
        "--primary-light-9": "#fde7e9",
        "--primary-dark-2": "#be081d",
        "--ump-main-bg": "#EE0A24",
        "--ump-main-text": "#ffffff",
        "--ump-vice-bg": "linear-gradient(to right, #FFD01E, #FF8917)",
        "--ump-vice-text": "#ffffff",
        "--ump-icon": "#EE0A24",
        "--ump-price": "#EE0A24",
        "--ump-tag-text": "#EE0A24",
        "--ump-tag-bg": "#FDE6E9",
        "--ump-coupon-bg": "#FFF2F4",
        "--ump-border": "#FCCED3",
        "--ump-start-bg": "#FF6034",
        "--ump-end-bg": "#EE0A24"
    }
]);

const themeColor = useCookie<any>("themeColor");

const selectItem = (themeId: number) => {
    ColorList.value.forEach((item: any) => {
        if (item.themeId === themeId) {
            Object.assign(formState.value, item);
            item.check = true;
            themeStore.themeId = item.themeId;
            themeStore.themeColor = item;
            themeColor.value = item;
        } else {
            item.check = false;
        }
    });
};

onBeforeMount(() => {
    ColorList.value.forEach((item: any) => {
        if (item.themeId === themeStore.themeId) {
            item.check = true;
        } else {
            item.check = false;
        }
    });
});
</script>
<style scoped lang="less">
.div-warp {
    --panel-width: 500px;
    display: flex;
    height: 100vh;
    box-shadow: 0 4px 8px 0 #dcdcdc;
    width: var(--panel-width);
    position: fixed;
    z-index: 2000;
    left: 0;
    top: 0;
    transition: left 0.5s ease;

    &.is-overseas {
        --panel-width: 720px;

        .min-div {
            .left-div .image-list {
                gap: 12px;

                .image-text-box {
                    min-width: 150px;

                    .shi-image .image {
                        height: 220px;
                    }
                }
            }
        }
    }

    .thematic-style {
        background-color: var(--main-bg);
        color: var(--main-text);
        position: absolute;
        top: calc(50% - 60px);
        z-index: 200;
        left: var(--panel-width);
        width: 70px;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;

        transition: width 0.5s ease;
        box-sizing: border-box;
        padding: 0 10px;
        line-height: 1.5;
        height: 120px;
        box-shadow: 6px 4px 8px 0px #00000021;
        border-top-right-radius: 6px;
        border-bottom-right-radius: 6px;

        .text {
            width: 50px;
            font-size: 14px;
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100%;
            .div-to-rotate {
                transform: rotate(-90deg);
            }
        }
    }

    .thematic-style:hover {
        width: 100px;
    }
    .min-div {
        width: var(--panel-width);
        z-index: 130;
        background-color: white;
        display: flex;
        align-items: center;
        transition: left 0.5s ease; /* 平滑过渡效果 */
        overflow: hidden; /* 隐藏溢出内容 */
        height: 100vh;
        box-sizing: border-box;
        flex-direction: column;
        position: fixed;
        box-shadow: 0px 4px 8px 0px #00000021;

        .title {
            height: 53rpx;
            font-family:
                PingFangSC,
                PingFang SC;
            font-weight: 500;
            font-size: 20px;
            color: #333333;
            line-height: 53rpx;
            text-align: left;
            font-style: normal;
        }

        .div-s {
            display: flex;
            flex-direction: column;
            box-sizing: border-box;
            padding: 20px;
        }

        .left-div {
            width: var(--panel-width);
            .image-list {
                display: flex;
                flex-direction: row;
                flex-wrap: wrap;
                box-sizing: border-box;
                gap: 10px;
                padding: 10px 0;

                .image-text-box {
                    flex: 1;
                    min-width: 100px;
                    display: flex;
                    flex-direction: column;
                    align-items: flex-start;
                    justify-content: center;

                    .shi-image {
                        border-radius: 6px;
                        padding: 10px;
                        box-sizing: border-box;
                        border: 2px solid #f5f5f5;
                        cursor: pointer;
                        width: 100%;

                        .image {
                            width: 100%;
                            height: 180px;
                            border-radius: 4px;
                        }

                        .text {
                            margin-top: 6px;
                            text-align: center;
                            font-size: 14px;
                        }
                    }
                }
            }
        }

        .right-div {
            width: var(--panel-width);
            .color-list {
                display: flex;
                flex-wrap: wrap;
                gap: 10px;
                margin-top: 10px;

                .color-card {
                    display: flex;
                    box-sizing: border-box;
                    padding: 2px;
                    cursor: pointer;

                    & > div {
                        height: 24px;
                        width: 24px;
                        flex: 1;
                    }
                }
            }
            .tips {
                margin-top: 10px;
                color: #9a9a9a;
                font-size: 14px;
            }
        }
    }
}
</style>
