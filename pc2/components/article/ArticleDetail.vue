<template>
    <div class="lyecs-main">
        <div class="article box">
            <div class="bd">
                <div class="article_title">
                    <h1>{{ $t("注册新用户") }}{{ query.id }}</h1>
                </div>
                <div class="content"></div>
                <p class="empty">{{ $t("没有内容") }}{{ query.id }}</p>
                <p class="nearby">
                    <a href="#" class="prev">«{{ $t("上一篇： 注册新用户") }}</a>
                    <a href="#" class="next">{{ $t("下一篇：订购流程") }}»</a>
                </p>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import router from "@/router";
import { ref, toRefs, shallowRef, onMounted } from "vue";
import { useRoute } from "vue-router";
const route = useRoute();
const { query } = toRefs(route);
</script>
<style lang="less" scoped>
.lyecs-main {
    flex: 1;
    width: 980px;
}
.box {
    margin-bottom: 10px;
    background: #fff;
}
.box .bd {
    border: 1px solid #eeeeee;
    border-top: 0;
    height: 100%;
    clear: both;
    padding: 15px;
    display: block;
}
.article .bd {
    padding: 9px 19px;
    border: 0;
}
.article .article_title h1 {
    font-size: 20px;
    font-weight: bold;
    height: 40px;
    line-height: 40px;
    display: block;
}
.empty {
    display: block;
    padding: 50px 0;
    color: #999;
    font-size: 1.5em;
    font-weight: normal;
    text-align: center;
}
.article .nearby {
    margin-top: 10px;
    padding: 10px;
    line-height: 18px;
    display: flex;
    justify-content: space-between;
}
</style>
