<template>
    <div class="lyecs-side">
        <div class="news-side-content">
            <div class="hot_news">
                <h2 class="yin_rh">{{ $t("文章分类") }}</h2>
                <div class="yin_na">
                    <p>
                        <a href="/news/detail" :title="$t('帮助中心')">- {{ $t("帮助中心") }}</a>
                    </p>
                    <div class="left_list">
                        <p @click="showNewsCat1 = !showNewsCat1">
                            <span class="news_title">- <a href="/news/detail">{{ $t("新闻资讯") }}</a> </span>
                            <span class="left_more" :class="{ left_ma: showNewsCat1 == true }"></span>
                        </p>
                        <div class="Left_cx" v-if="showNewsCat1">
                            <a class="link" :title="$t('行业资讯')" href="/news/detail">{{ $t("行业资讯") }}</a>
                        </div>
                    </div>
                    <div class="left_list">
                        <p @click="showNewsCat2 = !showNewsCat2">
                            <span class="news_title">- <a href="/news/detail">{{ $t("最新公告") }}</a> </span>
                            <span class="left_more" :class="{ left_ma: showNewsCat2 == true }"></span>
                        </p>
                        <div class="Left_cx" v-if="showNewsCat2">
                            <a class="link" :title="$t('本店1周年庆典，全场优惠不断')" href="/news/detail">{{ $t("本店1周年庆典，全场优惠不断") }}</a>
                            <a class="link" :title="$t('吃喝玩乐团购特约')" href="/news/detail">{{ $t("吃喝玩乐团购特约") }}</a>
                            <a class="link" :title="$t('整点抢购，全场秒杀不间断')" href="/news/detail">{{ $t("整点抢购，全场秒杀不间断") }}</a>
                        </div>
                    </div>
                    <div class="left_list">
                        <p @click="showNewsCat3 = !showNewsCat3">
                            <span class="news_title">- <a href="/news/detail">{{ $t("客服中心") }}</a> </span>
                            <span class="left_more" :class="{ left_ma: showNewsCat3 == true }"></span>
                        </p>
                        <div class="Left_cx flex" v-if="showNewsCat3">
                            <a class="link" :title="$t('加盟合作')" href="/news/detail">{{ $t("加盟合作") }}</a>
                            <a class="link" :title="$t('投诉与建议')" href="/news/detail">{{ $t("投诉与建议") }}</a>
                            <a class="link" :title="$t('会员特权')" href="/news/detail">{{ $t("会员特权") }}</a>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, shallowRef, onMounted } from "vue";

const showNewsCat1 = ref(true);
const showNewsCat2 = ref(true);
const showNewsCat3 = ref(true);
</script>
<style lang="less" scoped>
.lyecs-side {
    width: 300px;
    margin-left: 20px;
}
.news-side-content {
    padding: 30px 40px;
    background: #fff;
    position: relative;
}
.yin_rh {
    color: #333;
    display: block;
    font-size: 14px;
    font-weight: 700;
    height: 24px;
    line-height: 24px;
    overflow: hidden;
}
.yin_na {
    overflow: hidden;
    overflow-wrap: break-word;
    padding-top: 3px;
    margin-bottom: 15px;
}
.yin_na p {
    height: 30px;
    line-height: 30px;
    overflow: hidden;
    display: flex;
    justify-content: space-between;
}
.yin_na p a,
.news_title {
    color: #888;
    font-size: 14px;
}
.yin_na .left_list {
    overflow: hidden;
}
.Left_cx {
    max-height: 195px;
    overflow: hidden;
    margin-bottom: 10px;
    padding-left: 15px;
}
.Left_cx a {
    color: #bbb;
    font-size: 12px;
}
.yin_na .link,
.news_title {
    display: block;
    line-height: 14px;
    margin-top: 10px;
    padding-right: 10px;
    white-space: nowrap;
}
.yin_na p .left_more {
    background: rgba(0, 0, 0, 0) url("http://localhost/LYECS/LYECS-dev/public/static/pc/images/index/click.jpg") no-repeat scroll 15px -17px;
    cursor: pointer;
    display: block;
    height: 30px;
    width: 30px;
}
.yin_na p .left_more.left_ma {
    background: rgba(0, 0, 0, 0) url("http://localhost/LYECS/LYECS-dev/public/static/pc/images/index/click.jpg") no-repeat scroll 15px 11px;
}
.hot_news a:hover {
    text-decoration: none;
    color: #ff4040;
}
</style>
