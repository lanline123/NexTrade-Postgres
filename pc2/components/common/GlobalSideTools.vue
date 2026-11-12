<template>
    <client-only>
        <div class="global-side-tools side-tools-ZHR6ZTIyODI">
            <CartFloatEntry v-if="showCartEntry" variant="card" />
            <div v-show="showBackTop" class="side-card back-top" @click="backToTop">
                <i class="iconfont-pc icon-shangjiantou"></i>
                <span class="label">{{ $t("顶部") }}</span>
            </div>
        </div>
    </client-only>
</template>

<script lang="ts" setup>
import { computed, onMounted, onBeforeUnmount, ref } from "vue";
import { useRoute } from "vue-router";
import CartFloatEntry from "~/components/common/CartFloatEntry.vue";

const route = useRoute();
// 购物车及登录注册页无需展示购物车入口，仅保留回到顶部
const showCartEntry = computed(() => {
    const path = (route.path || "/").replace(/\/+/g, "/") || "/";
    const hiddenRoutes = ["/cart", "/member/login", "/member/register"];
    return !hiddenRoutes.some((prefix) => path === prefix || path.startsWith(`${prefix}/`));
});

const showBackTop = ref(false);

const onScroll = () => {
    showBackTop.value = (window.scrollY || document.documentElement.scrollTop) > 400;
};

const backToTop = () => {
    window.scrollTo({ top: 0, behavior: "smooth" });
};

onMounted(() => {
    window.addEventListener("scroll", onScroll, { passive: true });
    onScroll();
});

onBeforeUnmount(() => {
    window.removeEventListener("scroll", onScroll);
});
</script>

<style lang="less" scoped>
.global-side-tools {
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    position: fixed;
    right: 2vw;
    bottom: 100px;
    z-index: 99;
    gap: 8px;
}

.side-card {
    width: 68px;
    height: 68px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    font-size: 12px;
    color: #666666;
    background-color: #fff;
    border-radius: 6px;
    box-shadow: 0 4px 10px 0 rgba(0, 0, 0, 0.08);
    cursor: pointer;

    i {
        font-size: 24px;
    }

    &:hover {
        color: var(--general);
        transition: 0.3s;
    }
}
</style>
