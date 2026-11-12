<template>
    <div style="background-color: #f5f6fa">
        <CommonHeader :title="articleName ? articleName : $t('帮助中心')"></CommonHeader>
        <CommonPageHeader></CommonPageHeader>
        <CommonPageContainer class="news-content">
            <!-- <div class="breadcrumb-menu">
                <el-icon color="#b9b9b9">
                    <HomeFilled />
                </el-icon>
                <el-breadcrumb :separator-icon="ArrowRight">
                    <el-breadcrumb-item :to="{ path: '/' }">{{ $t("首页") }}</el-breadcrumb-item>
                    <el-breadcrumb-item :to="{ path: '/article/issue/index' }">{{ $t("帮助中心") }}</el-breadcrumb-item>
                    <el-breadcrumb-item v-if="articleName"
                        ><span class="breadcrumb-title">{{ articleName }}</span></el-breadcrumb-item
                    >
                </el-breadcrumb>
            </div> -->
            <div data-content-rev="MjI4Mm53Z3U" class="content">
                <div class="box left">
                    <IssueLeftMenu v-model:dataOver="dataOver" v-model:leftMenuList="leftMenuList"></IssueLeftMenu>
                </div>
                <div class="box right">
                    <!-- <div class="right-title">{{ $t("新手指南") }}</div>
                    <div class="issue_main_bg">
                        <template v-for="item in leftMenuList.slice(0, 5)">
                            <NuxtLink target="_blank" :to="'/article/issue/info?categoryId=' + item.categorySn" class="xszn-div">
                                <span class="text">{{ item.articleCategoryName }}</span>
                            </NuxtLink>
                        </template>
                    </div> -->
                    <div class="right-title zzfw">{{ $t("自助服务") }}</div>
                    <div class="zzfw-menu">
                        <NuxtLink v-for="item in zzfwList" :to="item.url" class="zzfw-menu-item" target="_blank">
                            <div :class="item.ico" class="left_info"></div>
                            <div class="right-info">
                                <ul>
                                    <li class="big">{{ $t(item.title) }}</li>
                                    <li class="min">{{ $t(item.subTitle) }}</li>
                                </ul>
                            </div>
                        </NuxtLink>
                    </div>
                    <div class="help_division" />
                    <template v-if="hotState.length > 0">
                        <div class="right-title rmwt">{{ $t("热门问题") }}</div>
                        <div class="rmwt-menu">
                            <div v-for="(item, index) in hotState" class="rmwt-menu-item">
                                <span class="help_hot_index">{{ index + 1 }}</span>
                                <NuxtLink target="_blank" :to="'/article/issue/info?articleId=' + item.articleId" class="help_hot_text">{{
                                    item.articleTitle
                                }}</NuxtLink>
                            </div>
                        </div>
                        <div class="help_division" />
                    </template>
                    <template v-if="leftMenuList.length > 0">
                        <div class="right-title rmwt">{{ $t("帮助分类") }}</div>
                        <div class="bzfl">
                            <div class="bzfl-menu">
                                <template v-for="item in leftMenuList.slice(0, 5)">
                                    <div :class="nowTab === item.categorySn ? 'check-tab' : ''" class="bzfl-menu-item" @mouseover="checkTab(item)">
                                        <span class="text">{{ item.articleCategoryName }}</span>
                                    </div>
                                </template>
                            </div>
                            <div class="bzfl-list">
                                <div v-for="(item, index) in nowBzflList" class="bzfl-list-item">
                                    <span class="help_hot_index"></span>
                                    <NuxtLink target="_blank" :to="'/article/issue/info?articleId=' + item.articleId">{{ item.articleTitle }}</NuxtLink>
                                </div>
                            </div>
                        </div>
                    </template>
                </div>
            </div>
        </CommonPageContainer>
        <CommonPageFooter></CommonPageFooter>
    </div>
</template>
<script lang="ts" setup>
/**
 * 帮助中心首页
 * - 展示新手指南、自助服务入口、热门问题、帮助分类
 * - 左侧显示帮助分类菜单，支持鼠标悬停切换分类
 * - 自助服务包含物流、退货、售后、账号安全等快捷入口
 */
import { ArrowRight } from "@element-plus/icons-vue";
import { ref } from "vue";
import IssueLeftMenu from "~/pages/article/issue/src/IssueLeftMenu.vue";
import type { CategoryFilterState } from "~/types/article/category";
import type { ArticleFilterState } from "~/types/article/article";
import { getArticleList } from "~/api/article/article";
import { useCommonStore } from "~/store/common";

const leftMenuList = ref<CategoryFilterState[]>([]);

const commonStore = useCommonStore();

const zzfwList = ref([
    { title: "发货物流", subTitle: "查看订单物流信息", url: "/member/order/list/", ico: "selfS_ones" },
    { title: "退货/退款", subTitle: "申请退货/退款", url: "/member/return/list", ico: "selfS_twos" },
    { title: "售后进度", subTitle: "查看售后处理进度", url: "/member/return/log", ico: "selfS_threes" },
    { title: "手机号修改", subTitle: "修改绑定的手机号", url: "/member/security/info", ico: "selfS_fours" },
    { title: "订单评价", subTitle: "查看订单详情、确认收货、评价订单等", url: "/member/comment/list", ico: "selfS_fives" },
    { title: "密码修改", subTitle: "修改账户密码", url: "/member/security/info", ico: "selfS_sixs" },
    { title: `${commonStore.integralName}查询`, subTitle: `查看账户${commonStore.integralName}、使用明细等`, url: "/member/point/list", ico: "selfS_sevens" },
    { title: "优惠券查询", subTitle: "查看未使用、已使用、已过期的优惠券信息等", url: "/member/coupon/list", ico: "selfS_eights" }
    // D1146：评价功能关闭时移除「订单评价」入口。与同数组的 integralName 一样在 setup 期求值，
    // 配置在 app.vue 渲染前已 await 就绪，故无需响应式
].filter((item) => item.url !== "/member/comment/list" || commonStore.productCommentEnabled === 1));

const hotState = ref(<ArticleFilterState[]>[]);
const loading = ref<boolean>(true);

const loadFilter = async () => {
    loading.value = true;
    try {
        let temp: any = {
            categorySn: "rmwt",
            page: 1,
            size: 999
        };
        const result = await getArticleList({ ...temp });
        console.log("帮助", result);
        hotState.value = result.records;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
loadFilter();

const articleName = ref<string>("");
const nowBzflList = ref<ArticleFilterState[]>([]);
const dataOver = ref(false);
const nowTab = ref("xssl");

const { t } = useI18n();
useSeo({
    title: computed(() => t("帮助中心")),
    canonical: "/article/issue/index"
});

const checkTab = (value: any) => {
    leftMenuList.value.forEach((item: any) => {
        if (item.articleCategoryId == value.articleCategoryId) {
            item.check = true;
            nowTab.value = item.categorySn;
            nowBzflList.value.length = 0;
            nowBzflList.value.push(...item.articles);
        } else {
            item.check = false;
        }
    });
};
watch(
    () => dataOver,
    () => {
        if (dataOver) {
            checkTab(leftMenuList.value[0]);
        }
    },
    { deep: true }
);
</script>
<style lang="less" scoped>
.news-content {
    margin-top: 32px;
    background-color: #f5f6fa;

    .breadcrumb-menu {
        display: flex;
        gap: 5px;

        .breadcrumb-title {
            display: inline-block;
            white-space: nowrap;
            width: 300px;
            overflow: hidden;
            text-overflow: ellipsis;
        }
    }

    .content {
        display: flex;
        margin-top: 20px;
        gap: 16px;

        .box {
            box-sizing: border-box;
            border-radius: 8px;
        }

        .left {
            width: 240px;
        }

        .right {
            flex: 1;
            border: 1px solid #eee;
            box-sizing: border-box;
            padding: 20px;
            background-color: #fff;
            border-radius: 8px;
            overflow: hidden;

            .right-title {
                color: #333333;
                font-size: 18px;
                font-weight: 700;
            }

            .issue_main_bg {
                margin-top: 40px;
                height: 80px;
                display: flex;
                align-items: center;
                justify-content: space-around;

                .xszn-div {
                    color: #333333;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    font-size: 16px;
                    font-weight: 700;
                    height: 80px;
                    width: 80px;
                    text-align: center;
                    background-color: white; /* 背景颜色设置为白色 */
                    border-radius: 50%; /* 使用百分比来创建完美的圆形 */
                    box-shadow: 0 0 10px rgba(0, 0, 0, 0.1); /* 可选：添加轻微的阴影以增强视觉效果 */

                    .text {
                        width: 40px;
                    }
                }
            }

            .zzfw {
            }

            .zzfw-menu {
                display: flex;
                flex-wrap: wrap; /* 允许内容换行 */
                margin-top: 16px;

                .zzfw-menu-item {
                    flex: 0 0 25%; /* flex-grow: 0, flex-shrink: 0, flex-basis: 25% */
                    box-sizing: border-box; /* 包括边框和填充在元素的总宽度和高度中 */
                    padding: 10px; /* 内填充 */
                    display: flex;
                    height: 100px; /* 高度设置 */
                    gap: 10px;
                    align-items: center;

                    .left_info {
                        min-width: 65px;

                        min-height: 65px;
                    }

                    .selfS_ones {
                        background: url("/assets/images/help/selfS_ones.png") no-repeat scroll center;
                    }

                    .selfS_twos {
                        background: url("/assets/images/help/selfS_twos.png") no-repeat scroll center;
                    }

                    .selfS_threes {
                        background: url("/assets/images/help/selfS_threes.png") no-repeat scroll center;
                    }

                    .selfS_fours {
                        background: url("/assets/images/help/selfS_fours.png") no-repeat scroll center;
                    }

                    .selfS_fives {
                        background: url("/assets/images/help/selfS_fives.png") no-repeat scroll center;
                    }

                    .selfS_sixs {
                        background: url("/assets/images/help/selfS_sixs.png") no-repeat scroll center;
                    }

                    .selfS_sevens {
                        background: url("/assets/images/help/selfS_sevens.png") no-repeat scroll center;
                    }

                    .selfS_eights {
                        background: url("/assets/images/help/selfS_eights.png") no-repeat scroll center;
                    }

                    .right-info {
                        font-size: 12px;
                        line-height: 2;

                        .big {
                            color: #333333;
                        }

                        .min {
                            color: #a8a8a8;
                        }
                    }
                }
            }

            .rmwt {
                margin-top: 10px;
            }

            .rmwt-menu {
                display: flex;
                flex-wrap: wrap; /* 允许内容换行 */

                .rmwt-menu-item {
                    flex: 0 0 50%; /* flex-grow: 0, flex-shrink: 0, flex-basis: 25% */
                    box-sizing: border-box; /* 包括边框和填充在元素的总宽度和高度中 */
                    padding: 10px; /* 内填充 */
                    display: flex;
                    height: 50px; /* 高度设置 */
                    gap: 10px;
                    align-items: center;

                    .help_hot_index {
                        background-color: var(--tag-bg);
                        border-radius: 3px;
                        color: var(--tag-text);
                        display: inline-block;
                        font-size: 12px;
                        height: 14px;
                        line-height: 13px;
                        text-align: center;
                        width: 19px;
                    }

                    .help_hot_text {
                        font-size: 14px;
                        color: #333333;
                        width: 400px;
                        display: inline-block;
                        white-space: nowrap;
                        overflow: hidden;
                        text-overflow: ellipsis;
                    }
                }
            }

            .bzfl {
                display: flex;
                flex-direction: column;

                .bzfl-menu {
                    display: flex;
                    align-items: center;
                    justify-content: space-around;
                    margin-top: 10px;
                    gap: 10px;

                    .bzfl-menu-item {
                        color: #585858;
                        cursor: pointer; /* 设置鼠标指针为默认指针 */
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        font-size: 16px;
                        font-weight: 700;
                        height: 45px;
                        width: auto;
                        text-align: center;
                        background-color: #e9e9e9; /* 背景颜色设置为白色 */
                        padding: 0 30px;
                    }

                    .check-tab {
                        background-color: var(--tag-bg);
                        color: var(--tag-text);
                        position: relative;
                    }

                    .check-tab::after {
                        content: "";
                        position: absolute;
                        bottom: -10px; /* 调整三角形的位置 */
                        left: 50%; /* 位于 div 的正中间 */
                        margin-left: -5px; /* 调整三角形的位置 */
                        border-width: 5px; /* 三角形大小 */
                        border-style: solid;
                        border-color: transparent transparent var(--tag-bg) transparent; /* 通过 border-color 设置三角形颜色 */
                        transform: rotate(180deg); /* 将三角形旋转180度 */
                    }
                }

                .bzfl-list {
                    display: flex;
                    flex-wrap: wrap; /* 允许内容换行 */
                    margin: 20px 10px;

                    .bzfl-list-item {
                        width: 100%;
                        flex: 0 0 50%; /* flex-grow: 0, flex-shrink: 0, flex-basis: 25% */
                        box-sizing: border-box; /* 包括边框和填充在元素的总宽度和高度中 */
                        padding: 10px; /* 内填充 */
                        display: flex;
                        font-size: 14px;
                        align-items: center;
                        color: #585858;
                        height: 40px;
                        line-height: 40px;
                        position: relative;
                    }

                    .bzfl-list-item::before {
                        content: "";
                        position: absolute;
                        top: 50%; /* 使小三角形垂直居中 */
                        left: 0; /* 放置在容器的左侧 */
                        transform: translateY(-50%) rotate(180deg); /* 旋转180度 */
                        width: 0;
                        height: 0;
                        border-top: 4px solid transparent; /* 设置三角形的样式 */
                        border-bottom: 4px solid transparent;
                        border-right: 4px solid var(--icon); /* 设置三角形的颜色 */
                    }
                }
            }
        }
    }

    .help_division {
        border: none;
        border-top: 1px dashed #e5e5e5; /* 设置虚线样式，可以根据需要调整颜色和粗细 */
    }
}
:deep(.el-breadcrumb) {
    font-size: 12px;
    .el-breadcrumb__inner a,
    .el-breadcrumb__inner.is-link {
        font-weight: normal;
        color: #606266;
    }
}
</style>
