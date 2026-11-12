<template>
    <!-- <CommonTopBar></CommonTopBar> -->
    <div class="header">
        <div class="top-area container">
            <div class="wrapper">
                <div class="logo">
                    <NuxtLink exact-active-class="''" to="/" class="alv1">
                        <img :src="imageFormat(commonStore.shopLogo)" alt="" />
                    </NuxtLink>
                </div>
                <div class="zs-nav">
                    <div class="nav-slide" :class="`slide-to-${activeTab} ${actActive}`"></div>
                    <ul>
                        <li :class="{ current: activeTab === tab.name }" @mouseenter="tabHover(tab.name)" @mouseleave="tabHoverLeave" v-for="tab in tabs" :key="tab.name">
                            <NuxtLink :to="tab.link">{{ $t(tab.title) }}</NuxtLink>
                        </li>
                    </ul>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useCommonStore } from "@/store/common";
const commonStore: any = useCommonStore();
const router = useRouter();
const actActive = computed(() => {
    switch (router.currentRoute.value.path) {
        case "/join/plan/":
            return "plan-active";
        case "/join/standard/":
            return "standard-active";
        case "/join/process/":
            return "process-active";
        default:
            return "";
    }
});

const activeTab = ref("");
const tabs = ref([
    { name: "", title: "招商首页", link: "/join/" },
    { name: "plan", title: "招商计划", link: "/join/plan/" },
    { name: "standard", title: "招商标准", link: "/join/standard/" },
    { name: "process", title: "入驻流程", link: "/join/process/" }
]);
const tabHover = (value: any) => {
    activeTab.value = value;
};
const tabHoverLeave = () => {
    switch (router.currentRoute.value.path) {
        case "/join/plan/":
            activeTab.value = 'plan'
            break;
        case "/join/standard/":
            activeTab.value = 'standard'
            break;
        case "/join/process/":
            activeTab.value = 'process'
            break;
        default:
            return "";
    }
}
watch(() => router.currentRoute.value.path, (newVal) => {
    switch (newVal) {
        case "/join/plan/":
            activeTab.value = 'plan'
            break;
        case "/join/standard/":
            activeTab.value = 'standard'
            break;
        case "/join/process/":
            activeTab.value = 'process'
            break;
        default:
            return "";
    }
},{ deep: true,immediate:true })
</script>

<style lang="less" scoped>
.header {
    background: #fff;
    border-bottom: 1px solid #ddd;
    position: relative;
}
.top-area {
    position: relative;

    .wrapper {
        display: flex;
        justify-content: space-between;
        align-items: center;

        .logo {
            display: flex;
            align-items: center;

            img {
                width: 190px;
                padding: 10px 0;
            }
        }
    }
}

.zs-nav {
    position: relative;
    .nav-slide {
        background: #fafafa;
        border-top: 3px solid var(--main-bg);
        height: 77px;
        left: 0;
        position: absolute;
        top: 0;
        width: auto;
        overflow: hidden;
        z-index: 1;
        transition: transform 0.3s ease;
        &.slide-to- {
            transform: translateX(132px);
        }
        &.slide-to-plan {
            transform: translateX(132px);
        }
        &.slide-to-standard {
            transform: translateX(264px);
        }
        &.slide-to-process {
            transform: translateX(396px);
        }
        &.plan-active {
            transform: translateX(132px);
        }
        &.standard-active {
            transform: translateX(264px);
        }
        &.process-active {
            transform: translateX(396px);
        }
    }
    ul {
        display: flex;
        position: relative;
        z-index: 2;
        li {
            height: 80px;
            position: relative;

            &::before {
                content: "";
                position: absolute;
                top: 0;
                left: 0;
                right: 0;
                height: 3px;
                background-color: transparent;
                transition: background-color 0.3s ease;
            }

            &:hover::before {
                background-color: var(--general);
            }

            a {
                display: block;
                font: 18px/76px "微软雅黑";
                padding: 0 30px;
            }

            &.current a {
                color: var(--main-bg);
            }
        }
    }
}
</style>
