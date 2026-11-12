<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>

    <CommonHeader title="申请详情"></CommonHeader>
    <TopJumpBar tltle="申请详情"></TopJumpBar>

    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="title-or-tabs">
                <div class="tag-and-input">
                    <div class="tig-tabs">
                        <MemberTopMenu :menuList="menuList"></MemberTopMenu>
                    </div>
                </div>
            </div>
            <template v-if="formState.returnGoodsTip">
                <div class="title-or-tabs">
                    <div>{{ formState.returnGoodsTip }}</div>
                </div>
            </template>

            <div v-if="!loading" class="log-return-content">
                <div class="return-status-header">
                    <div class="status-title">{{ statusTitle }}</div>
                    <div v-if="formState.canCancel || showReapply" class="header-actions">
                        <el-button v-if="formState.canCancel" class="cancel-button" @click="revocation(Number(formState.aftersaleId))">
                            {{ $t("撤销") }}
                        </el-button>
                        <el-button v-if="showReapply"  class="reapply-button" @click="goReapply">
                            {{ $t("重新申请") }}
                        </el-button>
                    </div>
                    <div v-if="showAutoAgreeCountdown || statusSubText" class="status-sub">
                        <template v-if="showAutoAgreeCountdown">
                            {{ $t("订单将在") }} <span class="countdown-text">{{ autoAgreeCountdownText }}</span> {{ $t("后自动同意您的退款申请") }}
                        </template>
                        <template v-else>{{ statusSubText }}</template>
                    </div>
                </div>
                <div v-if="formState.status != 6" class="file-card">
                    <el-form ref="formRef" :label-width="100" :model="formState" class="form-body" label-suffix="：" v-if="formState.status != 7">
                        <el-form-item :label="$t('问题补充')" prop="logInfo">
                            <el-input v-model="formState.logInfo" :row="4" clearable :placeholder="$t('请输入问题补充')" type="textarea" />
                        </el-form-item>
                        <template v-if="formState.status === 4">
                            <el-form-item :label="$t('回寄地址')" prop="returnAddress">
                                <el-input v-model="formState.returnAddress" readonly clearable type="textarea" />
                            </el-form-item>
                            <el-form-item :rules="[{ required: true, message: $t('快递公司不能为空!') }]" :label="$t('快递公司')" prop="logisticsName">
                                <el-input v-model="formState.logisticsName" clearable :placeholder="$t('请输入快递公司')" />
                            </el-form-item>
                            <el-form-item :rules="[{ required: true, message: $t('快递单号不能为空!') }]" :label="$t('快递单号')" prop="trackingNo">
                                <el-input v-model="formState.trackingNo" clearable :placeholder="$t('请输入快递单号')" type="textarea" />
                            </el-form-item>
                        </template>
                        <el-form-item :label="$t('上传凭证')" prop="returnPic">
                            <UploadImage v-model:uploadList="formState.returnPic" :fileTypes="['jpeg', 'png']" :limit="5"></UploadImage>
                        </el-form-item>
                        <el-form-item label="">
                            <el-button type="primary" @click="onSubmit">{{ $t("提交") }}</el-button>
                        </el-form-item>
                    </el-form>
                </div>
                <div class="info-card">
                    <table>
                        <thead class="table-header">
                            <tr>
                                <th class="header-cell">{{ $t("商品名称") }}</th>
                                <th class="header-cell" style="width: 60px">{{ $t("商品价格") }}</th>
                                <th class="header-cell" style="width: 60px">{{ $t("购买数量") }}</th>
                                <th class="header-cell" style="width: 60px">{{ $t("小计") }}</th>
                                <th class="header-cell" style="width: 60px">{{ $t("售后数量") }}</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-for="item in orderList">
                                <td>
                                    <NuxtLink
                                        :title="item.productName"
                                        :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })"
                                        class="nuxt"
                                        target="_blank"
                                    >
                                        <el-image
                                            :src="productImage(item)"
                                            class="pro_pic"
                                            loading="lazy"
                                            style="transition: opacity 0.2s linear; flex-shrink: 0"
                                            :alt="item.productName || ''"
                                        />
                                        <div class="product-info">
                                            <ul>
                                                <li>{{ $t("货号") }}：{{ item.productSn }}</li>
                                                <li class="product-name">{{ item.productName }}</li>
                                            </ul>
                                            <div v-if="item.skuValue" class="sku">{{ item.skuValue }}</div>
                                        </div>
                                    </NuxtLink>
                                </td>
                                <td>
                                    <FormatPrice v-model="item.price" :fontStyle="{ color: 'var(--price)' }" :showText="false"></FormatPrice>
                                </td>
                                <td>{{ item.quantity }}</td>
                                <td>
                                    <FormatPrice v-model="item.subtotal" :fontStyle="{ color: 'var(--price)' }" :showText="false"></FormatPrice>
                                </td>
                                <td>
                                    {{ item.number }}
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
                <div class="info-card">
                    <table>
                        <tbody>
                            <tr>
                                <td>{{ $t("买家问题描述") }}:</td>
                                <td>{{ formState.description }}</td>
                            </tr>
                            <tr>
                                <td>{{ $t("售后类型") }}:</td>
                                <td>{{ formState.aftersalesTypeName }}</td>
                            </tr>
                            <tr>
                                <td>{{ $t("退款金额") }}:</td>
                                <td><FormatPrice v-model="formState.refundAmount" :showText="false"></FormatPrice></td>
                            </tr>
                            <tr>
                                <td>{{ $t("售后原因") }}:</td>
                                <td>{{ formState.aftersaleReason }}</td>
                            </tr>
                            <tr v-if="formState.status === 4">
                                <td>{{ $t("退货地址") }}:</td>
                                <td>{{ formState.returnAddress }}</td>
                            </tr>
                            <tr>
                                <td>{{ $t("上传凭证") }}:</td>
                                <td>
                                    <div class="application-picture">
                                        <template v-for="item in formState.pics">
                                            <el-image
                                                :src="productImage(item)"
                                                fit="cover"
                                                style="width: 80px; height: 80px"
                                                :preview-src-list="[imageFormat(item.picUrl)]"
                                                :initial-index="0"
                                                :alt="item.productName || ''"
                                            />
                                        </template>
                                    </div>
                                </td>
                            </tr>
                            <template v-if="formState.refund">
                                <tr v-if="formState.refund.isOnline > 0">
                                    <td>{{ $t("线上退款金额") }}:</td>
                                    <td>
                                        <FormatPrice v-model="formState.refund.onlineBalance" :showText="false"></FormatPrice>
                                    </td>
                                </tr>
                                <tr v-if="formState.refund.isOffline > 0">
                                    <td>{{ $t("线下退款金额") }}:</td>
                                    <td>
                                        <FormatPrice v-model="formState.refund.offlineBalance" :showText="false"></FormatPrice>
                                    </td>
                                </tr>
                                <tr v-if="formState.refund.isReceive > 0">
                                    <td>{{ $t("余额退款金额") }}:</td>
                                    <td>
                                        <FormatPrice v-model="formState.refund.refundBalance" :showText="false"></FormatPrice>
                                    </td>
                                </tr>
                            </template>
                        </tbody>
                    </table>
                </div>
                <div class="info-card">
                    <table>
                        <thead class="table-header">
                            <tr>
                                <th class="header-cell" style="width: 120px">{{ $t("发言人") }}</th>
                                <th class="header-cell" style="width: 160px">{{ $t("时间") }}</th>
                                <th class="header-cell">{{ $t("信息") }}</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-for="item in formState.aftersalesLog">
                                <td>{{ item.userName || item.adminName }}</td>
                                <td>{{ item.addTime }}</td>
                                <td>
                                    <div class="bt-image">
                                        <div>{{ item.logInfo }}</div>
                                        <template v-for="img in item.returnPic">
                                            <el-image
                                                :src="imageFormat(img.picUrl)"
                                                fit="cover"
                                                class="bt-img"
                                                loading="lazy"
                                                style="transition: opacity 0.2s linear"
                                                :preview-src-list="[imageFormat(img.picUrl)]"
                                                :initial-index="0"
                                            />
                                        </template>
                                    </div>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
            <div v-else v-loading="loading" class="log-return-content"></div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
    <el-dialog v-model="dialogVisible">
        <div class="image">
            <el-image :src="imageFormat(dialogImageUrl)" alt="" fit="cover" />
        </div>
    </el-dialog>
</template>
<script lang="ts" setup>
/**
 * 售后详情页
 * - 展示售后申请详情、处理进度
 * - 支持填写物流信息（退货时）
 * - 支持取消售后申请
 */
import { onMounted, onUnmounted, reactive, ref, computed } from "vue";
import { useRouter } from "vue-router";
import { message, Modal } from "ant-design-vue";
import type { AfterSalesRecordFormState } from "~/types/user/afterSalesRecord";
import { cancelAfterSalesRecord, getAfterSalesRecord, updateAfterSalesRecord } from "~/api/user/afterSalesRecord";
import UploadImage from "~/components/form/src/UploadImage.vue";
import { urlFormat } from "~/utils/format";
import type { AfterSalesFormState } from "~/types/user/afterSales";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
definePageMeta({
    middleware: "auth"
});
const { t } = useI18n();
const dialogImageUrl = ref("");
const dialogVisible = ref(false);
const openImage = (url: string) => {
    dialogImageUrl.value = url;
    dialogVisible.value = true;
};
const orderList = ref(<AfterSalesFormState[]>[]);
const router = useRouter();
const formState = ref<AfterSalesRecordFormState>({
    stepStatus: {},
    picsList: [],
    returnPic: [],
    pics: []
});
const loading = ref<boolean>(true);

// ============ 顶部状态头（标题 + 副行/倒计时） ============
const statusTitle = computed(() => String((formState.value as any)?.statusName || ""));
const nowTs = ref(Date.now());
const autoAgreeEndTime = computed(() => String((formState.value as any)?.autoAgreeEndTime || ""));
const autoAgreeEndTimeMs = computed(() => {
    const raw = autoAgreeEndTime.value;
    if (!raw) return 0;
    const ms = Date.parse(raw.replace(/-/g, "/"));
    return Number.isFinite(ms) ? ms : 0;
});
const isProcessing = computed(() => [0, 1, 21].includes(Number((formState.value as any)?.status ?? -1)));
const showAutoAgreeCountdown = computed(() => isProcessing.value && autoAgreeEndTimeMs.value > nowTs.value);
const autoAgreeCountdownText = computed(() => {
    const diff = autoAgreeEndTimeMs.value - nowTs.value;
    if (diff <= 0) return "";
    const totalSec = Math.floor(diff / 1000);
    const days = Math.floor(totalSec / 86400);
    const h = Math.floor((totalSec % 86400) / 3600);
    const m = Math.floor((totalSec % 3600) / 60);
    const s = totalSec % 60;
    const pad = (n: number) => (n < 10 ? `0${n}` : `${n}`);
    return days > 0 ? `${days}${t("天")}${pad(h)}:${pad(m)}:${pad(s)}` : `${pad(h)}:${pad(m)}:${pad(s)}`;
});
const statusSubText = computed(() => {
    if (showAutoAgreeCountdown.value) return "";
    const desc = String((formState.value as any)?.statusDesc || "").trim();
    if (desc) return desc;
    const status = Number((formState.value as any)?.status ?? -1);
    if ([0, 1, 21].includes(status)) return t("您的售后申请正在处理，请耐心等待");
    if ([2, 4, 5, 22].includes(status)) return t("您的退款申请已提交，请耐心等待财务处理");
    if (status === 6) return t("您的退款申请已处理完成，款项将按原支付路径退回");
    return "";
});

// ============ 被拒后的重新申请入口 ============
/** 重新申请功能开关：后端接口对接完成后再开启 */
const ENABLE_REAPPLY = false;
const REJECTED_LOG_KEYWORDS = ["财务审核不通过", "审核未通过", "拒绝", "驳回", "拒收"];
/** 最新一条售后日志 */
const latestLogInfo = computed(() => {
    const logs = formState.value.aftersalesLog || [];
    return String(logs[logs.length - 1]?.logInfo || "");
});
/** 售后被拒绝：状态为审核未通过，或状态文案、最新处理结果为不通过 */
const showReapply = computed(() => {
    if (!ENABLE_REAPPLY) return false;
    if (Number(formState.value.status ?? -1) === 3) return true;
    if (["拒绝", "未通过", "失败", "驳回"].some((keyword) => statusTitle.value.includes(keyword))) return true;
    return REJECTED_LOG_KEYWORDS.some((keyword) => latestLogInfo.value.includes(keyword));
});
/** 重新申请的售后申请页地址，缺少订单信息时回退到售后申请列表 */
const reapplyPath = computed(() => {
    const orderId = Number((formState.value as any)?.orderId || orderList.value?.[0]?.orderId || 0);
    if (!orderId) return "/member/return/list";
    const orderItemId = orderList.value.length === 1 ? Number((orderList.value[0] as any)?.orderItemId || 0) : 0;
    return orderItemId ? `/member/return/Info?orderId=${orderId}&itemId=${orderItemId}` : `/member/return/Info?orderId=${orderId}`;
});
const goReapply = () => {
    navigateTo(reapplyPath.value);
};

let countdownTimer: ReturnType<typeof setInterval> | null = null;
let refetchedOnZero = false;
onMounted(() => {
    fetchAfterSalesRecord();
    countdownTimer = setInterval(() => {
        if (!isProcessing.value || autoAgreeEndTimeMs.value <= 0) return;
        nowTs.value = Date.now();
        if (autoAgreeEndTimeMs.value - nowTs.value <= 0 && !refetchedOnZero) {
            refetchedOnZero = true;
            fetchAfterSalesRecord();
        }
    }, 1000);
});
onUnmounted(() => {
    if (countdownTimer) {
        clearInterval(countdownTimer);
        countdownTimer = null;
    }
});
const fetchAfterSalesRecord = async () => {
    try {
        loading.value = true;
        const result = await getAfterSalesRecord({ id: router.currentRoute.value.query.id });
        console.log(result);

        Object.assign(formState.value, result);
        orderList.value = result.aftersalesItems ? result.aftersalesItems : [];
        orderList.value?.forEach((data: any) => {
            data.subtotal = data.quantity * data.price;
        });
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

const formRef = shallowRef();
const onSubmit = async () => {
    await formRef.value.validate();
    try {
        // formState.value.returnPic.forEach((item: any) => {
        //     formState.value.returnPic.push(item.picUrl);
        // });
        const result = await updateAfterSalesRecord({ id: formState.value.aftersaleId, ...formState.value });
        message.success(t("提交成功"));
        await formRef.value.resetFields();
        await fetchAfterSalesRecord();
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
const revocation = async (id: number) => {
    Modal.confirm({
        title: t("确认要撤销申请吗"),
        cancelText: t("取消"),
        okText: t("确定"),
        maskClosable: true,
        centered: true,
        onOk: async () => {
            try {
                const result = await cancelAfterSalesRecord({ aftersaleId: id });
                message.success(t("撤销成功"));
                navigateTo("/member/return/log");
            } catch (error: any) {
                message.error(error.message);
            } finally {
            }
        }
    });
};

const splitStr = (str: string) => {
    return str.split("?")[0];
};
const menuList = reactive<any>([
    { value: "售后申请", url: "/member/return/list", size: 0 },
    { value: "申请记录", url: "/member/return/logInfo", size: 0 }
]);
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.log-return-content {
    .return-status-header {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: center;
        column-gap: 20px;
        margin-bottom: 20px;

        .status-title {
            grid-column: 1;
            grid-row: 1;
            font-size: 20px;
            font-weight: 700;
            color: #1a1a1a;
            line-height: 1.2;
        }

        .status-sub {
            grid-column: 1;
            grid-row: 2;
            margin-top: 8px;
            font-size: 13px;
            color: #8b8e97;

            .countdown-text {
                color: #cc0c1c;
                font-weight: 600;
            }
        }

        .header-actions {
            grid-column: 2;
            grid-row: 1;
            display: flex;
            align-items: center;
            gap: 12px;
        }
    }

    background: #fff;
    border: 0;
    padding: 0 0 20px;
    box-sizing: border-box;
    min-height: 340px;

    .file-card {
        margin: 0 0 30px;
        padding: 20px;
        background-color: #fafafa;

        .form-body {
            width: 600px;
            margin-top: 20px;

            .a-btn {
                color: #005ea7;
                margin-left: 6px;
            }
        }
    }

    .info-card {
        margin: 0 0 30px;
        box-sizing: border-box;
        border-collapse: collapse; /* 边框合并为单一边框 */

        .table-header {
            background-color: #f5f5f5;
            border: 1px solid #ddd;
        }

        .header-cell {
            border: 1px solid #ddd;
            border-right: none;
            padding: 8px 10px;
            box-sizing: border-box;
            text-align: left;
        }

        /* 设置单元格样式 */

        td {
            border: 1px solid #e8e8e8; /* 单元格边框颜色 */
            padding: 10px; /* 单元格内边距 */
        }

        /* 设置左侧列宽度 */

        td:first-child {
            width: 120px; /* 左侧列宽度 */
        }
    }

    .application-picture {
        display: flex;
        gap: 8px;
        cursor: pointer;
    }
}

.info-card {
    margin: 0 0 30px;
    box-sizing: border-box;
    border-collapse: collapse; /* 边框合并为单一边框 */

    .table-header {
        background-color: #f5f5f5;
    }

    .header-cell {
        border: 1px solid #ddd;
        padding: 8px 10px;
        box-sizing: border-box;
        text-align: left;
    }

    /* 设置单元格样式 */

    td {
        border: 1px solid #e8e8e8; /* 单元格边框颜色 */
        padding: 10px; /* 单元格内边距 */

        .nuxt {
            display: flex;
            gap: 10px;

            .pro_pic {
                width: 40px;
                height: 40px;
            }

            .product-info {
                flex: 1;
                min-width: 0;
            }

            .product-name {
                display: -webkit-box;
                -webkit-box-orient: vertical;
                -webkit-line-clamp: 2;
                line-clamp: 2;
                overflow: hidden;
                text-overflow: ellipsis;
                line-height: 20px;
                max-height: 40px;
                word-break: break-word;
            }

            .sku {
                margin-top: 4px;
                font-size: 12px;
                line-height: 20px;
                color: #999;
                white-space: nowrap;
                overflow: hidden;
                text-overflow: ellipsis;
            }
        }
    }
}

.image {
    width: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
}

.bt-image {
    display: flex;
    flex-wrap: wrap;
    flex-direction: row;
    align-items: center;
    width: 100%;
    gap: 8px;

    .bt-img {
        width: 80px;
        height: 80px;
        cursor: pointer;
    }
}
</style>
