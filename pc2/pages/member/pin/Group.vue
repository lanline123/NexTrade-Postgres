<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonPageContainer class="group-info-body">
        <div class="group-info">
            <div class="title">
                <!--                <img src="https://oss.lyecs.com/img/item/demo/1682045946N1L3hAOFXpRekIdsLf!!pic400x400.jpeg">-->
                <div class="title-info">
                    <div>
                        <h2>{{ $t("赛八仙贡品芽尖2023新茶毛尖茶叶信阳绿茶明前特级嫩芽高端毛尖茶") }}</h2>
                    </div>
                    <div class="wire"></div>
                    <div class="info">
                        2 {{ $t("人团，拼团价") }}：
                        <FormatPrice v-model="price" :fontStyle="{ color: '#ED3B3B', fontSize: '30px', fontWeight: '700' }"></FormatPrice>
                        {{ $t("单购价") }}：
                        <FormatPrice v-model="price" :fontStyle="{ color: '#333', fontSize: '14px', fontWeight: 'normal' }"></FormatPrice>
                        ，{{ $t("组团采购省") }}
                        <FormatPrice v-model="price" :fontStyle="{ color: '#ED3B3B', fontSize: '14px' }"></FormatPrice>
                        ， <a href="/member/order/info" style="text-decoration: underline">{{ $t("查看商品详情") }}</a>
                    </div>
                </div>
            </div>
            <div class="user">
                <div class="avatar-list">
                    <template v-for="(item, index) in userList">
                        <div class="avatar-container">
                            <div class="circle">
                                <img :src="item.url" />
                                <div v-if="index == 0" class="tips">{{ $t("团长") }}</div>
                            </div>
                        </div>
                        <div v-if="index < userList.length - 1" class="group-line"></div>
                    </template>
                </div>
                <div class="avatar-list">
                    <template v-for="(item, index) in userList">
                        <div class="username">
                            {{ item.name }}
                        </div>
                        <div v-if="index < userList.length - 1" style="width: 60px"></div>
                    </template>
                </div>
                <div class="message">{{ $t("还差") }} <span class="red">2</span> {{ $t("人就成团啦，盼来盼去我在盼着你出现！") }}</div>
                <div class="tig-button theme-button big-button">{{ $t("邀请好友参团") }}</div>
                <div>
                    {{ $t("开团并邀请好友参团，或等系统推荐陌生人参团，组团成功立即发货。详情见") }}<a href="/member/order/info" style="text-decoration: underline"
                        >{{ $t("拼团玩法") }}</a
                    >
                </div>
            </div>
            <div class="time">
                {{ $t("剩余") }}：
                <div>33</div>
                {{ $t("天") }}
                <div>07</div>
                :
                <div>22</div>
            </div>
        </div>
    </CommonPageContainer>

    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 拼团详情页面
 * - 展示拼团活动详情，包括参团成员头像
 * - 显示拼团价格和商品信息
 */
import { ref } from "vue";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";

const price = ref(999);

const userList = ref([
    { url: "https://demo2.lyecs.com/img/user_photos/202307/1688456028wlkWXLfH03DL8vuFHa!!pic.jpeg", name: "1****3" },
    { url: "https://demo2.lyecs.com/static/pc/images/pin/empty_face.png", name: "" },
    { url: "https://demo2.lyecs.com/static/pc/images/pin/empty_face.png", name: "" }
]);
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.group-info-body {
    background-color: white;
    width: 100%;
    display: flex;
    justify-content: center;

    .group-info {
        width: 1190px;

        .title {
            margin: 30px 0 20px;
            padding: 10px;
            box-sizing: border-box;
            background-color: #f7f7f7;
            gap: 30px;
            display: flex;
            flex-direction: row;

            & > img {
                height: 150px;
                width: 150px;
            }

            .title-info {
                width: 100%;
                display: flex;
                flex-direction: column;

                .wire {
                    border-bottom: 1px solid #dfdfdf;
                    padding: 10px 0 20px;
                    width: 100%;
                }

                h2 {
                    font-size: 22px;
                    font-weight: 700;
                }

                .info {
                    margin: 10px 0;
                    font-size: 14px;
                }
            }
        }

        .user {
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            gap: 20px;

            .avatar-list {
                display: flex;
                width: 100%;
                justify-content: center;
                flex-direction: row;
                align-items: center;

                .avatar-container {
                    position: relative;
                    box-sizing: border-box;

                    display: inline-block;

                    .circle {
                        width: 100px;
                        height: 100px;
                        background-color: #f8f8f8;
                        border: 1px solid #e4e4e4;
                        border-radius: 50%;
                        overflow: hidden;

                        .tips {
                            position: absolute;
                            top: 0;
                            right: 0;
                            padding: 0 8px;

                            color: white;
                            line-height: 22px;
                            border-radius: 22px;
                            background-color: #f98825;
                        }

                        & > img {
                            width: 100px;
                            height: 100px;
                        }
                    }
                }

                .username {
                    width: 100px;
                    text-align: center;
                    color: #999999;
                    margin-top: -20px;
                }

                .group-line {
                    border-bottom: 1px solid #ddd;
                    width: 60px;
                    height: 3px;
                }
            }

            .message {
                font-size: 14px;
                line-height: 1.8;
            }
        }

        .time {
            margin: 30px 0 20px;
            padding: 10px;
            box-sizing: border-box;
            background-color: #f7f7f7;
            display: flex;
            color: #999999;
            justify-content: center;
            font-size: 16px;
            align-items: center;
            gap: 5px;

            & > div {
                width: 30px;
                height: 30px;
                border-radius: 4px;
                line-height: 30px;
                text-align: center;
                color: white;
                background: linear-gradient(to bottom, rgba(0, 0, 0, 0.5) 50%, rgba(0, 0, 0, 0.8) 50%, rgba(0, 0, 0, 1) 50%);
            }
        }
    }
}
</style>
