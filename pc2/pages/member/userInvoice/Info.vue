<template>
    <CommonPageHeaderNew></CommonPageHeaderNew>
    <CommonHeader title="增票资质"></CommonHeader>
    <TopJumpBar tltle="增票资质"></TopJumpBar>
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
            <div class="invoice-info-content">
                <div class="tips">
                    <p>
                        <strong>{{ $t("温馨提示") }}</strong>
                    </p>
                    <p>{{ $t("我们会在一个工作日内完成审核工作") }}。</p>
                    <p>
                        1）{{ $t("资质审核结果查询时间") }}：<span class="font-color">{{ $t("资质上传一到两个工作日后即可查询") }}。</span>
                    </p>
                    <p>2）{{ $t("注意有效增值税专用发票开票资质仅为一个") }}。</p>
                </div>
                <div v-show="!showEdit" class="remind-box mt20">
                    <div class="i-mt">
                        <strong>{{ $t("您的增票资质") }}：</strong>
                        <span v-if="formState.status === 0" class="gray">{{ $t("未申请专用发票") }}</span>
                        <span v-else-if="formState.status === 1" class="green">{{ $t("审核通过") }}</span>
                        <span v-else-if="formState.status === 2" class="orange">{{ $t("已申请，待审核") }}</span>
                        <span v-else-if="formState.status === 3" class="red">{{ $t("审核未通过") }}</span>
                        <span v-else></span>
                    </div>
                    <el-button type="primary" size="small" v-if="!loading" class="mt10" @click="showEditFun">{{
                        formState.status === 0 ? $t("添加增票资质") : $t("修改并重新审核")
                    }}</el-button>
                    <div v-if="formState.status === 3" class="error-info">{{ $t("审核失败原因") }}：{{ formState.applyReply }}</div>
                </div>
            </div>
            <div v-if="!showEdit && formState.hasOwnProperty('companyName')" class="invoice-info-content">
                <div class="title-tips">
                    <li>{{ $t("填写增票资质信息") }}</li>
                </div>
                <el-form label-width="auto" class="form-body" label-suffix="：">
                    <el-form-item :label="$t('单位名称')">{{ formState.companyName }}</el-form-item>
                    <el-form-item :label="$t('纳税人识别码')">{{ formState.companyCode }}</el-form-item>
                    <el-form-item :label="$t('注册地址')">{{ formState.companyAddress }}</el-form-item>
                    <el-form-item :label="$t('注册电话')">{{ formState.companyPhone }}</el-form-item>
                    <el-form-item :label="$t('开户银行')">{{ formState.companyBank }}</el-form-item>
                    <el-form-item :label="$t('银行账户')">{{ formState.companyAccount }}</el-form-item>
                </el-form>
            </div>

            <div v-if="showEdit" class="invoice-info-content">
                <div class="tips mb-tips">
                    <span
                        >{{ $t("填写增票资质信息") }} <span class="font-color">（{{ $t("所有信息均为必填") }}）</span></span
                    >
                </div>
                <el-form ref="formRef" label-width="auto" :model="formState" :rules="rules" class="form-body" label-suffix="：">
                    <el-form-item :rules="[{ required: true, message: $t('单位名称不能为空') }]" :label="$t('单位名称')" prop="companyName">
                        <el-input v-model="formState.companyName" clearable :placeholder="$t('请输入单位名称')" />
                    </el-form-item>
                    <el-form-item :label="$t('纳税人识别码')" prop="companyCode">
                        <el-input v-model="formState.companyCode" clearable :placeholder="$t('请输入纳税人识别码')" />
                    </el-form-item>
                    <el-form-item :rules="[{ required: true, message: $t('注册地址不能为空') }]" :label="$t('注册地址')" prop="companyAddress">
                        <el-input v-model="formState.companyAddress" clearable :placeholder="$t('请输入注册地址')" />
                    </el-form-item>
                    <el-form-item :label="$t('注册电话')" prop="companyPhone">
                        <el-input v-model="formState.companyPhone" clearable :placeholder="$t('请输入注册电话')" />
                    </el-form-item>
                    <el-form-item :rules="[{ required: true, message: $t('开户银行不能为空') }]" :label="$t('开户银行')" prop="companyBank">
                        <el-input v-model="formState.companyBank" clearable :placeholder="$t('请输入开户银行')" />
                    </el-form-item>
                    <el-form-item :label="$t('银行账户')" prop="companyAccount">
                        <el-input v-model="formState.companyAccount" clearable :placeholder="$t('请输入银行账户')" />
                    </el-form-item>
                    <el-form-item label="" prop="isConfirm">
                        <el-checkbox v-model="formState.isConfirm" :false-value="0" :true-value="1"
                            >{{ $t("我已阅读并同意") }}
                            <NuxtLink class="font-color" target="_blank" to="/member/userInvoice/confirmInfo">《{{ $t("增票资质确认书") }}》</NuxtLink>
                        </el-checkbox>
                    </el-form-item>
                    <el-form-item label="">
                        <el-button type="primary" @click="onSubmit">{{ $t("确认") }}</el-button>
                        <el-button @click="showEditFun">{{ $t("取消") }}</el-button>
                    </el-form-item>
                </el-form>
            </div>
        </div>
    </CommonPageContainer>

    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 发票抬头管理页
 * - 查看和编辑默认发票抬头信息
 * - 支持个人/企业发票类型
 * - 企业发票需填写税号
 */
import { reactive, ref } from "vue";
import { useRouter } from "vue-router";
import { getUserInvoice, updateUserInvoice, invoiceUserInvoice } from "~/api/user/userInvoice";
import type { UserInvoiceFormState } from "~/types/user/userInvoice";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
definePageMeta({
    middleware: "auth"
});
const showEdit = ref(false);

const menuList = reactive<any>([
    { value: "发票申请", url: "/member/orderInvoice/list", size: 0 },
    { value: "增票资质", url: "/member/userInvoice/info", size: 0 }
]);
const loading = ref<boolean>(true);
const formRef = shallowRef();
const formState = ref<UserInvoiceFormState>({});
const fetchUserInvoice = async () => {
    try {
        const result = await getUserInvoice();
        Object.assign(formState.value, result);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
onMounted(() => {
    fetchUserInvoice();
});

const showEditFun = () => {
    fetchUserInvoice();
    showEdit.value = !showEdit.value;
    formState.value.isConfirm = 0;
};
const { t } = useI18n();

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

const rules = {
    companyCode: [
        { required: true, message: t("纳税人识别码不能为空"), trigger: "blur" },
        { validator: createNoChineseValidator(t("纳税人识别码")), trigger: "blur" }
    ],
    companyPhone: [
        { required: true, message: t("注册电话不能为空"), trigger: "blur" },
        { validator: createNoChineseValidator(t("注册电话")), trigger: "blur" }
    ],
    companyAccount: [
        { required: true, message: t("银行账户不能为空"), trigger: "blur" },
        { validator: createNoChineseValidator(t("银行账户")), trigger: "blur" }
    ],
    isConfirm: [
        { required: true, message: t("请勾选确认书"), trigger: "change" },
        {
            validator: (rule: object, value: number, callback: Function) => {
                if (value !== 1) {
                    callback(new Error(t("需要勾选确认才能继续")));
                } else {
                    callback();
                }
            },
            trigger: "change"
        }
    ]
};

const onSubmit = async () => {
    await formRef.value.validate();
    try {
        formState.value.titleType = 2;
        formState.value.invoiceType = 1;
        formState.value.isAdd = formState.value.status == 0 ? 1 : 0;
        let result: any = {};
        if (formState.value.status === 0) {
            delete formState.value.status;
            result = await invoiceUserInvoice(formState.value);
        } else {
            delete formState.value.status;
            result = await updateUserInvoice({ id: formState.value.invoiceId, ...formState.value });
        }

        showEditFun();
        message.success(t("操作成功"));
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.invoice-info-content {
    background: #fff;
    border: 0;
    padding: 20px 0;
    margin-bottom: 10px;
    line-height: 1.5;

    .title-tips {
        color: #999999;
        margin-bottom: 10px;
    }

    .tips {
        font-size: 14px;
    }

    .mb-tips {
        color: #999999;
        margin-bottom: 10px;
    }

    .remind-box {
        margin-top: 20px;
        background: none repeat scroll 0 0 #fffdee;
        border: 1px solid #edd28b;
        padding: 10px;

        .error-info {
            font-size: 14px;
            color: #666;
            margin-top: 10px;
        }
    }

    .update {
        cursor: pointer;
        color: #09f;
    }

    .form-body {
        width: 500px;

        :deep(.el-form-item__label) {
            white-space: nowrap;
        }
    }

    .form-item {
        & > .label {
            width: 100px;
            text-align: right;
        }
    }
}

.bt-content {
    min-height: 340px;
}

.gray {
    color: #999999;
}

.orange {
    color: #ff6c00;
}

.green {
    color: green;
}

.red {
    color: red;
}
</style>
