<template>
    <div v-for="item in menuList" :class="router.currentRoute.value.path == splitStr(item.url) ? 'active' : ''" class="tab">
        <NuxtLink :to="item.url" class="tig-font-style">{{ $t(item.value) }}</NuxtLink>
    </div>
</template>
<script setup lang="ts">
import { useRouter } from "vue-router";
const router = useRouter();
const splitStr = (str: string) => {
    return str.split("?")[0];
};
const props = defineProps({
    menuList: Array<any>
});
</script>
<style scoped lang="less">
@import "/assets/css/member/member";
.tab {
    position: relative;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    height: 32px;
    .tig-font-style {
        font-size: 18px !important;
        color: #505259;
    }
}
</style>
