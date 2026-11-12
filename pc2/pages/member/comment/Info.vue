<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="评价晒单"></CommonHeader>
    <TopJumpBar tltle="评价晒单"></TopJumpBar>
    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="collection">
                <div class="nav">
                    <div class="title-or-tabs">
                        <span class="card-title">{{ $t("订单评价") }}</span>
                    </div>
                    <!-- <div>
                        {{ $t("评价立送") }}<span class="font-color">5</span>{{ $t("积分") }}，{{ $t("晒单额外赠送") }}<span class="font-color">5</span
                        >{{ $t("积分") }}
                    </div> -->
                </div>
                <!-- D1146：评价功能关闭时页面保留、内容区换成空态，不做重定向 -->
                <template v-if="commentEnabled">
                    <div v-for="(item, index) in formState">
                        <div class="white-card" v-if="!item.isShowed">
                            <div class="product-left">
                                <NuxtLink
                                    :title="item.productName"
                                    :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })"
                                    class="nuxt"
                                    target="_blank"
                                >
                                    <div>
                                        <img :alt="item.productName || ''" :src="productImage(item)" />
                                    </div>
                                    <div>
                                        <div style="line-height: 2">
                                            <a>{{ item.productName }}</a>
                                        </div>
                                        <div>
                                            <FormatPrice
                                                v-model="item.price"
                                                :showText="false"
                                                :priceStyle="{ fontSize: '14px', lineHeight: '16px' }"
                                            ></FormatPrice>
                                        </div>
                                    </div>
                                </NuxtLink>
                            </div>
                            <div class="product-right">
                                <div>
                                    <span class="lab-text">{{ $t("商品评分") }}：</span>
                                    <span>
                                        <el-rate :disabled="item.isCommentInfo" v-model="item.commentRank" size="large" :texts="desc" show-text />
                                    </span>
                                </div>
                                <div v-if="!item.rankVerification">
                                    <span class="lab-text"></span>
                                    <p class="tips">{{ $t("请先评价商品") }}</p>
                                </div>
                                <div>
                                    <div class="lab-text">{{ $t("评价标签") }}：</div>
                                    <div class="label-list" v-if="!item.isCommentInfo">
                                        <div
                                            v-for="lb in item.allCommentTag"
                                            :style="{ border: item.commentTag.includes(lb) ? ' 2px solid var(--general)' : ' 1px solid #ccc' }"
                                            class="label-item"
                                            @click="checkTag(index, lb)"
                                        >
                                            <span>{{ $t(lb) }}</span>
                                        </div>
                                        <div class="label-edit" v-if="!item.isAdd" @click="isShowadd(index)">
                                            <div></div>
                                            <p>{{ $t("自定义") }}</p>
                                        </div>
                                        <div class="label-edit-input" v-else>
                                            <div>
                                                <el-input v-model="tabInput" autofocus />
                                            </div>
                                            <p @click="tabAdd(index)">{{ $t("确定") }}</p>
                                        </div>
                                    </div>
                                    <div class="label-list" v-else>
                                        <div v-for="lb in item.commentTag" class="label-item-txt">
                                            <span>{{ lb }}</span>
                                        </div>
                                    </div>
                                </div>
                                <div>
                                    <div class="lab-text">{{ $t("购物心得") }}：</div>
                                    <textarea
                                        cols="50"
                                        name="content"
                                        :placeholder="$t('商品是否给力？快分享你的购买心得吧~')"
                                        rows="5"
                                        style="border: 1px solid #d9d9d9; padding: 4px"
                                        v-model="item.content"
                                        :disabled="item.isCommentInfo"
                                    ></textarea>
                                </div>
                                <div v-if="!item.contentVerification">
                                    <span class="lab-text"></span>
                                    <p class="tips">{{ $t("请填写购买心得") }}</p>
                                </div>
                                <div>
                                    <div class="lab-text">{{ $t("晒单") }}：</div>
                                    <UploadImage v-if="!item.isShowed" v-model:uploadList="item.showPics" :limit="9" :fileTypes="['jpeg', 'png']"></UploadImage>
                                    <div v-else>
                                        <el-image
                                            v-for="product in item.showPics"
                                            :src="productImage(product)"
                                            loading="lazy"
                                            style="height: 80px; width: 80px; transition: opacity 0.2s linear; margin-right: 10px"
                                        />
                                    </div>
                                </div>
                                <!-- D1146：匿名勾选，仅在总开关与匿名开关都开启且该条尚未评价时出现 -->
                                <div v-if="anonymousEnabled && !item.isCommentInfo" class="anonymous-row">
                                    <el-checkbox v-model="item.isAnonymous" :true-value="1" :false-value="0">
                                        {{ $t("匿名评价") }}
                                    </el-checkbox>
                                    <span v-if="anonymousPreview" class="anonymous-name">{{ anonymousPreview }}</span>
                                </div>
                                <div>
                                    <el-button type="primary" @click="onSubmit(item, index)">{{ $t("提交评价") }}</el-button>
                                </div>
                            </div>
                        </div>
                    </div>
                </template>
                <CommonAppEmpty v-else :description="$t('评价功能已关闭')" />
            </div>
        </div>
    </CommonPageContainer>

    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 发表评价页面
 * - 对订单商品进行评分（1-5星）
 * - 填写评价内容、上传图片
 * - 支持匿名评价
 */
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { getCommentData, updateCommentData } from "~/api/user/comment";
import type { ProductItems, CommentEditFormData } from "~/types/user/comment.d";
import UploadImage from "~/components/form/src/UploadImage.vue";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
import { useCommonStore } from "~/store/common";
import { useUserStore } from "~/store/user";
const { t } = useI18n();
const commonStore = useCommonStore();
const userStore = useUserStore();
// D1146：匿名展示名由后端按统一规则下发（UserClientDetailVO.anonymousUsername），前端不复制掩码规则
const anonymousPreview = computed(() => userStore.userInfo?.anonymousUsername || "");
// D1146：商品评价总开关；匿名开关只在总开关开启时才有意义
const commentEnabled = computed(() => commonStore.productCommentEnabled === 1);
const anonymousEnabled = computed(() => commentEnabled.value && commonStore.anonymousCommentEnabled === 1);
definePageMeta({
    middleware: "auth"
});
const price = ref(999);
const value = ref<number>(0);
// i18n-ignore-next-line 数组元素即翻译 key，紧随其后的 map 已逐个 t()
const desc = computed<string[]>(() => ["1分", "2分", "3分", "4分", "5分"].map((rank) => t(rank)));
const loading = ref<boolean>(true);
// i18n-ignore-next-line 默认评价标签作为 key 存入 allCommentTag，模板里逐个 $t(lb)
const labelList = ref(["送货快", "质量很好", "服务周到", "正品", "很划算", "包装仔细", "价格实惠", "还可以", "相当好"]);
const formState = ref<any[]>([]);
onMounted(() => {
    // D1146：关闭时不取数，页面直接呈现空态
    if (!commentEnabled.value) {
        loading.value = false;
        return;
    }
    // 匿名开关开启时才需要展示名做预览；getUserInfo 内部按 userInfoLoaded 去重，不会重复请求
    if (anonymousEnabled.value) {
        userStore.getUserInfo();
    }
    // 获取详情数据
    fetchCommentData();
});

const fetchCommentData = async () => {
    try {
        formState.value = [];
        const result = await getCommentData({ id: router.currentRoute.value.query.id });
        result.items.map((data: any) => {
            data.allCommentTag = JSON.parse(JSON.stringify(labelList.value)); // 使用深拷贝来创建独立的数组
            Object.assign(
                data,
                {
                    commentRank: 0,
                    content: "",
                    showPics: [],
                    commentTag: [],
                    isAdd: false,
                    rankVerification: true,
                    contentVerification: true,
                    // D1146：默认实名；已评价的条目会被后面的 commentInfo 覆盖成历史真实值
                    isAnonymous: 0,
                    isCommentInfo: data.commentInfo ? true : false
                },
                data.commentInfo
            );
        });
        Object.assign(formState.value, result.items);
        const allAreTrue = formState.value.every((obj: any) => obj.isShowed === 1);
        if (allAreTrue) {
            navigateTo("/member/comment/list?orderType=2");
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const router = useRouter();
const tabInput = ref("");
const isShowadd = (index: number) => {
    formState.value[index].isAdd = !formState.value[index].isAdd;
};
const tabAdd = (index: number) => {
    if (tabInput.value !== "") {
        const newCommentTag = JSON.parse(JSON.stringify(formState.value[index].allCommentTag));
        newCommentTag.push(tabInput.value);
        formState.value[index].allCommentTag = newCommentTag;
        checkTag(index, tabInput.value);
    }
    tabInput.value = "";
    formState.value[index].isAdd = false;
};
const checkTag = (index: number, label: string) => {
    let arr = formState.value[index].commentTag;
    if (arr.includes(label)) {
        formState.value[index].commentTag = arr.filter((item: any) => item !== label);
    } else {
        arr.push(label);
    }
};
const onSubmit = async (item: any, index: number) => {
    if (item.commentRank < 1) {
        item.rankVerification = false;
        return;
    }

    if (item.content == "") {
        item.contentVerification = false;
        return;
    }

    if (item.content.length < 10) {
        message.error(t("购买心得最少10个字"));
        return;
    }

    try {
        const data = {
            productId: item.productId,
            orderId: item.orderId,
            orderItemId: item.itemId,
            shopId: item.shopId,
            commentRank: item.commentRank,
            commentTag: item.commentTag,
            content: item.content,
            showPics: item.showPics,
            // D1146：匿名开关关闭时恒传 0，后端也会再兜一层
            isAnonymous: anonymousEnabled.value && item.isAnonymous ? 1 : 0
        };
        const result = await updateCommentData(data);
        item.rankVerification = true;
        item.contentVerification = true;
        message.success(t("评价成功"));
        fetchCommentData();
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.white-card {
    background: #ffffff;
    margin-bottom: 20px;
    display: flex;
    width: 100%;
    box-sizing: border-box;
    font-size: 14px;
    padding: 10px 18px;
}

.collection {
    width: 100%;

    .nav {
        background: #ffffff;
        margin-bottom: 20px;
        display: flex;
        width: 100%;
        box-sizing: border-box;
        padding: 10px 18px;
        align-items: center;

        .collection-list-tag {
            border: 0;
            flex: 1;
            display: flex;
            flex-direction: row;
            gap: 20px;
            line-height: 32px;
            font-size: 14px;
            cursor: pointer;
        }
    }
}

.product-left {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    width: 280px;
    text-align: center;
    line-height: 3;
    padding: 30px;
    border-right: 1px solid #e8e8e8;
    box-sizing: border-box;
    :deep(.el-rate .el-rate__text) {
        color: #666 !important;
    }
    img {
        width: 100px;
        height: 100px;
    }
}

.product-right {
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    width: 600px;
    padding: 20px;
    gap: 20px;

    & > div {
        display: flex;
        flex-direction: column;
    }

    // D1146：匿名勾选与掩码名同行展示，对齐 H5 写法，故覆盖上面的纵向排列
    & > div.anonymous-row {
        flex-direction: row;
        align-items: center;
        gap: 8px;
        font-size: 12px;

        :deep(.el-checkbox__label) {
            font-size: 12px;
            color: #999999;
        }
        .anonymous-name {
            color: #666666;
        }
    }

    .lab-text {
        min-width: 80px;
        line-height: 2;
    }
    .tips {
        display: block;
        background: none repeat scroll 0 0 var(--tag-bg);
        border: 1px solid var(--tag-text);
        color: var(--tag-text);
        line-height: 24px;
        clear: both;
        padding: 2px 6px;
    }
}

.label-list {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;

    .label-item {
        padding: 6px;
        cursor: pointer;
        height: 16px;
        white-space: nowrap;
        display: flex;
        gap: 8px;
    }
    .label-item-txt {
        border: 1px solid #eee;
        line-height: 18px;
        background: #eee;
        margin-bottom: 5px;
        margin-right: 5px;
        height: 18px;
        padding: 6px 9px;
        white-space: nowrap;
    }
    .label-edit {
        padding: 6px;
        cursor: pointer;
        height: 18px;
        white-space: nowrap;
        display: flex;
        gap: 8px;
        border: 1px solid #ccc;
        div {
            width: 16px;
            height: 18px;
            background: url("/assets/images/user/cicon.png") no-repeat scroll 4px -58px #fff;
        }
    }
    .label-edit-input {
        display: flex;
        align-items: center;
        width: 150px;
        border: 2px solid var(--general);
        p {
            width: 70px;
            height: 26px;
            line-height: 26px;
            cursor: pointer;
            text-align: center;
            color: var(--main-text);
            background-color: var(--general);
        }
        :deep(.el-input__wrapper) {
            height: 24px;
            box-shadow: none !important;
        }
    }
}
</style>
