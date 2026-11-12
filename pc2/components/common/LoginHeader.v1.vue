<template>
    <div class="header">
        <div class="wrapper">
            <div class="logo">
                <NuxtLink to="/" class="alv1">
                    <img :src="imageFormat(commonStore.shopLogo)" alt="" />
                </NuxtLink>
            </div>
            <div v-if="showLogin" class="info">
                {{ $t("您好，欢迎光临{0}！", [commonStore.shopName]) }}
                <template v-if="type == 'register'">
                    <NuxtLink class="font-color" to="/member/login">{{ $t("已有账号？去登录！") }}</NuxtLink>
                </template>
                <template v-else-if="commonStore.shopRegClosed === 0">
                    <NuxtLink class="font-color" to="/member/register/">{{ $t("没有账号？注册新用户！") }}</NuxtLink>
                </template>
            </div>
        </div>
    </div>
</template>
<script setup lang="ts">
import { useCommonStore } from "~/store/common";
const props = defineProps({
    showLogin: {
        type: Boolean,
        default: true
    },
    type: {
        type: String,
        default: "login"
    }
});
const commonStore: any = useCommonStore();
commonStore.loadNav();
</script>
<style scoped lang="less">
.header {
    background: #fff;
    border-bottom: 1px solid #ddd;
    display: flex;
    height: 90px;
    justify-content: center;
    box-sizing: border-box;

    .wrapper {
        width: 1190px;
        display: flex;
        justify-content: space-between;
        box-sizing: border-box;
        padding: 15px 10px;

        .logo {
            display: flex;
            align-items: center;

            img {
                width: 190px;
            }
        }

        .info {
            min-width: 300px;
            color: #666666;
            gap: 10px;
            display: flex;

            .help {
                text-decoration: underline;
                color: #666666;
            }
        }
    }
}
</style>
