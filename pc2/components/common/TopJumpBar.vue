<template>
    <div class="container topStyle">
        <NuxtLink to="/member" class="right-link">
            <div class="topStyle_left">
                <SvgIcon name="userName" width="20px" height="20px" style="margin-right: 4px"></SvgIcon>
                {{ $t("我的") }}
            </div>
        </NuxtLink>
        <div class="mg_8">
            <img src="/assets/images/common/separator_ico.png" alt="">
        </div>
        <div class="topStyle_right">
            {{ $t(topTitle) }}
        </div>
    </div>
</template>
<script lang="ts" setup>
const props = defineProps({
    tltle: {
        type: String,
        default: false
    }
});
const topTitle = props.tltle;
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";
.topStyle {
    padding-top: 32px;
    padding-bottom: 32px;
    display: flex;
    align-items: flex-end;
    font-size: 14px;
    line-height: 18px;
    .topStyle_left {
        display: flex;
        align-items: center;
        color: #888b94;
    }
    .topStyle_right {
        color: rgb(26, 26, 26);
        font-weight: 600;
        cursor: pointer;
    }
}
.mg_8 {
    width: 19px;
    height: 19px;
    padding: 0 8px;
    img{
        width: 19px;
        height: 19px;
    }
}
</style>
