<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="会员中心"></CommonHeader>
    <CommonPageContainer>
        <div style="padding: 30px 0 0">
            <div data-panel-rev="aW1qeTIyODI" class="infout">
                <div class="user-info">
                    <div class="image-div">
                        <!-- <el-upload
                            :action="requestPrefix + '/user/user/modifyAvatar'"
                            :headers="uploadHeaders"
                            :on-success="handleAvatarSuccess"
                            :show-file-list="false"
                            class="avatar-uploader"
                        > -->
                        <el-image :src="imageFormat(formState.avatar)" fit="cover" />
                        <!-- </el-upload> -->
                    </div>
                    <div class="user-col">
                        <div class="username">
                            <div class="username_top">
                                <span> {{ formState.nickname }}</span>
                                <div v-if="isB2B() && commonStore.closeAuth === 1">
                                    <div class="auth" @click="toAuth">
                                        <AuthBadge :verified="Number(formState.isCompanyAuth) > 0" />
                                    </div>
                                </div>
                                <span class="user_rank" style="margin: 0 0 0 10px">
                                    <el-image :src="formState.rankIco" style="width: 20px" />
                                </span>
                            </div>
                        </div>
                        <div class="userTitle">
                            <div class="flex_csb" @click="editMaterials">
                                <SvgIcon name="entd" width="24px" height="24px"></SvgIcon>
                                <div class="mg_8">
                                    {{ $t("编辑个人资料") }}
                                </div>
                            </div>
                            <div class="flex_csb" style="margin: 0 32px" @click="Account">
                                <SvgIcon name="Settings" width="24px" height="24px"></SvgIcon>
                                <div class="mg_8">
                                    {{ $t("账户") }}
                                </div>
                            </div>
                            <NuxtLink to="/member/address/list" class="right-link">
                                <div class="flex_csb">
                                    <SvgIcon name="adders" width="24px" height="24px"></SvgIcon>
                                    <div class="mg_8">
                                        {{ $t("我的地址") }}
                                    </div>
                                </div>
                            </NuxtLink>
                        </div>
                    </div>
                </div>
            </div>
            <div class="userTotu">
                <div class="userTotuOne userTotuTwo">
                    <span>
                        <SvgIcon name="myAde" width="24px" height="24px"></SvgIcon>
                    </span>
                    <NuxtLink to="/member/order/list" class="right-link">
                        <span class="userTotuTwo_text userTotuTwo_om">
                            <span class="flex_ac">
                                <span>
                                    {{ $t("我的订单") }}
                                </span>
                                <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                            </span>
                        </span>
                    </NuxtLink>
                </div>
                <NuxtLink to="/member/order/list?type=awaitPay" class="right-link">
                    <div class="userTotuTwo">
                        <span class="userTotuTwo_number">{{ formState.awaitPay ? formState.awaitPay : "-" }}</span>
                        <span class="userTotuTwo_text">{{ $t("待付款") }}</span>
                    </div>
                </NuxtLink>
                <NuxtLink to="/member/order/list?type=awaitReceived" class="right-link">
                    <div class="userTotuTwo">
                        <span class="userTotuTwo_number">{{ formState.awaitReceived ? formState.awaitReceived : "-" }}</span>
                        <span class="userTotuTwo_text">{{ $t("待收货") }}</span>
                    </div>
                </NuxtLink>
                <!-- D1146：评价功能关闭时不展示「评论」入口 -->
                <NuxtLink to="/member/comment/list" class="right-link" v-if="commonStore.productCommentEnabled === 1">
                    <div class="userTotuTwo">
                        <span class="userTotuTwo_number">{{ ordernum.awaitComment ? ordernum.awaitComment : "-" }}</span>
                        <span class="userTotuTwo_text">{{ $t("评论") }}</span>
                    </div>
                </NuxtLink>
                <NuxtLink to="/member/return/list" class="right-link">
                    <div class="userTotuTwo">
                        <span class="userTotuTwo_number">{{ ordernum.awaitAftersalesCollect ? ordernum.awaitAftersalesCollect : "-" }}</span>
                        <span class="userTotuTwo_text">{{ $t("售后") }}</span>
                    </div>
                </NuxtLink>
            </div>
            <div class="userBottom">
                <div class="userBottom_boy">
                    <NuxtLink to="/member/coupon/list" class="right-link">
                        <div class="userBottom_boy_text">
                            <span class="flex_ac">
                                <SvgIcon name="exop" width="24px" height="24px"></SvgIcon>
                                <div class="mg_8">
                                    <span style="margin-right: 8px">
                                        {{ $t("优惠券") }}
                                    </span>
                                    {{ formState.awaitCoupon ? formState.awaitCoupon : "-" }}
                                </div>
                            </span>
                            <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                        </div>
                    </NuxtLink>
                    <div class="userBottom_boy_bot">{{ $t("查看优惠券") }}</div>
                </div>
                <div class="userBottom_boy">
                    <NuxtLink to="/member/collectProduct/list" class="right-link">
                        <div class="userBottom_boy_text">
                            <span class="flex_ac">
                                <SvgIcon name="love" width="24px" height="24px"></SvgIcon>
                                <div class="mg_8">
                                    {{ $t("心愿单") }}
                                </div>
                            </span>
                            <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                        </div>
                    </NuxtLink>
                    <div class="userBottom_boy_bot">{{ $t("查看心愿单") }}</div>
                </div>
                <div class="userBottom_boy" style="margin-right: 0">
                    <NuxtLink to="/member/browsingHistory/list" class="right-link">
                        <div class="userBottom_boy_text">
                            <span class="flex_ac">
                                <SvgIcon name="record" width="24px" height="24px"></SvgIcon>
                                <div class="mg_8">
                                    {{ $t("浏览记录") }}
                                </div>
                            </span>
                            <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                        </div>
                    </NuxtLink>
                    <div class="userBottom_boy_bot">{{ $t("查看浏览记录") }}</div>
                </div>
            </div>
        </div>
        <ProdSwiper class="member-prod-swiper" titleName="精选好物"></ProdSwiper>
    </CommonPageContainer>
    <!-- 编辑个人资料 -->
    <el-dialog v-model="visibleComment" :title="$t('编辑个人资料')" :footer="null" top="20vh" width="500px">
        <info @updateValue="updateValue"></info>
    </el-dialog>
    <el-dialog v-model="visibleSecurity" :title="$t('账户安全')" :footer="null" top="15vh" width="510px">
        <securityInfo @updateValue="updateValue"></securityInfo>
    </el-dialog>
    <CommonPageFooter :type="2" bg="#fff"></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 会员中心首页
 * - 展示用户基本信息、等级、余额、积分
 * - 各状态订单数量快捷入口
 * - 收藏商品、浏览历史展示
 * - 快捷功能入口（地址、收藏、优惠券等）
 */
import { reactive, ref, onMounted } from "vue";
import type { UserFormState } from "~/types/user/user";
import { getHistoryProduct, getMemberCenter } from "~/api/user/user";
import { getOrderNum } from "~/api/user/order";
import type { CollectProductFilterParams, CollectProductFilterState } from "~/types/user/collectProduct";
import { getCollectProductList } from "~/api/user/collectProduct";
import { urlFormat } from "~/utils/format";
import type { UploadProps } from "element-plus";
import { requestHeader, requestPrefix } from "~/utils/request";
import { useUserStore } from "~/store/user";
import { isB2B } from "@/utils/util";

import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import AuthBadge from "~/components/common/AuthBadge.vue";
import ProdSwiper from "../cart/src/ProdSwiper.vue";
import info from "./profile/Info.vue";
import securityInfo from "./security/dialogInfo.vue";
import { useCommonStore } from "~/store/common";

definePageMeta({
    middleware: "auth"
});

const commonStore = useCommonStore();

const uploadHeaders = requestHeader();
const handleAvatarSuccess: UploadProps["onSuccess"] = (response, uploadFile) => {
    formState.value.avatar = URL.createObjectURL(uploadFile.raw!);
};
const userStore = useUserStore();
userStore.getUserInfo();
const tabType = ref<number>(1);
const ordernum = ref<object>({});

const swiperOption = ref<any>({
    autoplay: false,
    navigation: true,
    effect: "slide",
    loop: true
});
const formState = ref<UserFormState>({
    ...userStore.userInfo
});
const toAuth = () => {
    navigateTo(`/member/userCertification/info`);
};
const getUserInfo = async () => {
    try {
        const result = await getMemberCenter();
        Object.assign(formState.value, result);
        console.log(formState.value);
    } catch (error: any) {
        message.error(error.message);
    }
};
const getawaitInfo = async () => {
    try {
        const result = await getOrderNum();
        ordernum.value = result;
    } catch (error: any) {
        message.error(error.message);
    }
};
const visibleComment = ref(false);
const visibleSecurity = ref(false);
onMounted(() => {
    getUserInfo();
    getawaitInfo();
    getBrowseList();
});
const collectList = ref<CollectProductFilterState[]>([]);
const collectParams = reactive<CollectProductFilterParams>({
    //初使化用于查询的参数
    page: 1,
    size: 15,
    sortField: "",
    sortOrder: "",
    keyword: ""
});
const browseList = ref<CollectProductFilterState[]>([]);
const getBrowseList = async () => {
    try {
        const result = await getHistoryProduct({ ...collectParams });
        browseList.value.push(...filterStateToList(result));
    } catch (error: any) {
        message.error(error.message);
    }
};
const getCollectList = async () => {
    try {
        const result = await getCollectProductList({ ...collectParams });
        if (result.records.length > 0) collectList.value.push(...filterStateToList(result.records));
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
getCollectList();
const filterStateToList = (list: any) => {
    let result = [];
    for (let i = 0; i < list.length; i += 5) {
        let chunk: any = list.slice(i, i + 5);
        result.push(chunk);
    }
    return result;
};
onMounted(() => {
    document.body.style.backgroundColor = "#fff";
});

onUnmounted(() => {
    document.body.style.backgroundColor = "";
});

/**编辑资料 */
const editMaterials = () => {
    visibleComment.value = true;
};
const updateValue = () => {
    visibleComment.value = false;
    getUserInfo();
};
/**账户 */
const Account = () => {
    visibleSecurity.value = true;
};
/**我的地址 */
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.member-prod-swiper {
    margin-top: 64px;
}

.bw {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 10px;

    .h4 {
        display: flex;
        justify-content: center;
        cursor: pointer;
        align-items: center;
    }
}
.infout {
    width: 100%;
    padding: 20px 24px;
    height: 80px;
    display: flex;
    justify-content: space-between;
    padding-right: 0;
    .user-info {
        display: flex;
        align-items: center;
        flex: 1;

        .image-div {
            position: relative;
            width: 60px;
            height: 60px;
            border-radius: 50%;
            margin-right: 20px;
            overflow: hidden;
            display: flex;
            justify-content: center;
            align-items: center;
            background-color: #fff;

            .el-image {
                width: 100%;
                height: 100%;
            }
        }

        .user-col {
            display: flex;
            flex-direction: column;
            justify-content: space-between;
            height: 76px;
            margin-right: 20px;
            .username {
                font-size: 18px;
                display: flex;
                // justify-content: center;
                color: #212121;
                .username_top {
                    display: flex;
                    align-items: flex-end;
                    width: 750px;
                    height: 32px;
                    font-size: 28px;
                    font-weight: 700;
                    line-height: 28px;
                    text-align: left;
                    // text-underline-position: from-font;
                    text-decoration-skip-ink: none;
                    margin-right: 16px;
                    white-space: nowrap;
                    text-overflow: ellipsis;
                }

                .auth {
                    display: flex;
                    align-items: center;
                    position: relative;
                    margin-left: 15px;
                    cursor: pointer;
                    .auth-icon {
                        height: 20px;
                        // width: 20px;
                        //position: absolute;
                    }
                    .auth-txt {
                        height: 16px;
                        padding: 0 5px 0 23px;
                        font-size: 12px !important;
                        line-height: 16px;
                        border-radius: 100px;
                        border-left: none;
                        color: #f8aa19;
                        border: 1px solid #f8aa19;
                    }
                    .unverified {
                        color: #999;
                        border-color: #999;
                    }
                }
            }
            .userTitle {
                display: flex;
                .flex_csb {
                    color: #000;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    font-size: 20px;
                    font-weight: 400;
                    line-height: 24px;
                    text-align: center;
                    text-underline-position: from-font;
                    text-decoration-skip-ink: none;
                    cursor: pointer;
                    &:hover {
                        color: var(--general);
                    }
                }
            }
        }

        .right-info {
            display: flex;
            flex-direction: row;
            align-items: center;
            justify-content: center;
            font-size: 14px;

            gap: 40px;
            flex: 1;
            .right-link {
                height: 100px;
            }
            .ico-page {
                cursor: pointer;
                width: 68px;
                display: flex;
                flex-direction: column;
                justify-content: center;
                align-items: center;
                gap: 4px;
                color: #969696;

                &:hover {
                    .pic {
                        top: -10px;
                    }

                    .mes_name {
                        color: #333;
                    }
                }
            }

            .coupon_pic {
                background: var(--main-bg) url(/assets/images/index/icon-coupon.png) no-repeat center;
            }

            .pay_pic {
                background: var(--main-bg) url(/assets/images/index/icon-pay.png) no-repeat center;
            }

            .take_pic {
                background: var(--main-bg) url(/assets/images/index/icon-take.png) no-repeat center;
            }

            .ass_pic {
                background: var(--main-bg) url(/assets/images/index/icon-ass.png) no-repeat center;
            }

            .pic {
                height: 60px;
                width: 60px;
                border-radius: 60px;
                position: relative;
                top: 0;
                transition: all 0.2s ease 0s;
            }
        }
    }
}
.userTotu {
    display: flex;
    justify-content: space-between;
    margin-top: 24px;
    background-color: #f5f6fa;
    padding: 20px 24px;
    border-radius: 8px;
    .userTotuOne {
        background-color: transparent !important;
        &:hover {
            background-color: #f0f1f5 !important;
        }
    }
    .userTotuTwo {
        width: 224px;
        height: 60px;
        text-align: center;
        display: flex;
        flex-direction: column;
        justify-content: space-between;
        align-items: center;
        padding: 10px 0;
        background-color: #fff;
        border-radius: 8px;
        position: relative;
        cursor: pointer;
        .userTotuTwo_number {
            font-size: 24px;
            font-weight: 400;
            line-height: 24px;
            text-align: left;
            text-underline-position: from-font;
            text-decoration-skip-ink: none;
            color: #1a1a1a;
        }
        .userTotuTwo_text {
            font-size: 18px;
            font-weight: 400;
            line-height: 24px;
            text-align: left;
            text-underline-position: from-font;
            text-decoration-skip-ink: none;
            color: #1a1a1a;
        }
        .userTotuTwo_om {
            font-size: 22px;
            font-weight: 700;
            line-height: 24px;
            text-align: center;
            text-underline-position: from-font;
            text-decoration-skip-ink: none;
        }
        &:hover {
            background-color: #fafafa;
        }
    }
}
.userBottom {
    display: flex;
    justify-content: space-between;
    margin-top: 32px;
    .userBottom_boy {
        width: 33%;
        height: 64px;
        border-radius: 8px;
        background-color: #f5f6fa;
        padding: 24px 24px 16px;
        margin-right: 16px;
        display: flex;
        flex-direction: column;
        justify-content: space-between;
        cursor: pointer;
        position: relative;
        .userBottom_boy_text {
            display: flex;
            justify-content: space-between;
            align-items: baseline;
            font-size: 22px;
            font-weight: 600;
            line-height: 22px;
            text-align: left;
            text-underline-position: from-font;
            text-decoration-skip-ink: none;
            color: #1a1a1a;
        }
        .userBottom_boy_bot {
            font-size: 18px;
            font-weight: 400;
            line-height: 24px;
            text-align: left;
            text-underline-position: from-font;
            text-decoration-skip-ink: none;
            color: #888b94;
        }
        &:hover {
            background-color: #f0f1f5;
        }
    }
}
.flex_ac {
    display: flex;
    align-items: center;
}
.mg_8 {
    margin-left: 4px;
}
</style>
