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
@import "/assets/css/member/member.v1";
.tab {
    color: #333333;
    font-size: 16px;
    font-weight: 400;
    letter-spacing: 0;
    display: inline-block;
    .tig-font-style {
        font-size: 14px;
    }
}
</style>
