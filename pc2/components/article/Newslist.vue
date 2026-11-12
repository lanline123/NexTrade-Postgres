<template>
    <div class="lyecs-main">
        <div class="news-list">
            <div class="item" v-for="i in 6" :key="i">
                <div class="pic-desc">
                    <a href="/news/detail" alt="苹果推出储蓄服务年化收益率超4％ 每笔消费最高返现3％">
                        <img src="http://oss.lyecs.com/img/gallery/202304/1681799042kSoDuThq3gH8j7fszO!!pic.jpeg" />
                    </a>
                    <div>
                        <p>
                            <a class="pd-title" title="苹果推出储蓄服务年化收益率超4％ 每笔消费最高返现3％" href="/news/detail">
                                <span class="yin_agn">公告</span>苹果推出储蓄服务年化收益率超4％ 每笔消费最高返现3％
                            </a>
                        </p>
                        <p>
                            <a class="pd-det" href="/news/detail">
                                苹果公司周一宣布通过高盛银行发行Apple
                                Card储蓄卡，年化收益率为4.15%。苹果在官方网站上称，这一利率水平达到美国全国平均利率水平的10倍。
                            </a>
                        </p>
                        <div class="info-btn">
                            <div class="evaluate">
                                <span class="time">发布时间：2023-06-26 13:47:20</span>
                            </div>
                            <a class="redBtn" href="/news/detail" target="_blank">相关链接</a>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, shallowRef, onMounted } from "vue";
</script>
<style lang="less" scoped>
.lyecs-main {
    flex: 1;
    background: #fff;
}
.news-list {
    margin: 30px;
}
.news-list .item {
    overflow: hidden;
    border-bottom: 1px solid #f2f3f5;
    margin-bottom: 20px;
    padding-bottom: 20px;
}
.news-list .item .pic-desc {
    overflow: hidden;
    display: flex;
}
.news-list .item .pic-desc img {
    height: 80px;
    margin-right: 20px;
    overflow: hidden;
    width: 120px;
    border-radius: 2px;
}
.news-list .item .pic-desc .pd-title {
    color: #333;
    font-size: 16px;
    font-weight: bold;
    margin: 0 0 10px;
    overflow: hidden;
    display: block;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.news-list .item .pic-desc .pd-det {
    color: #999;
    font-size: 14px;
    line-height: 20px;
    overflow: hidden;
    width: 100%;
    display: block;
}
.news-list .item .info-btn {
    margin-top: 10px;
    display: flex;
    align-items: center;
}
.news-list .item .evaluate {
    border: 1px solid #e3e3e3;
    border-radius: 5px;
    height: 23px;
    line-height: 23px;
    margin-right: 19px;
}
.news-list .item .evaluate .time {
    padding-left: 10px;
    color: #9b9b9b;
    padding-right: 10px;
}
.news-list .item .info-btn .redBtn {
    border-radius: 5px;
    display: inline-block;
    width: 80px;
    height: 23px;
    background: #e03737;
    line-height: 23px;
    text-align: center;
    color: #fff;
}
.yin_agn {
    background: #ffecec none repeat scroll 0 0;
    color: #c00;
    font-size: 12px;
    font-weight: 500;
    height: 20px;
    line-height: 20px;
    margin: 0px 10px 0 0;
    display: inline-block;
    padding: 0 8px;
}
</style>
