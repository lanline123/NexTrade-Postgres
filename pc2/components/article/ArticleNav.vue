<template>
    <div class="lyecs-side">
        <div class="searchboximg">
            <a class="img" href="/article/">
                <img alt="" src="https://demo2.lyecs.com/static/pc/images/help/helptip.png" />
            </a>
        </div>
        <div class="help-center">
            <div class="wrap">
                <div class="aitem" v-for="i in 9" :key="i" :class="{ active: activeSubView === i }" @click="showSubView(i)">
                    <h3 class="organ-title" :class="{ active: activeSubView === i }">{{ $t("最新公告") }}</h3>
                    <div class="organ-main" :class="{ active: activeSubView === i }">
                        <template v-for="(item, index) in newsCatList" :key="index">
                            <div class="item-li" :class="{ active: item.id === query.id }">
                                <a :href="'/article/detail?id=' + item.id">{{ $t(item.title) }}</a>
                            </div>
                        </template>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, shallowRef, onMounted, reactive, toRefs } from "vue";
import { useRoute } from "vue-router";
const route = useRoute();
const { query } = toRefs(route);
const activeSubView = ref(0);
if (query.value.id !== undefined) {
    activeSubView.value = 1;
}
const showSubView = (index) => {
    activeSubView.value = index;
};

const newsCatList = reactive([
    { id: "1", title: "整点抢购，全场秒杀不间断", content: "西兰花" },
    { id: "2", title: "吃喝玩乐团购特约", content: "学IT" },
    { id: "3", title: "本店1周年庆典，全场优惠不断", content: "今天是周一" }
]);
</script>
<style lang="less" scoped>
.lyecs-side {
    width: 190px;
    margin-right: 20px;
}
.searchboximg {
    width: 190px;
    height: 76px;
    background: #fff;
}
.searchboximg img {
    width: 100%;
    height: 100%;
}
.help-center {
    background: #fff;
}
.help-center .wrap {
    margin-top: 15px;
    background: #f7f7f7;
}
.help-center h3 {
    background: url("https://demo2.lyecs.com/static/pc/images/help/h_bj_01.png") no-repeat scroll right 0 transparent;
    cursor: pointer;
    font-size: 12px;
    font-weight: bold;
    height: 35px;
    line-height: 35px;
    margin-top: 1px;
    padding-left: 30px;
}
.help-center h3.active {
    background-position: right -38px;
}
.help-center .organ-main {
    overflow: hidden;
    display: none;
}
.help-center .organ-main.active {
    display: block;
}
.help-center .organ-main .item-li {
    height: 38px;
    line-height: 38px;
}
.help-center .organ-main .item-li a {
    background: none repeat scroll 0 0 #ffffff;
    display: block;
    height: 38px;
    text-indent: 20px;
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
    padding: 0 10px;
}
.help-center .organ-main .item-li a:hover {
    background: none repeat scroll 0 0 #efefef;
    text-decoration: none;
}
.help-center .organ-main .item-li.active a {
    font-weight: 500;
    color: #ff4040;
}
</style>
