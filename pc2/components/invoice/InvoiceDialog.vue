<!-- 订单开票弹窗，结算页与会员发票列表共用 -->
<template>
    <el-dialog v-model="dialogVisible" :title="$t('发票申请')" width="800">
        <el-form v-if="params.type === 'info'" label-width="auto" :model="formInfoState" class="form-body" :label-suffix="labelSuffix">
            <div class="audit-status">
                {{ $t("审核状态") }}：
                <span :style="getStatusStyle(formInfoState.status)">{{ formInfoState.statusName }}</span>
                <br />
                <span>{{ formInfoState.applyReply }}</span>
            </div>
            <el-form-item v-if="params.type != 'check'" :label="$t('订单号')" prop="orderSn">
                <span class="message">
                    <NuxtLink :to="'/member/order/info?id=' + formInfoState.orderId" class="font-color" target="_blank">{{ formInfoState.orderSn }}</NuxtLink>
                </span>
            </el-form-item>
            <el-form-item v-if="params.type != 'check'" :label="$t('订单金额')" prop="totalAmount">
                <FormatPrice v-model="formInfoState.totalAmount" :showText="false"></FormatPrice>
            </el-form-item>
            <el-form-item v-if="params.type != 'check'" :label="$t('发票金额')" prop="amount">
                <FormatPrice v-model="formInfoState.amount" :fontStyle="{ color: 'var(--price)' }" :showText="false"></FormatPrice>
            </el-form-item>
            <el-form-item :label="$t('发票类型')" prop="invoiceType">
                <span class="message">{{ formInfoState.invoiceType === 1 ? $t("普通发票") : $t("增值税专用发票") }}</span>
                <div class="extra">{{ $t("所有发票由商品供货商开具") }}</div>
            </el-form-item>
            <el-form-item :label="$t('发票抬头')" prop="titleType">
                <span class="message"> {{ formInfoState.titleType === 1 ? $t("个人") : $t("企业") }}</span>
            </el-form-item>
            <template v-if="formInfoState.invoiceType === 1">
                <template v-if="formInfoState.titleType === 1">
                    <el-form-item :label="$t('个人名称')" prop="companyName">
                        <span class="message"> {{ formInfoState.companyName }}</span>
                    </el-form-item>
                </template>
                <template v-else>
                    <el-form-item :label="$t('企业名称')" prop="companyName">
                        <span class="message"> {{ formInfoState.companyName }}</span>
                    </el-form-item>
                    <el-form-item :label="$t('纳税人识号')" prop="companyCode">
                        <span class="message">{{ formInfoState.companyCode }}</span>
                    </el-form-item>
                    <el-form-item :label="$t('单位地址')" prop="companyAddress">
                        <span class="message"> {{ formInfoState.companyAddress }}</span>
                    </el-form-item>
                    <el-form-item :label="$t('单位电话')" prop="companyPhone">
                        <span class="message"> {{ formInfoState.companyPhone }}</span>
                    </el-form-item>
                    <el-form-item :label="$t('开户银行')" prop="companyBank">
                        <span class="message"> {{ formInfoState.companyBank }}</span>
                    </el-form-item>
                    <el-form-item :label="$t('银行账户')" prop="companyAccount">
                        <span class="message"> {{ formInfoState.companyAccount }}</span>
                    </el-form-item>
                </template>
            </template>
            <el-form-item :label="$t('发票内容')" prop="invoiceContent">
                <span class="message">{{ $t("商品明细") }}</span>
                <div class="extra">{{ $t("发票内容将显示详细商品名称与价格信息") }}</div>
            </el-form-item>
            <template v-if="formState.invoiceType === 2">
                <el-form-item :label="$t('单位名称')"
                    ><span class="message">{{ formInfoState.companyName }}</span></el-form-item
                >
                <el-form-item :label="$t('纳税人识别码')"
                    ><span class="message">{{ formInfoState.companyCode }}</span></el-form-item
                >
                <el-form-item :label="$t('注册地址')"
                    ><span class="message">{{ formInfoState.companyAddress }}</span></el-form-item
                >
                <el-form-item :label="$t('注册电话')"
                    ><span class="message">{{ formInfoState.companyPhone }}</span></el-form-item
                >
                <el-form-item :label="$t('开户银行')"
                    ><span class="message">{{ formInfoState.companyBank }}</span></el-form-item
                >
                <el-form-item :label="$t('银行账户')"
                    ><span class="message">{{ formInfoState.companyAccount }}</span></el-form-item
                >
                <el-form-item :label="$t('发票内容')" prop="invoiceContent">
                    <span class="message"> {{ $t("商品明细") }}</span>
                    <div class="extra">{{ $t("发票内容将显示详细商品名称与价格信息") }}</div>
                </el-form-item>
            </template>
            <el-form-item :label="$t('收票人手机')" prop="mobile">
                <span class="message"> {{ formInfoState.mobile }}</span>
            </el-form-item>
            <el-form-item :label="$t('收票人邮箱')" prop="email">
                <span class="message"> {{ formInfoState.email }}</span>
            </el-form-item>
            <el-form-item v-if="(formInfoState as any).invoiceAttachment?.length" :label="$t('发票图片')" prop="invoiceAttachment">
                <el-image
                    v-for="(item, index) in (formInfoState as any).invoiceAttachment"
                    :key="'a' + index"
                    :src="item.picUrl"
                    :preview-src-list="(formInfoState as any).invoiceAttachment.map((i: any) => i.picUrl)"
                    :initial-index="index"
                    :preview-teleported="true"
                    fit="contain"
                    class="invoice-img"
                />
            </el-form-item>
            <el-form-item v-else-if="(formInfoState as any).invoiceImg" :label="$t('发票图片')" prop="invoiceImg">
                <el-image
                    :src="imageFormat((formInfoState as any).invoiceImg)"
                    :preview-src-list="[imageFormat((formInfoState as any).invoiceImg)]"
                    :preview-teleported="true"
                    fit="contain"
                    class="invoice-img"
                />
            </el-form-item>
        </el-form>
        <el-form v-else ref="formRef" :label-width="110" :model="formState" class="form-body" :label-suffix="labelSuffix">
            <div v-if="params.type === 'reapply'" class="audit-status">
                {{ $t("审核状态") }}：
                <span :style="getStatusStyle(formState.status)">{{ formState.statusName }}</span>
                <br />
                <span>{{ formState.applyReply }}</span>
            </div>
            <el-form-item v-if="params.type != 'check'" :label="$t('订单号')" prop="orderSn">
                <span class="message blue">
                    <NuxtLink :to="'/member/order/info?id=' + formState.orderId" target="_blank">{{ formState.orderSn }}</NuxtLink>
                </span>
            </el-form-item>
            <el-form-item v-if="params.type != 'check'" :label="$t('订单金额')" prop="totalAmount">
                <FormatPrice v-model="formState.totalAmount" :showText="false"></FormatPrice>
            </el-form-item>
            <el-form-item v-if="params.type != 'check'" :label="$t('发票金额')" prop="amount">
                <FormatPrice v-model="formState.amount" :fontStyle="{ color: 'var(--price)' }" :showText="false"></FormatPrice>
            </el-form-item>
            <el-form-item :label="$t('发票类型')" prop="invoiceType">
                <el-radio-group v-model="formState.invoiceType" @change="changeType('invoiceType')">
                    <el-radio :value="1">{{ $t("普通发票") }}</el-radio>
                    <el-radio v-if="commonStore.invoiceAdded == 1" :value="2">{{ $t("增值税专用发票") }}</el-radio>
                </el-radio-group>
                <template v-if="formState.invoiceType === 1">
                    <div class="extra">{{ $t("所有发票由商品供货商开具") }}</div>
                </template>
                <template v-if="formState.invoiceType === 2 && invoiceStatus">
                    <div class="extra">{{ $t("所有发票由商品供货商开具") }}</div>
                </template>
                <template v-if="formState.invoiceType === 2 && !invoiceStatus">
                    <div class="extra">
                        {{ $t("您还未通过增票资质申请，暂时无法开具增值税专用发票") }}，
                        <NuxtLink class="font-color" to="/member/userInvoice/info">{{ $t("点击这里") }}</NuxtLink>
                        {{ $t("去申请增票资质") }}。
                    </div>
                </template>
            </el-form-item>
            <template v-if="formState.invoiceType === 1">
                <div class="audit-status">
                    <p>{{ $t("根据现行税收政策，部分公司提供数电票，数电（普通发票）法律效力与现有电子普通发票相同") }}；</p>
                    <p>{{ $t("电子普通发票和纸质普通发票具备同等法律效力，可支持报销入账") }}；</p>
                    <p>{{ $t("如商品由第三方卖家销售，发票类型及内容将由该卖家决定") }}；</p>
                    <p>{{ $t("如您为企业采购，需要多单合并开具或批量提交发票可注册企业用户") }}；</p>
                    <p class="var(--price)">{{ $t("使用礼品卡支付部分的金额，不支持开发票") }}</p>
                </div>
                <el-form-item :label="$t('发票抬头')" prop="titleType">
                    <el-radio-group v-model="formState.titleType" @change="changeType('titleType')">
                        <el-radio :value="1">{{ $t("个人") }}</el-radio>
                        <el-radio :value="2">{{ $t("企业") }}</el-radio>
                    </el-radio-group>
                </el-form-item>
                <template v-if="formState.titleType === 1">
                    <el-form ref="formOneRef" :label-width="110" :model="formOneState" class="form-body" :label-suffix="labelSuffix">
                        <el-form-item :rules="[{ required: true, message: $t('个人名称不能为空') }]" :label="$t('个人名称')" prop="companyName">
                            <el-input v-model="formOneState.companyName" clearable :placeholder="$t('请输入个人名称')" />
                        </el-form-item>
                    </el-form>
                </template>
                <template v-else>
                    <el-form ref="formFirmRef" :label-width="110" :model="formFirmState" class="form-body" :label-suffix="labelSuffix">
                        <el-form-item :rules="[{ required: true, message: $t('企业名称不能为空') }]" :label="$t('企业名称')" prop="companyName">
                            <el-input v-model="formFirmState.companyName" clearable :placeholder="$t('请输入企业名称')" />
                        </el-form-item>
                        <el-form-item
                            :rules="[
                                { required: true, message: $t('纳税人识号不能为空') },
                                { validator: createNoChineseValidator($t('纳税人识号')), trigger: 'blur' }
                            ]"
                            :label="$t('纳税人识号')"
                            prop="companyCode"
                        >
                            <el-input v-model="formFirmState.companyCode" clearable :placeholder="$t('请输入纳税人识号')" />
                        </el-form-item>
                        <el-form-item :label="$t('单位地址')" prop="companyAddress">
                            <el-input v-model="formFirmState.companyAddress" clearable :placeholder="$t('请输入单位地址')" />
                        </el-form-item>
                        <el-form-item
                            :label="$t('单位电话')"
                            :rules="[
                                { validator: createNoChineseValidator($t('单位电话')), trigger: 'blur' }
                            ]"
                            prop="companyPhone"
                        >
                            <el-input v-model="formFirmState.companyPhone" clearable :placeholder="$t('请输入单位电话')" />
                        </el-form-item>
                        <el-form-item :label="$t('开户银行')" prop="companyBank">
                            <el-input v-model="formFirmState.companyBank" clearable :placeholder="$t('请输入开户银行')" />
                        </el-form-item>
                        <el-form-item
                            :label="$t('银行账户')"
                            :rules="[
                                { validator: createNoChineseValidator($t('银行账户')), trigger: 'blur' }
                            ]"
                            prop="companyAccount"
                        >
                            <el-input v-model="formFirmState.companyAccount" clearable :placeholder="$t('请输入银行账户')" />
                        </el-form-item>
                    </el-form>
                </template>
            </template>
            <template v-else-if="formState.invoiceType === 2 && invoiceStatus">
                <el-form ref="formInvoiceRef" :label-width="110" :model="formInvoiceState" class="form-body" :label-suffix="labelSuffix">
                    <div class="audit-status">
                        <p>{{ $t("我公司依法开具发票，请您按照税法规定使用发票") }}。</p>
                        <p>{{ $t("根据现行税收政策，部分公司提供数电票, 数电（专用发票）法律效力与现有增值税专用发票相同") }};</p>
                        <p>{{ $t("如商品由第三方卖家销售，发票类型和内容将由该卖家决定，发票由卖家开具并提供") }}。</p>
                        <p>{{ $t("如您为企业采购，需要多单合并开具或批量提交发票可注册企业用户") }}；</p>
                        <p class="var(--price)">{{ $t("使用礼品卡支付部分的金额，不支持开发票") }}</p>
                    </div>
                    <el-form-item :label="$t('单位名称')">{{ formInvoiceState.companyName }}</el-form-item>
                    <el-form-item :label="$t('纳税人识别码')">{{ formInvoiceState.companyCode }}</el-form-item>
                    <el-form-item :label="$t('注册地址')">{{ formInvoiceState.companyAddress }}</el-form-item>
                    <el-form-item :label="$t('注册电话')">{{ formInvoiceState.companyPhone }}</el-form-item>
                    <el-form-item :label="$t('开户银行')">{{ formInvoiceState.companyBank }}</el-form-item>
                    <el-form-item :label="$t('银行账户')">{{ formInvoiceState.companyAccount }}</el-form-item>
                </el-form>
            </template>
            <template v-if="formState.invoiceType != 0">
                <el-form-item
                    :rules="[
                        { validator: createNoChineseValidator($t('收票人手机')), trigger: 'blur' }
                    ]"
                    :label="$t('收票人手机')"
                    prop="mobile"
                >
                    <el-input v-model="formState.mobile" clearable :placeholder="$t('请输入收票人手机')" />
                </el-form-item>
                <el-form-item
                    :rules="[
                        { required: formState.invoiceType !== 1, message: $t('邮箱不能为空') },
                        { validator: validateEmail, trigger: 'blur' }
                    ]"
                    :label="$t('收票人邮箱')"
                    prop="email"
                >
                    <el-input v-model="formState.email" clearable :placeholder="$t('请输入收票人邮箱')" />
                </el-form-item>
            </template>
        </el-form>
        <template #footer>
            <div class="dialog-footer">
                <el-button plain v-if="params.type === 'reapply' || params.type === 'apply' || params.type === 'check'" @click="handleCancel">
                    {{ $t("暂不开票") }}
                </el-button>
                <el-button
                    :disabled="formState.invoiceType === 2 && !invoiceStatus"
                    type="primary"
                    v-if="params.type === 'reapply' || params.type === 'apply' || params.type === 'check'"
                    @click="onSubmit"
                >
                    {{ $t("提交开票申请") }}
                </el-button>
            </div>
        </template>
    </el-dialog>
    <span @click="showDialog">
        <slot></slot>
    </span>
</template>
<script lang="ts" setup>
/**
 * 订单开票弹窗组件（共享组件）
 * - 为指定订单申请开具发票
 * - 选择发票类型、抬头、内容
 * - 电子发票填写接收邮箱
 *
 * 同时被 pages/order/check.vue（下单结算）与 pages/member/orderInvoice/List.vue（会员发票管理）复用，
 */
import { ref } from "vue";
import type { OrderInvoiceFormState } from "~/types/user/orderInvoice";
import { getOrderInvoice, insertOrderInvoice } from "~/api/user/orderInvoice";
import { getInvoiceStatus } from "~/api/user/userInvoice";
import { getCheckInvoice } from "@/api/order/check";
import { useCommonStore } from "~/store/common";

const props = defineProps({
    params: {
        type: Object,
        default: {}
    }
});
const commonStore = useCommonStore();
const getStatusStyle = (status: number | undefined) => {
    switch (status) {
        case 0:
            return { color: "#f58b49" }; // Pending
        case 1:
            return { color: "green" }; // Approved
        case 2:
            return { color: "red" }; // Denied
        default:
            return {}; // Default style
    }
};

const loading = ref<boolean>(true);
const formRef = shallowRef();
const formOneRef = shallowRef();
const formFirmRef = shallowRef();
const dialogVisible = ref(false);
const formState = ref<OrderInvoiceFormState>({
    orderId: props.params.orderId,
    orderSn: props.params.orderSn,
    amount: props.params.amount,
    totalAmount: props.params.totalAmount
});
const formInfoState = ref<OrderInvoiceFormState>({});

const formOneState = ref<OrderInvoiceFormState>({});
const formFirmState = ref<OrderInvoiceFormState>({});
const formInvoiceState = ref<OrderInvoiceFormState>({});

const showDialog = async () => {
    dialogVisible.value = true;
    if (formRef.value) await formRef.value.resetFields();
    // 结算页和首次申请回填最近一次开票信息，重新申请/查看走订单发票详情
    if (props.params.type === "check" || props.params.type === "apply") {
        // 接口按发票类型、抬头类型精确查询，未选择时默认普通发票、个人抬头
        formState.value.invoiceType = props.params.invoiceData?.invoiceType || formState.value.invoiceType || 1;
        formState.value.titleType = props.params.invoiceData?.titleType || formState.value.titleType || 1;
        await fetcCheckInvoice(formState.value.titleType as number);
    }
    if (props.params.type != "apply" && props.params.type != "check") {
        await fetchOrderInvoice();
    }
    if (props.params.type == "check") {
        Object.assign(formState.value, props.params.invoiceData);
        Object.assign(formInfoState.value, props.params.invoiceData);
        Object.assign(formOneState.value, props.params.invoiceData);
        Object.assign(formFirmState.value, props.params.invoiceData);
    }
    await fetchInvoiceStatus();
};

/**
 * 回填指定抬头类型最近一次的普通发票信息
 * 只取表单字段，避免把其他订单的 id、状态等写入当前申请；手机、邮箱仅在未填写时回填
 */
const fetcCheckInvoice = async (titleType: number) => {
    try {
        const result = await getCheckInvoice({ invoiceType: 1, titleType });
        const data: OrderInvoiceFormState = result
            ? result
            : titleType === 1
              ? { companyName: props.params.address?.consignee, mobile: props.params.address?.mobile, email: props.params.address?.email }
              : {};
        if (titleType === 1) {
            formOneState.value.companyName = data.companyName || "";
        } else {
            formFirmState.value.companyName = data.companyName || "";
            formFirmState.value.companyCode = data.companyCode || "";
            formFirmState.value.companyAddress = data.companyAddress || "";
            formFirmState.value.companyPhone = data.companyPhone || "";
            formFirmState.value.companyBank = data.companyBank || "";
            formFirmState.value.companyAccount = data.companyAccount || "";
        }
        if (!formState.value.mobile) formState.value.mobile = data.mobile || "";
        if (!formState.value.email) formState.value.email = data.email || "";
        formState.value.amount = props.params.amount;
    } catch (error: any) {
        message.error(error.message);
    }
};

const fetchOrderInvoice = async () => {
    try {
        loading.value = true;
        const result = await getOrderInvoice({ id: props.params.orderId });
        Object.assign(formState.value, result);
        Object.assign(formInfoState.value, result);
        Object.assign(formOneState.value, result);
        Object.assign(formFirmState.value, result);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const invoiceStatus = ref<boolean>(false);
const fetchInvoiceStatus = async () => {
    try {
        const result = await getInvoiceStatus();
        if (result) {
            invoiceStatus.value = true;
            Object.assign(formInvoiceState.value, result);
        } else {
            invoiceStatus.value = false;
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const emit = defineEmits(["loadFilter", "callback", "changeType"]);
const changeType = (data: string) => {
    // 个人、企业表单分开保存；切到尚未填写的抬头时回填该抬头最近一次的开票信息
    if (data == "titleType" && props.params.type != "info") {
        const target = formState.value.titleType === 1 ? formOneState.value : formFirmState.value;
        if (!target.companyName) fetcCheckInvoice(formState.value.titleType as number);
    }
    if (props.params.type == "check") {
        emit("changeType", {
            type: data,
            value: data == "invoiceType" ? formState.value.invoiceType : formState.value.titleType
        });
        if (data == "invoiceType" && formState.value.invoiceType == 2) {
            fetchInvoiceStatus();
        }
    }
};

const handleCancel = () => {
    emit("callback", {});
    dialogVisible.value = false;
};
const { t, locale } = useI18n();

const labelSuffix = computed(() => (String(locale.value).toLowerCase().startsWith("zh") ? "：" : ":"));

// 验证不能包含中文（生成带字段名的验证器）
const createNoChineseValidator = (fieldName: string) => {
    return (rule: any, value: any, callback: any) => {
        if (value && /[\u4e00-\u9fa5]/.test(value)) {
            callback(new Error(t(`${fieldName}格式不正确`)));
        } else {
            callback();
        }
    };
};

// 验证邮箱格式
const validateEmail = (rule: any, value: any, callback: any) => {
    if (!value) {
        callback();
        return;
    }
    const emailReg = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;
    if (!emailReg.test(value)) {
        callback(new Error(t("请输入正确的邮箱格式")));
    } else {
        callback();
    }
};

const onSubmit = async () => {
    if (formRef.value) await formRef.value.validate();
    try {
        loading.value = true;
        let temp: OrderInvoiceFormState = {
            id: props.params.orderId,
            invoiceType: formState.value.invoiceType,
            titleType: formState.value.titleType,
            mobile: formState.value.mobile,
            email: formState.value.email,
            amount: formState.value.amount
        };
        temp.invoiceContent = formState.value.invoiceContent;
        if (formState.value.invoiceType === 1 && formState.value.titleType === 1) {
            //普通个人
            if (formRef.value) await formOneRef.value.validate();
            temp.companyCode = "";
            temp.companyName = formOneState.value.companyName;
            temp.companyAddress = "";
            temp.companyPhone = "";
            temp.companyBank = "";
            temp.companyAccount = "";
        } else if (formState.value.invoiceType === 1 && formState.value.titleType === 2) {
            //普通企业
            if (formRef.value) await formFirmRef.value.validate();
            temp.companyCode = formFirmState.value.companyCode;
            temp.companyName = formFirmState.value.companyName;
            temp.companyAddress = formFirmState.value.companyAddress;
            temp.companyPhone = formFirmState.value.companyPhone;
            temp.companyBank = formFirmState.value.companyBank;
            temp.companyAccount = formFirmState.value.companyAccount;
        } else if (formState.value.invoiceType === 2) {
            temp.companyCode = formInvoiceState.value.companyCode;
            temp.companyName = formInvoiceState.value.companyName;
            temp.companyAddress = formInvoiceState.value.companyAddress;
            temp.companyPhone = formInvoiceState.value.companyPhone;
            temp.companyBank = formInvoiceState.value.companyBank;
            temp.companyAccount = formInvoiceState.value.companyAccount;
        }
        if (props.params.type != "check") {
            const result = await insertOrderInvoice(temp, props.params.type === "apply" ? "create" : "update");
            await formRef.value.resetFields();
            message.success(t("操作成功"));
            dialogVisible.value = false;
            emit("loadFilter");
        } else {
            delete temp.id;
            dialogVisible.value = false;
            emit("callback", temp);
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.message {
    font-size: 12px;
}

.audit-status {
    background: none repeat scroll 0 0 #fffdee;
    border: 1px solid #edd28b;
    padding: 10px;
    font-size: 12px;

    line-height: 2;
    text-align: left;
    margin: 26px;
}

.no-invoice {
    margin-left: 100px;
    margin-top: 20px;
    margin-bottom: 20px;
    font-size: 16px;
    color: #666;
    line-height: 2.5;
    text-align: left;

    p {
        font-size: 14px;
    }
}

.invoice-img {
    width: 100px;
    height: 100px;
    margin-right: 10px;
    border: 1px solid #eee;
    border-radius: 4px;
    cursor: pointer;
}
</style>
