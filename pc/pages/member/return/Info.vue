<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>

    <CommonHeader :title="$t('售后申请')"></CommonHeader>
    <TopJumpBar :tltle="$t('售后申请')"></TopJumpBar>

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
            <div v-if="!loading" class="log-return-content">
                <div class="info-card">
                    <table>
                        <thead class="table-header">
                            <tr>
                                <th class="header-cell">{{ $t("商品名称") }}</th>
                                <th class="header-cell" style="width: 90px">{{ $t("商品价格") }}</th>
                                <th class="header-cell" style="width: 90px">{{ $t("购买数量") }}</th>
                                <th class="header-cell" style="width: 90px">{{ $t("小计") }}</th>
                                <th class="header-cell" style="width: 90px">{{ $t("可售后数量") }}</th>
                                <th class="header-cell" style="width: 90px">{{ $t("售后数量") }}</th>
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
                                            style="transition: opacity 0.2s linear"
                                            :alt="item.productName || ''"
                                        />
                                        <div class="text-ellipsis">
                                            <ul>
                                                <li>{{ $t("货号") }}：{{ item.productSn }}</li>
                                                <li>{{ item.productName }}</li>
                                                <li>
                                                    <template v-for="(sku, index) in item.skuData">
                                                        <span class="name">{{ $t(sku.name) }}：</span><span class="value">{{ $t(sku.value) }}</span
                                                        ><span v-if="index < item.skuData?.length - 1">；</span>
                                                    </template>
                                                </li>
                                            </ul>
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
                                <td>{{ item.canApplyQuantity }}</td>
                                <td>
                                    <el-input-number
                                        v-model.number="item.number"
                                        :max="item.canApplyQuantity"
                                        :min="0"
                                        clearable
                                        :disabled="item.isGift === 1"
                                        :placeholder="$t('请输入数量')"
                                    />
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
                <div class="file-card">
                    <el-form ref="formRef" label-width="auto" :model="formState" class="form-body" label-suffix="：">
                        <el-form-item :rules="[{ required: true, message: $t('请选择售后类型') }]" :label="$t('售后类型')" prop="aftersaleType">
                            <el-radio-group v-model="formState.aftersaleType">
                                <el-radio v-for="item in convertToObjectArray(aftersaleType ? aftersaleType : {})" :value="item.id">{{
                                    $t(item.name)
                                }}</el-radio>
                            </el-radio-group>
                        </el-form-item>
                        <el-form-item
                            v-if="afterSalesReasonEnabled"
                            :rules="[{ required: true, message: $t('请选择售后原因') }]"
                            :label="$t('售后原因')"
                            prop="aftersaleReason"
                        >
                            <el-select v-model="formState.aftersaleReason" :placeholder="$t('请选择')" style="width: 500px">
                                <el-option v-for="item in aftersaleReason" :key="item" :label="item" :value="item" />
                            </el-select>
                        </el-form-item>
                        <el-form-item :label="$t('预计退款金额')" prop="refundAmount">
                            <el-input disabled v-model="formState.refundAmount" clearable :placeholder="$t('请输入申请金额')" />
                        </el-form-item>
                        <el-form-item :label="$t('问题描述')" prop="description">
                            <el-input v-model="formState.description" :row="4" clearable :placeholder="$t('请输入问题描述')" type="textarea" />
                        </el-form-item>
                        <el-form-item :label="$t('上传凭证')">
                            <UploadImage
                                v-model:uploadList="formState.pics"
                                :fileTypes="['gif', 'jpeg', 'png', 'word', 'excel', 'txt', 'zip', 'ppt', 'pdf']"
                                :limit="5"
                            ></UploadImage>
                        </el-form-item>
                        <el-form-item label="">
                            <el-button type="primary" @click="onSubmit">{{ $t("提交") }}</el-button>
                        </el-form-item>
                    </el-form>
                </div>
            </div>
            <div v-else v-loading="loading" class="log-return-content"></div>
        </div>
    </CommonPageContainer>

    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 申请售后页面
 * - 选择售后类型（退货退款/仅退款/换货）
 * - 填写退款原因、金额、凭证图片
 * - 提交售后申请
 */
import { onMounted, reactive, ref } from "vue";
import { useRouter } from "vue-router";
import { getAfterSales, updateAfterSales, getAfterSalesConfig } from "~/api/user/afterSales";
import type { AfterSalesFormState } from "~/types/user/afterSales";
import { urlFormat } from "~/utils/format";
import UploadImage from "~/components/form/src/UploadImage.vue";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
const { t } = useI18n();
const orderList = ref(<AfterSalesFormState[]>[]);
const aftersaleReason: any = ref([]);
const afterSalesReasonEnabled = ref(false);
const aftersaleType: any = ref([]);
definePageMeta({
    middleware: "auth"
});
const formState = ref<AfterSalesFormState>({
    items: [],
    pics: [],
    refundAmount: 0,
    refundAmountMax: 0
});
const afterSaleType = computed(() => {
    let itemId = router.currentRoute.value.query.itemId;
    let orderId = router.currentRoute.value.query.orderId;
    if (orderId && itemId) {
        return "one";
    } else if (!itemId && orderId) {
        return "all";
    }
});
const loading = ref<boolean>(true);
/** 金额是否近似为 0（规避浮点误差） */
const isZeroMoney = (value: unknown) => Math.abs(Number(value ?? 0)) < 0.005;
/** 全积分兑换且无现金支付 */
const fullPointsExchange = ref(false);
onMounted(async () => {
    await fetchAfterSales();
    await fetchAfterSalesConfig();
});
const fetchAfterSales = async () => {
    try {
        let temp = {
            itemId: router.currentRoute.value.query.itemId ? router.currentRoute.value.query.itemId : undefined,
            orderId: router.currentRoute.value.query.orderId ? router.currentRoute.value.query.orderId : undefined
        };
        formState.value.orderId = Number(temp.orderId);
        const result = await getAfterSales({ ...temp });
        const order = result.order as any;
        formState.value.orderStatus = order.orderStatus;
        fullPointsExchange.value =
            Number(order.isExchangeOrder) === 1 &&
            Number(order.pointsAmount ?? 0) > 0 &&
            isZeroMoney(order.totalAmount) &&
            isZeroMoney(order.paidAmount);

        orderList.value = result.list;
        orderList.value.forEach((item: any) => {
            item.number = item.isGift === 1 ? item.canApplyQuantity : 0;
        });
        if (fullPointsExchange.value) {
            formState.value.refundAmount = 0;
            formState.value.refundAmountMax = 0;
            productAmount.value = order?.productAmount;
            totalAmount.value = order?.totalAmount;
            return;
        }
        let tempPrice: any = 0;
        result.list.forEach((item: any, index: any) => {
            tempPrice = tempPrice + Number(item.canApplyQuantity * item.price);
        });
        productAmount.value = order?.productAmount;
        totalAmount.value = order?.totalAmount;
        tempPrice = (tempPrice / Number(productAmount.value)) * Number(totalAmount.value);
        formState.value.refundAmountMax = tempPrice.toFixed(2);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
/** 写入售后原因（后端按订单阶段返回已支付/已收货对应列表），已选原因不在新列表中时清空 */
const applyAftersaleReasons = (result: any) => {
    aftersaleReason.value = Array.isArray(result.aftersaleReason) ? result.aftersaleReason : [];
    afterSalesReasonEnabled.value = Number(result.afterSalesReasonEnabled) === 1;
    if (formState.value.aftersaleReason && !aftersaleReason.value.includes(formState.value.aftersaleReason)) {
        formState.value.aftersaleReason = "";
    }
};
/** 提交失败后按订单最新阶段刷新原因，应对后台改过列表或订单已变为已收货的情况 */
const refreshAftersaleReasons = async () => {
    try {
        const result = await getAfterSalesConfig({ orderId: Number(formState.value.orderId) });
        applyAftersaleReasons(result);
    } catch (error) {
        console.error(error);
    }
};
const fetchAfterSalesConfig = async () => {
    try {
        const result = await getAfterSalesConfig({ orderId: Number(formState.value.orderId) });
        applyAftersaleReasons(result);
        aftersaleType.value = result.aftersaleType;
        if (formState.value.orderStatus == 1) {
            for (let key in aftersaleType.value) {
                if (aftersaleType.value[key] === t("退货/退款")) {
                    delete aftersaleType.value[key];
                }
            }
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const productAmount = ref();
const totalAmount = ref();

watch(
    () => orderList.value,
    (newValue) => {
        if (fullPointsExchange.value) {
            formState.value.refundAmount = 0;
            return;
        }
        let tempPrice: any = 0;
        newValue.forEach((item: AfterSalesFormState) => {
            const productAmount = item?.attrPrice ? Number(item.attrPrice) + Number(item.price) : Number(item.price);
            tempPrice = tempPrice + (Number(item.number) || 0) * productAmount;
        });
        formState.value.refundAmount = Number(tempPrice.toFixed(2));
    },
    { deep: true, immediate: true }
);

function convertToObjectArray(obj: object) {
    if (!obj) {
        return [];
    }
    return Object.entries(obj).map(([key, value]) => ({
        id: Number(key),
        name: String(value)
    }));
}

const formRef = shallowRef();
const onSubmit = async () => {
    try {
        await formRef.value.validate();
        // loading.value = true;
        formState.value.orderItemId = formState.value.itemId;

        formState.value.items = [];
        orderList.value.forEach((item: any) => {
            formState.value.items.push({ orderItemId: item.itemId, number: item.number });
        });
        const payload: Record<string, any> = { ...formState.value };
        if (!afterSalesReasonEnabled.value) delete payload.aftersaleReason;
        const result = await updateAfterSales(payload);
        message.success(t("提交成功"));
        await formRef.value.resetFields();

        navigateTo("/member/return/log");
    } catch (error: any) {
        if (error.message) {
            message.error(error.message);
            await refreshAftersaleReasons();
        }
    } finally {
        loading.value = false;
    }
};

const router = useRouter();
const splitStr = (str: string) => {
    return str.split("?")[0];
};
const menuList = reactive<any>([
    { value: "售后申请", url: "/member/return/Info", size: 0 },
    { value: "申请记录", url: "/member/return/log", size: 0 }
]);
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.log-return-content {
    background: #fff;
    border: 0;
    padding: 20px;
    box-sizing: border-box;
    min-height: 340px;

    .file-card {
        margin: 30px;

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
        margin: 0 30px 30px 30px;
        box-sizing: border-box;
        border-collapse: collapse; /* 边框合并为单一边框 */

        .table-header {
            background-color: #f5f5f5;
            border: 1px solid #ddd;
        }

        .header-cell {
            border: 1px solid #ddd;
            padding: 8px 10px;
            box-sizing: border-box;
            text-align: center;
        }

        /* 设置单元格样式 */

        td {
            border: 1px solid #e8e8e8; /* 单元格边框颜色 */
            padding: 10px; /* 单元格内边距 */
            text-align: center;
            .pro_pic {
                min-width: 40px;
                max-width: 40px;
                min-height: 40px;
                max-height: 40px;
            }
            .text-ellipsis {
                line-height: 1.8;
                display: flex;
                justify-content: flex-start;
                text-align: left;
                .name {
                    font-weight: 600;
                    //color: var(--general);
                }
            }

            .nuxt {
                display: flex;
                gap: 10px;
            }
        }
    }
}
</style>
