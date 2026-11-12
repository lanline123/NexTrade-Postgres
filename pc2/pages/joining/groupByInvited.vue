<template>
    <div class="page">
        <div class="content">
            <!-- 产品信息区域 -->
            <div class="product-section card-box" shadow="never">
                <div class="product-info">
                    <div class="product-image">
                        <img :src="productImage(formState)" :alt="formState.productName" />
                    </div>
                    <div class="product-details">
                        <div class="product-details-top">
                            <h1 class="product-title">{{ formState.productName }}</h1>
                            <div class="group-tag">
                                <el-tag plain type="primary" class="tag">{{ formState.minGroupNum }}{{ $t("人团") }}</el-tag>
                                <span class="joined-count">{{ $t("已有") }}{{ formState.joinNumCount }}{{ $t("人参团") }}</span>
                            </div>
                        </div>
                        <div class="price-info">
                            <span class="group-price"
                                >{{ $t("拼团价") }}:
                                <FormatPrice
                                    class="groupPriceCom"
                                    :decimalsStyle="{ fontSize: '14px', alignSelf: 'self-end' }"
                                    :fontStyle="{ fontSize: '18px', fontWeight: 'bold' }"
                                    v-model="formState.minGrouponPrice"
                                >
                                </FormatPrice>
                            </span>
                            <span class="original-price">
                                <FormatPrice
                                    :fontStyle="{ fontSize: '16px', color: '#999' }"
                                    v-model="formState.productPrice"
                                    style="text-decoration: line-through"
                                ></FormatPrice>
                            </span>
                        </div>
                    </div>
                </div>
            </div>

            <!-- 拼团状态区域 -->
            <div class="group-status-section card-box" shadow="never">
                <div class="status-info">
                    <div class="remaining-members">
                        <span class="remaining-text">
                            <span>{{ formState.joinStatusDesc || "" }}</span>
                            <span v-if="formState.joinStatus === 1">
                                ,{{ $t("还差") }}
                                <span style="color: var(--main-bg)">{{
                                    formState.joinNumCount >= formState.minGroupNum ? 0 : formState.minGroupNum - formState.joinNumCount
                                }}</span>
                                {{ $t("人成团") }}
                            </span>
                        </span>
                    </div>
                    <div class="countdown" v-if="formState.joinStatus === 1">
                        <CountdownNew countdown-type="specifyTime" :specify-time="new Date(formState?.endTime)?.getTime()" class="countdown-text">
                        </CountdownNew>
                    </div>
                </div>
                <div class="group-content" v-if="joinUserList.length > 0 || formState.minGroupNum > 0">
                    <div class="group-content-item" v-for="item in joinUserList" :key="item.userId">
                        <div class="group-content-item-image">
                            <div v-if="item.classIcon" :class="item.classIcon" class="iconBox"></div>
                            <div v-else class="group-content-item-image-img">
                                <img
                                    :src="item.avatar ? item.avatar : '/assets/avatar/default.gif'"
                                    :alt="item.username"
                                />
                            </div>
                            <div class="group-content-item-image-mask" v-if="item.leader">{{ $t("团长") }}</div>
                        </div>
                        <div class="group-content-item-bottom">
                            <span>{{ item.username === "待加入" ? $t("待加入") : item.username }}</span>
                        </div>
                    </div>
                </div>
                <div v-else class="empty-state">
                    <p>{{ $t("暂无拼团数据") }}</p>
                </div>
            </div>

            <!-- 拼团流程区域 -->
            <div class="process-section card-box" shadow="never">
                <h3 class="section-title">{{ $t("拼团流程") }}</h3>
                <div class="group-steps">
                    <div class="group-steps-content">
                        <div class="group-steps-item">
                            <img src="/assets/images/group/group_open.png" />
                            <span>{{ $t("1.开团/参加") }}</span>
                        </div>
                        <span class="step-dots">
                            <span v-for="i in 3" :key="i" class="dot"></span>
                        </span>
                        <div class="group-steps-item">
                            <img src="/assets/images/group/group_invite.png" />
                            <span>{{ $t("2.邀友参团") }}</span>
                        </div>
                        <span class="step-dots">
                            <span v-for="i in 3" :key="i" class="dot"></span>
                        </span>
                        <div class="group-steps-item">
                            <img src="/assets/images/group/group_done.png" />
                            <span>{{ $t("3.人满成团") }}</span>
                        </div>
                    </div>
                </div>
            </div>

            <!-- 门店信息区域 -->
            <div class="store-section card-box" shadow="never" v-if="shopInfo.shopId !== 0">
                <div class="store-info">
                    <div class="store-logo">
                        <div class="img-box">
                            <img :src="shopInfo.shopLogo" alt="" v-if="shopInfo.shopLogo" />
                            <p v-else>T</p>
                        </div>

                        <div class="store-name">{{ shopInfo.shopTitle ? shopInfo.shopTitle : "--" }}</div>
                    </div>
                    <div class="store-actions">
                        <el-button class="action-btn" type="primary" plain @click="collectShop" size="large">
                            <span class="iconfont-pc" :class="{ 'icon-shoucang': !shopInfo.collectShop, 'icon-yishoucang': shopInfo.collectShop }"></span>
                            <span>{{ $t("收藏门店") }}</span>
                        </el-button>
                        <el-button type="primary" plain size="large" @click="goToShop" class="action-btn">
                            <span class="iconfont-pc icon-guanli"></span>
                            <span>{{ $t("进店逛逛") }}</span>
                        </el-button>
                    </div>
                </div>
            </div>
        </div>
        <!-- 底部导航栏 -->
        <div class="bottom-nav">
            <div class="nav-left">
                <NuxtLink target="_blank" :to="urlFormat('/member/joining/list')">
                    <div class="nav-item">
                        <div class="nav-item-img">
                            <img src="/assets/images/group/group_snap.png" />
                        </div>
                        <span>{{ $t("我的拼团") }}</span>
                    </div>
                </NuxtLink>
                <div class="nav-item" @click="goToShop" v-if="shopInfo.shopId !== 0">
                    <div class="nav-item-img">
                        <img src="/assets/images/group/group_store.png" />
                    </div>
                    <span>{{ $t("门店") }}</span>
                </div>
            </div>
            <div class="nav-right">
                <el-button type="primary" style="width: 100%" :disabled="formState.joinStatus !== 1" @click="joinGroup">{{ $t("去参团") }}</el-button>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import type { JoiningDetail, JoinUser } from "~/types/joining/joining";
import type { OrderFormState } from "~/types/user/order";
import { getOrder } from "~/api/user/order";
import { getJoiningDetail } from "~/api/joining/joining";
import { message, Modal } from "ant-design-vue";
import { useCommonStore } from "~/store/common";
import { CountdownNew } from "@/components/product";
import { productImage } from "~/utils/format";
import { getShopDetail, getShopCollect } from "~/api/product/product";
import { hasToken } from "~/utils/util";

const commonStore = useCommonStore();
const { t } = useI18n();
const router = useRouter();
const { id } = router.currentRoute.value.query as { id: string };

definePageMeta({
    middleware: "auth"
});

const formState = ref<JoiningDetail>({
    joinStatus: 0,
    grouponPromotionRecordId: 0,
    grouponPromotionItemId: 0,
    productName: "",
    joinType: 0,
    minGroupNum: 0,
    shopId: 0,
    shopTitle: "",
    grouponImage: "",
    minGrouponPrice: 0,
    productPrice: 0,
    picThumb: "",
    joinNumCount: 0,
    joinUsers: [],
    endTime: "",
    orderId: 0,
    productSn: "",
    joinStatusDesc: ""
} as JoiningDetail);

const orderFormState = ref<OrderFormState>({
    orderSn: "",
    orderId: 0,
    items: [],
    availableActions: {},
    shop: {}
} as OrderFormState);

const shopInfo = ref<any>({});
const loading = ref<boolean>(false);
const joinUserList = ref<JoinUser[]>([]);
const isLogin = ref<boolean>(false);
const LoadingBox = ref();

const joinStatusInfo = ref(["待成团", "拼团成功", "拼团失败"]);

//获取门店信息
const fetchShopDetail = async (shopId: number) => {
    try {
        const result = await getShopDetail(shopId);
        Object.assign(shopInfo.value, result);
    } catch (error: any) {
        message.error(error.message);
    }
};

//获取订单信息
const fetchOrder = async (id: number) => {
    try {
        const result = await getOrder({ id });
        Object.assign(orderFormState.value, result);
    } catch (error: any) {
        message.error(error.message);
    }
};
//获取拼团信息
const fetchJoiningDetail = async () => {
    try {
        LoadingBox.value = ElLoading.service({
            lock: false,
            text: t("请求加载中"),
            customClass: "tig-normal-loading"
        });
        const result = await getJoiningDetail({ grouponPromotionRecordId: router.currentRoute.value.query.id });
        formState.value = result;
        joinUserList.value = result.joinUsers;
        //如果参团人数不够，差多少人显示多少个占位
        if (formState.value.joinNumCount < formState.value.minGroupNum) {
            joinUserList.value.push({
                userId: 0,
                // i18n-ignore-next-line 占位哨兵值，模板里按 username === "待加入" 判定后再 $t
                username: "待加入",
                avatar: "/assets/avatar/default.gif",
                classIcon: "iconfont-pc icon-zengjia"
            });
        }
        //将joinUserList.value中的leader（团长）为true的放在最前面
        joinUserList.value.sort((a, b) => {
            return a.leader === true ? -1 : 1;
        });
        fetchOrder(result.orderId);
        fetchShopDetail(result.shopId);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
        LoadingBox.value?.close();
    }
};

//去门店
const goToShop = () => {
    if (shopInfo.value.shopId === 0) {
        return message.error(t("自营门店无法进店"));
    }
    navigateTo(`/store/info?shopId=${shopInfo.value?.shopId}`);
};

//收藏门店
const collectShop = async () => {
    try {
        if (shopInfo.value.shopId === 0) {
            return message.error(t("自营门店无法收藏"));
        }
        const result = await getShopCollect(shopInfo.value.shopId);
        message.success(t("操作成功"));
    } catch (error: any) {
        message.error(error.message);
    }
};

//去参团
const joinGroup = () => {
    if (hasToken()) {
        navigateTo(
            urlFormat({
                path: "product",
                seoLink: formState.value.seoLink,
                id: formState.value.productId,
                sn: formState.value.productSn,
                type: "invited",
                grouponPromotionRecordId: id
            })
        );
    } else {
        navigateTo("/member/login");
    }
};

//确认用户是否已登录
// const checkLogin = () => {
//   if (hasToken()) {
//     isLogin.value = true;
//   }
// };

// checkLogin();
fetchJoiningDetail();
</script>

<style scoped lang="less">
.page {
    height: 100vh;
    background-color: #fff;
}

.card-box {
    padding: 20px;
    background-color: #fff;
}

.header {
    text-align: center;
    padding: 40px 20px;
    background: #fff;
    margin-bottom: 20px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);

    h1 {
        font-size: 28px;
        color: #333;
        margin: 0 0 10px 0;
        font-weight: bold;
    }

    p {
        font-size: 16px;
        color: #666;
        margin: 0;
    }
}

.content {
    width: calc(100vh * 0.5);
    max-width: 100vw;
    margin: 0 auto;
    overflow: hidden;
    height: 100vh;
    padding: 10px;
    box-sizing: border-box;
    overflow-y: auto;
    background: #f4f4f4;

    &::-webkit-scrollbar {
        display: none;
    }
}

.product-section {
    margin-bottom: 10px;
}

.product-info {
    display: flex;
    gap: 20px;
    padding: 20px 0;

    .product-image {
        width: calc(100vh * 0.5 * 0.3);
        height: calc(100vh * 0.5 * 0.3);
        border-radius: 8px;
        overflow: hidden;
        flex-shrink: 0;

        img {
            width: 100%;
            height: 100%;
            object-fit: contain;
        }
    }

    .product-details {
        flex: 1;
        display: flex;
        flex-direction: column;
        justify-content: space-between;

        .product-details-top {
            display: flex;
            flex-direction: column;
            gap: 15px;
        }

        .product-title {
            font-size: 20px;
            font-weight: 600;
            color: #333;
            line-height: 1.4;
            font-weight: normal;
            overflow: hidden;
            text-overflow: ellipsis;
            display: -webkit-box;
            -webkit-line-clamp: 2;
            -webkit-box-orient: vertical;
        }

        .group-tag {
            display: flex;
            align-items: center;
            gap: 15px;
            margin-bottom: 15px;

            .tag {
                color: var(--general);
                padding: 4px 8px;
                border-radius: 4px;
                font-size: 14px;
            }

            .joined-count {
                color: #999;
                font-size: 14px;
            }
        }

        .price-info {
            display: flex;
            align-items: end;
            gap: 15px;

            .group-price {
                color: var(--general);
                font-size: 18px;
                font-weight: bold;
                display: flex;
                justify-content: space-between;
                align-items: end;
                gap: 5px;
            }

            .original-price {
                color: #999;
                font-size: 14px;
            }
        }
    }
}
.groupPriceCom {
    height: 22px;
    :deep(.util) {
        font-size: 12px !important;
    }
}

.group-status-section {
    margin-bottom: 10px;

    .status-info {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 20px;
        padding-bottom: 20px;
        border-bottom: 1px solid #f0f0f0;

        .remaining-members {
            .remaining-text {
                font-size: 18px;
                font-weight: bold;
            }
        }

        .countdown {
            // display: flex;
            // align-items: center;
            // gap: 8px;
            font-size: 16px;
            font-weight: bold;
        }
    }

    .group-content {
        display: flex;
        align-items: center;
        gap: 20px;
        padding: 20px 0;
        // 添加滚动条
        overflow-x: auto;

        &::-webkit-scrollbar {
            display: none;
        }

        .group-content-item {
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 15px;
            padding: 0 20px;

            .group-content-item-image {
                background-color: #f4f4f4;
                position: relative;
                width: calc(100vw * 0.5 * 0.06);
                height: calc(100vw * 0.5 * 0.06);
                border-radius: 50%;

                .iconBox {
                    position: absolute;
                    left: 50%;
                    top: 50%;
                    transform: translate(-50%, -50%);
                    font-size: 28px;
                    color: #c3c3c3;
                }

                .group-content-item-image-img {
                    width: 100%;
                    height: 100%;
                    border-radius: 50%;
                    overflow: hidden;

                    img {
                        width: 100%;
                        height: 100%;
                        object-fit: contain;
                    }

                    .empty-avatar {
                        width: 100%;
                        height: 100%;
                        background: #f0f0f0;
                        border-radius: 50%;
                    }
                }

                .group-content-item-image-mask {
                    position: absolute;
                    bottom: 0;
                    right: 50%;
                    transform: translateX(50%) translateY(70%);
                    background-color: var(--main-bg);
                    color: #fff;
                    border-radius: 2px;
                    padding: 2.5px 4px;
                    z-index: 999;
                    font-size: 12px;
                    text-align: center;
                    min-width: 24px;
                }
            }

            .group-content-item-bottom {
                span {
                    display: inline-block;
                    font-size: 16px;
                    font-weight: bold;
                    color: #9a9a9a;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    white-space: nowrap;
                    max-width: 5em;
                }
            }

            &:first-child .group-content-item-image {
                border: 2px solid var(--main-bg);
            }
        }
    }
}

.process-section {
    margin-bottom: 10px;

    .section-title {
        font-size: 18px;
        color: #333;
        margin-bottom: 10px;
        font-weight: bold;
    }

    .group-steps {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 20px 0;

        .group-steps-content {
            width: 90%;
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 10px;
            padding: 10px;

            .group-steps-item {
                display: flex;
                flex-direction: column;
                align-items: center;
                justify-content: center;
                gap: 10px;
                width: calc(100vh * 0.5 * 0.12);
                height: calc(100vh * 0.5 * 0.12);

                img {
                    width: calc(100vh * 0.5 * 0.1);
                    height: calc(100vh * 0.5 * 0.1);
                    object-fit: contain;
                }

                span {
                    font-size: 16px;
                    color: #666;
                    font-weight: bold;
                    min-width: 6em;
                    text-align: center;
                }
            }

            .step-dots {
                display: inline-flex;
                align-items: center;

                .dot {
                    display: inline-block;
                    width: 10px;
                    height: 10px;
                    border-radius: 50%;
                    background: #f5f5f5;
                    margin: 0 3px;
                }
            }
        }
    }
}

.store-section {
    margin-bottom: 10px;

    .store-info {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        padding: 20px 0;
        gap: 30px;

        .store-logo {
            width: 100%;
            display: flex;
            align-items: center;
            gap: 20px;
            height: 40px;

            .img-box {
                width: calc(100vh * 0.5 * 0.1);
                height: calc(100vh * 0.5 * 0.1);
                overflow: hidden;
                border-radius: 50%;
                border: 1px solid #f0f0f0;

                img {
                    width: 100%;
                    height: 100%;
                    object-fit: contain;
                }

                p {
                    width: 100%;
                    height: 100%;
                    background: #f0f0f0;
                    border-radius: 50%;
                    font-size: 30px;
                    font-weight: bold;
                    color: #333;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                }
            }

            .store-name {
                font-size: 16px;
                color: #333;
                margin-bottom: 10px;
            }
        }

        .store-actions {
            width: 100%;
            display: flex;
            align-items: center;
            gap: 40px;

            .action-btn {
                flex: 1;
                width: 100%;
                padding: 0;

                :deep(.el-button) {
                    width: 100%;
                }

                &:hover {
                    .iconfont-pc {
                        color: #fff;
                        transition: color 0.3s ease;
                    }
                }

                &:not(:hover) {
                    .iconfont-pc {
                        color: var(--general);
                        transition: color 0.3s ease;
                    }
                }
            }
        }
    }
}

.bottom-nav {
    background: #fff;
    border-top: 1px solid #f0f0f0;
    padding: 15px 10px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: calc((100vh * 0.5) * 0.1);
    position: fixed;
    left: 50%;
    transform: translateX(-50%);
    bottom: 0;
    z-index: 99;
    width: calc(100vh * 0.5);
    height: calc(100vh * 0.5 * 0.2);
    box-sizing: border-box;

    .nav-left {
        display: flex;
        gap: 30px;
        justify-content: center;

        .nav-item {
            flex: 1;
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 8px;
            cursor: pointer;

            .nav-item-img {
                width: calc(100vh * 0.5 * 0.05);
                height: calc(100vh * 0.5 * 0.05);

                img {
                    width: 100%;
                    height: 100%;
                    object-fit: contain;
                }

                span {
                    font-size: 16px;
                    color: #666;
                }

                &:hover {
                    i,
                    span {
                        color: var(--main-bg);
                    }
                }
            }
        }

        .empty-state {
            text-align: center;
            padding: 40px 20px;
            color: #999;
            font-size: 14px;
        }

        .iconfont-pc {
            font-size: 18px;
            color: var(--general);
            margin-right: 5px;
        }
    }

    .nav-right {
        flex: 1;
        height: 60%;

        :deep(.el-button) {
            flex: 1;
            width: 100%;
            height: 100%;
            font-size: 20px;
            font-weight: bold;
            border-radius: calc((100vh * 0.5 * 0.2) * 0.6 * 0.5);
        }
    }
}

@media (max-width: 1700px) {
    //当屏幕宽度小于1700px时，高度为100vh，等比缩小页面内容
    .page {
        height: 100vh;
        overflow: hidden;
        font-size: 0.8em; // 字体同步缩小到80%
        position: relative;
    }

    .content {
        height: calc(100vh * 1.2); // 为底部导航栏留出空间
        transform: scale(0.8);
        transform-origin: top center;
        width: calc(100vh * 0.5 / 0.8);
    }

    .status-info {
        padding-bottom: 10px !important;
        margin-bottom: 10px !important;
    }

    .bottom-nav {
        transform: scale(0.8);
        transform-origin: bottom center;
        width: calc(100vh * 0.5 / 0.8);
        position: absolute;
        bottom: 0;
        left: 50%;
        transform: translateX(-50%) scale(0.8);
        box-sizing: border-box;
        padding: 10px;

        .nav-right {
            height: 90%;

            :deep(.el-button) {
                padding: 5px 0;
                font-size: 16px !important;
            }
        }
    }

    .dot {
        display: inline-block;
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: #ccc;
        margin: 0 3px;
    }

    .card-box {
        padding: 10px !important;
    }

    // 调整各种字体大小
    .product-title {
        font-size: 16px !important; // 20px * 0.8
    }

    .group-price {
        font-size: 14.4px !important; // 18px * 0.8
    }

    .original-price {
        font-size: 12.8px !important; // 16px * 0.8
    }

    .remaining-text {
        font-size: 14.4px !important; // 18px * 0.8
    }

    .countdown {
        font-size: 12.8px !important; // 16px * 0.8

        :deep(.countdown-text) {
            font-size: 12.8px !important; // 16px * 0.8
        }
    }

    .section-title {
        font-size: 14.4px !important; // 18px * 0.8
    }

    .group-steps-item span {
        font-size: 12.8px !important; // 16px * 0.8
    }

    .store-name {
        font-size: 12.8px !important; // 16px * 0.8
    }

    .store-info p {
        font-size: 20px !important;
    }

    .nav-item span {
        font-size: 12.8px !important; // 16px * 0.8
    }

    :deep(.el-button) {
        font-size: 12.8px !important;
    }

    .tag {
        font-size: 11.2px !important; // 14px * 0.8
    }

    .joined-count {
        font-size: 11.2px !important; // 14px * 0.8
    }

    .group-content-item-bottom span {
        font-size: 12.8px !important; // 16px * 0.8
    }

    .group-content-item-image-mask {
        font-size: 9.6px !important; // 12px * 0.8
    }

    // 成团人数头像最小尺寸
    .group-content-item-image {
        min-width: 50px !important;
        min-height: 50px !important;
    }

    // 拼团流程图片最小尺寸
    .group-steps-item img {
        min-width: 40px !important;
        min-height: 40px !important;
    }

    .iconBox {
        font-size: 24px;
    }
}

@media (max-width: 375px) {
    .content {
        width: 100vw;
        max-width: 100vw;
        border-radius: 0;
    }
}
</style>
