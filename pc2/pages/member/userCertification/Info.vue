<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>

    <CommonHeader title="个人资料"></CommonHeader>
    <TopJumpBar tltle="会员实名认证"></TopJumpBar>

    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="title-or-tabs">
                <span>{{ $t("会员实名认证") }}</span>
            </div>
            <div v-if="loading">
                <div class="page-info">
                    <div class="card">
                        <div class="form-info">
                            <step :modelValue="currentStep"></step>
                            <el-form ref="formStateRef" :model="formState" class="form-body" label-suffix="：" label-width="auto">
                                <div v-if="formState.status == 0 && currentStep == 1">
                                    <div class="title-item">{{ $t("认证类型") }}</div>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请选择实名类型') }]"
                                        class="form-item"
                                        :label="$t('认证类型')"
                                        prop="type"
                                    >
                                        <div class="input">
                                            <el-radio-group v-model="formState.type">
                                                <el-radio v-if="commonStore.companyDataType != 1" :value="1">{{ $t("个人") }}</el-radio>
                                                <el-radio :value="2">{{ $t("企业") }}</el-radio>
                                            </el-radio-group>
                                        </div>
                                    </el-form-item>
                                    <div class="title-item">
                                        {{ formState.type == 1 ? $t("个人信息") : $t("法定代表人信息") }}
                                    </div>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请选择证件类型') }]"
                                        class="form-item"
                                        :label="$t('证件类型')"
                                        prop="companyData.documentType"
                                    >
                                        <div class="input">
                                            <el-select v-model="formState.companyData.documentType" class="item-width" :placeholder="$t('请选择证件类型')">
                                                <el-option v-for="item in documentTypeOptions" :key="item.value" :label="$t(item.label)" :value="item.value" />
                                            </el-select>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :label="formState.type == 1 ? $t('姓名') : $t('法定代表人姓名')"
                                        :rules="[
                                            {
                                                required: true,
                                                message: $t('请输入') + (formState.type == 1 ? $t('姓名') : $t('法定代表人姓名')) + '!'
                                            }
                                        ]"
                                        class="form-item"
                                        prop="companyData.corporateName"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.companyData.corporateName" class="item-width"></el-input>
                                        </div>
                                        <div v-if="formState.companyData.documentType == 2" class="info-tips">
                                            {{ $t("填写中文名（英文名），如“张三（Zhang San）”") }}
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入证件号码') }]"
                                        class="form-item"
                                        :label="$t('证件号码')"
                                        prop="companyData.documentNumber"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.companyData.documentNumber" class="item-width"></el-input>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[
                                            { required: true, message: $t('请输入联系人手机') },
                                            { pattern: /^1[3456789]\d{9}$/, message: $t('请输入有效的手机号码') }
                                        ]"
                                        class="form-item"
                                        :label="$t('联系人手机')"
                                        prop="companyData.contactPhone"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.companyData.contactPhone" class="item-width"></el-input>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请选择出生日期') }]"
                                        class="form-item"
                                        :label="$t('出生日期')"
                                        prop="companyData.birthday"
                                    >
                                        <div class="input">
                                            <el-date-picker
                                                v-model="formState.companyData.birthday"
                                                :placeholder="$t('请选择出生日期')"
                                                style="width: 280px"
                                                type="date"
                                                value-format="YYYY-MM-DD"
                                            />
                                        </div>
                                    </el-form-item>
                                    <el-form-item class="form-item">
                                        <template #label> <span class="required">*</span> {{ $t("证件有效期") }}： </template>
                                        <div class="one-item">
                                            <div class="to-time">
                                                <el-radio-group v-model="formState.companyData.certificateValidityPeriod">
                                                    <el-radio :value="1">{{ $t("区间有效") }}</el-radio>
                                                </el-radio-group>
                                                <el-form-item
                                                    :rules="
                                                        formState.companyData.certificateValidityPeriod === 1
                                                            ? [{ required: true, message: $t('请选择日期') }]
                                                            : []
                                                    "
                                                    label=""
                                                    prop="companyData.certificateValidityPeriodDate"
                                                >
                                                    <el-date-picker
                                                        v-model="formState.companyData.certificateValidityPeriodDate"
                                                        :clearable="false"
                                                        :disabled="formState.companyData.certificateValidityPeriod === 2"
                                                        :end-placeholder="$t('结束日期')"
                                                        range-separator="～"
                                                        :start-placeholder="$t('开始日期')"
                                                        type="daterange"
                                                        value-format="YYYY-MM-DD"
                                                    />
                                                </el-form-item>
                                            </div>
                                            <div class="to-time">
                                                <el-radio-group v-model="formState.companyData.certificateValidityPeriod">
                                                    <el-radio :value="2">{{ $t("长期有效") }}</el-radio>
                                                </el-radio-group>
                                                <el-form-item
                                                    :rules="
                                                        formState.companyData.certificateValidityPeriod === 2
                                                            ? [{ required: true, message: $t('请选择日期') }]
                                                            : []
                                                    "
                                                    label=""
                                                    prop="companyData.certificateValidityPeriodEnd"
                                                >
                                                    <el-date-picker
                                                        v-model="formState.companyData.certificateValidityPeriodEnd"
                                                        :clearable="false"
                                                        :disabled="formState.companyData.certificateValidityPeriod === 1"
                                                        :placeholder="$t('请选择日期')"
                                                        style="width: 168px"
                                                        type="date"
                                                        value-format="YYYY-MM-DD"
                                                    />
                                                    <span class="ml10">～ {{ $t("长期") }}</span>
                                                </el-form-item>
                                            </div>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入居住地址') }]"
                                        class="form-item"
                                        :label="$t('居住地址')"
                                        prop="companyData.residentialAddress"
                                    >
                                        <div class="input">
                                            <el-input
                                                :autosize="{ minRows: 2, maxRows: 6 }"
                                                v-model="formState.companyData.residentialAddress"
                                                class="item-width"
                                                :placeholder="$t('请输入居住地址')"
                                                type="textarea"
                                            ></el-input>
                                        </div>
                                    </el-form-item>
                                    <div class="title-item">
                                        {{ formState.type == 1 ? $t("个人证件照") : $t("法定代表人证件照") }}
                                    </div>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请上传证件照正面') }]"
                                        class="form-item"
                                        :label="$t('证件照正面')"
                                        prop="companyData.frontOfPhoto"
                                    >
                                        <div class="input">
                                            <UploadImage
                                                v-model:uploadList="formState.companyData.frontOfPhoto"
                                                :fileTypes="['jpeg', 'png', 'jpeg']"
                                                :limit="1"
                                            ></UploadImage>
                                        </div>
                                        <div class="info-tips">
                                            <p>
                                                {{ $t("如你的证件类型是身份证，请在此处上传人像面，若为复印件，需加盖公司鲜章。注意：证件盖章不支持电子章。") }}
                                            </p>
                                            <p>{{ $t("护照需要增加翻译件，加盖公司鲜章，外籍商家姓名需为中文（英文），如张三（Zhangsan）。") }}</p>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请上传证件照反面') }]"
                                        class="form-item"
                                        :label="$t('证件照反面')"
                                        prop="companyData.backOfPhoto"
                                    >
                                        <div class="input">
                                            <UploadImage
                                                v-model:uploadList="formState.companyData.backOfPhoto"
                                                :fileTypes="['jpeg', 'png', 'jpeg']"
                                                :limit="1"
                                            ></UploadImage>
                                        </div>
                                        <div class="info-tips">
                                            <p>
                                                {{ $t("如你的证件类型是身份证，请在此处上传国徽面，若为复印件，需加盖公司鲜章。注意：证件盖章不支持电子章。") }}
                                            </p>
                                            <p>{{ $t("护照需要增加翻译件，加盖公司鲜章，外籍商家姓名需为中文（英文），如张三（Zhangsan）。") }}</p>
                                        </div>
                                    </el-form-item>
                                    <div v-if="formState.type == 2">
                                        <div class="title-item">{{ $t("企业信息") }}</div>
                                        <el-form-item
                                            :rules="[{ required: true, message: $t('请输入企业名称') }]"
                                            class="form-item"
                                            :label="$t('企业名称')"
                                            prop="companyData.companyName"
                                        >
                                            <div class="input">
                                                <el-input v-model="formState.companyData.companyName" class="item-width"></el-input>
                                            </div>
                                        </el-form-item>
                                        <el-form-item
                                            :rules="[{ required: true, message: $t('请选择注册地址') }]"
                                            class="form-item"
                                            :label="$t('注册地址')"
                                            prop="companyData.licenseAddrProvince"
                                        >
                                            <div class="input">
                                                <SelectRegion
                                                    v-model="formState.companyData.licenseAddrProvince"
                                                    @change="onLicenseAddrProvince"
                                                ></SelectRegion>
                                            </div>
                                        </el-form-item>
                                        <el-form-item
                                            :rules="[{ required: true, message: $t('请输入详细地址') }]"
                                            class="form-item"
                                            :label="$t('详细地址')"
                                            prop="companyData.businessLicenseAddress"
                                        >
                                            <div class="input">
                                                <el-input
                                                    :autosize="{ minRows: 2, maxRows: 6 }"
                                                    v-model="formState.companyData.businessLicenseAddress"
                                                    class="item-width"
                                                    type="textarea"
                                                ></el-input>
                                            </div>
                                            <div class="info-tips">{{ $t("注册地址需与营业执照登记住所一致") }}</div>
                                        </el-form-item>
                                        <el-form-item
                                            :rules="[{ required: true, message: $t('请输入经营范围') }]"
                                            class="form-item"
                                            :label="$t('经营范围')"
                                            prop="companyData.businessScope"
                                        >
                                            <div class="input">
                                                <el-input
                                                    :autosize="{ minRows: 2, maxRows: 6 }"
                                                    v-model="formState.companyData.businessScope"
                                                    class="item-width"
                                                    type="textarea"
                                                ></el-input>
                                            </div>
                                            <div class="info-tips">{{ $t("与企业工商营业执照上一致") }}</div>
                                        </el-form-item>
                                        <el-form-item
                                            :rules="[{ required: true, message: $t('请输入统一社会信用代码') }]"
                                            class="form-item"
                                            :label="$t('统一社会信用代码')"
                                            prop="companyData.businessLicenseId"
                                        >
                                            <div class="input">
                                                <el-input v-model="formState.companyData.businessLicenseId" class="item-width"></el-input>
                                            </div>
                                            <div class="info-tips">{{ $t("请输入营业执照上18位统一社会信用代码") }}</div>
                                        </el-form-item>
                                        <el-form-item class="form-item">
                                            <template #label> <span class="required">*</span> {{ $t("营业期限") }}： </template>
                                            <div class="one-item">
                                                <div class="to-time">
                                                    <el-radio-group v-model="formState.companyData.operatingTermType">
                                                        <el-radio :value="1">{{ $t("区间有效") }}</el-radio>
                                                    </el-radio-group>
                                                    <el-form-item
                                                        :rules="
                                                            formState.companyData.operatingTermType === 1 ? [{ required: true, message: $t('请选择日期') }] : []
                                                        "
                                                        label=""
                                                        prop="companyData.operatingTermTypeDate"
                                                    >
                                                        <el-date-picker
                                                            v-model="formState.companyData.operatingTermTypeDate"
                                                            :clearable="false"
                                                            :disabled="formState.companyData.operatingTermType === 2"
                                                            :end-placeholder="$t('结束日期')"
                                                            range-separator="～"
                                                            :start-placeholder="$t('开始日期')"
                                                            type="daterange"
                                                            value-format="YYYY-MM-DD"
                                                        />
                                                    </el-form-item>
                                                </div>
                                                <div class="to-time">
                                                    <el-radio-group v-model="formState.companyData.operatingTermType">
                                                        <el-radio :value="2">{{ $t("长期有效") }}</el-radio>
                                                    </el-radio-group>
                                                    <el-form-item
                                                        :rules="
                                                            formState.companyData.operatingTermType === 2 ? [{ required: true, message: $t('请选择日期') }] : []
                                                        "
                                                        label=""
                                                        prop="companyData.operatingTermTypeEnd"
                                                    >
                                                        <el-date-picker
                                                            v-model="formState.companyData.operatingTermTypeEnd"
                                                            :clearable="false"
                                                            :disabled="formState.companyData.operatingTermType === 1"
                                                            :placeholder="$t('请选择日期')"
                                                            style="width: 168px"
                                                            type="date"
                                                            value-format="YYYY-MM-DD"
                                                        />
                                                        <span class="ml10">～ {{ $t("长期") }}</span>
                                                    </el-form-item>
                                                </div>
                                            </div>
                                        </el-form-item>
                                        <el-form-item
                                            :rules="[{ required: true, message: $t('请上传营业执照') }]"
                                            class="form-item"
                                            :label="$t('营业执照电子版')"
                                            prop="companyData.businessLicenseImg"
                                        >
                                            <div class="input">
                                                <UploadImage
                                                    v-model:uploadList="formState.companyData.businessLicenseImg"
                                                    :fileTypes="['jpeg', 'png', 'jpeg']"
                                                    :limit="1"
                                                ></UploadImage>
                                            </div>
                                            <div class="info-tips">{{ $t("必须为彩色图片且小于2M，若为复印件，需加盖公司红章。") }}</div>
                                        </el-form-item>
                                    </div>
                                </div>
                                <el-form class="form-body" label-suffix="：" label-width="auto" v-if="formState.status != 0 && currentStep == 2">
                                    <div v-if="formState.status == 1" class="remind-box">
                                        {{ $t("申请已受理（{0}），将在{1}个工作日内完成审核。以下为本次申请内容：", [formState.addTime, commonStore.companyDataTips || 3]) }}
                                    </div>
                                    <div v-if="formState.status == 2" class="remind-box">
                                        {{ $t("恭喜您，您的申请审核") }}<span class="green">{{ $t("已通过") }}</span
                                        >。
                                        <br />
                                        <div v-if="formState.auditRemark">{{ $t("备注") }}：{{ formState.auditRemark }}</div>
                                    </div>
                                    <div v-if="formState.status == 3" class="remind-box">
                                        {{ $t("很遗憾，您的申请审核") }}<span class="red">{{ $t("未通过") }}</span
                                        >。
                                        <el-button class="ml10" type="primary" @click="review">{{ $t("重新申请") }}</el-button>
                                        <br />
                                        <div v-if="formState.auditRemark">{{ $t("备注") }}：{{ formState.auditRemark }}</div>
                                    </div>
                                    <div class="title-item">{{ $t("基本信息") }}</div>
                                    <div class="sub-title-item">
                                        {{ formState.type == 1 ? $t("个人信息") : $t("法定代表人信息") }}
                                    </div>
                                    <el-form-item :label="formState.type == 1 ? $t('姓名') : $t('法定代表人姓名')" class="form-item">
                                        <div class="input">
                                            {{ maskStringOne(formState.companyData.corporateName) }}
                                        </div>
                                    </el-form-item>
                                    <el-form-item class="form-item" :label="$t('证件类型')">
                                        <div class="input">
                                            {{ $t(documentTypeOptions.find((option) => option.value === formState.companyData.documentType)?.label || "") }}
                                        </div>
                                    </el-form-item>
                                    <el-form-item class="form-item" :label="$t('证件号码')">
                                        <div class="input">
                                            {{ maskString(formState.companyData.documentNumber) }}
                                        </div>
                                    </el-form-item>
                                    <el-form-item class="form-item" :label="$t('联系人手机号')">
                                        <div class="input">
                                            {{ maskString(formState.companyData.contactPhone) }}
                                        </div>
                                    </el-form-item>
                                    <el-form-item class="form-item" :label="$t('出生日期')">
                                        <div class="input">
                                            {{ formState.companyData.birthday }}
                                        </div>
                                    </el-form-item>
                                    <el-form-item class="form-item" :label="$t('证件有效期')">
                                        <div class="input">
                                            <template v-if="formState.companyData.certificateValidityPeriod == 1">
                                                {{
                                                    formState.companyData.certificateValidityPeriodDate[0] +
                                                    " ～ " +
                                                    formState.companyData.certificateValidityPeriodDate[1]
                                                }}
                                            </template>
                                            <template v-else> {{ formState.companyData.certificateValidityPeriodEnd }} ～ {{ $t("长期") }} </template>
                                        </div>
                                    </el-form-item>
                                    <el-form-item class="form-item" :label="$t('居住地址')">
                                        <div class="input">
                                            {{ formState.companyData.residentialAddress }}
                                        </div>
                                    </el-form-item>
                                    <div class="sub-title-item">{{ $t("证件照") }}</div>
                                    <el-form-item class="form-item" :label="$t('证件照正面')">
                                        <div class="input">
                                            <el-tooltip :content="$t('为了保护个人隐私信息不泄露，该照片不予展示')" effect="light" placement="right">
                                                <el-image :src="lock" class="image-size" fit="cover"></el-image>
                                            </el-tooltip>
                                        </div>
                                    </el-form-item>
                                    <el-form-item class="form-item" :label="$t('证件照反面')">
                                        <div class="input">
                                            <el-tooltip :content="$t('为了保护个人隐私信息不泄露，该照片不予展示')" effect="light" placement="right">
                                                <el-image :src="lock" class="image-size" fit="cover"></el-image>
                                            </el-tooltip>
                                        </div>
                                    </el-form-item>
                                    <template v-if="formState.type == 2">
                                        <div class="sub-title-item">{{ $t("企业信息") }}</div>
                                        <el-form-item class="form-item" :label="$t('企业名称')">
                                            <div class="input">
                                                {{ formState.companyData.companyName }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('注册地址')">
                                            <div class="input">
                                                {{ formState.companyData.licenseAddrProvinceName }} {{ formState.companyData.businessLicenseAddress }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('经营范围')">
                                            <div class="input">
                                                {{ formState.companyData.businessScope }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('统一社会信用代码')">
                                            <div class="input">
                                                {{ formState.companyData.businessLicenseId }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('营业期限')">
                                            <div class="input">
                                                <template v-if="formState.companyData.operatingTermType == 1">
                                                    {{
                                                        formState.companyData.operatingTermTypeDate[0] +
                                                        " ～  " +
                                                        formState.companyData.operatingTermTypeDate[1]
                                                    }}
                                                </template>
                                                <template v-else> {{ formState.companyData.operatingTermTypeEnd }} ～{{ $t("长期") }} </template>
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('营业执照')">
                                            <div class="input">
                                                <template v-if="formState.companyData.businessLicenseImg?.length > 0">
                                                    <el-image
                                                        :src="imageFormat(formState.companyData.businessLicenseImg[0].picUrl)"
                                                        class="image-size"
                                                        fit="cover"
                                                        @click="showImage(imageFormat(formState.companyData.businessLicenseImg[0].picUrl))"
                                                    ></el-image>
                                                </template>
                                            </div>
                                        </el-form-item>
                                    </template>
                                </el-form>
                            </el-form>
                        </div>
                    </div>
                </div>
                <div class="fixed-bottom" v-if="formState.status == 0">
                    <el-button size="large" type="primary" @click="onSubmit()">{{ $t("提交") }}</el-button>
                </div>
                <el-dialog v-model="dialogVisible">
                    <div class="image">
                        <el-image :src="dialogImageUrl" alt="" fit="cover" />
                    </div>
                </el-dialog>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 用户实名认证页面
 * - 填写企业/个人认证信息，包括营业执照、法人信息等
 * - 多步骤流程：资料填写 -> 审核公示
 * - B2B模式下可用，非B2B环境重定向到404
 */
import { message } from "ant-design-vue";
import { applyApply, getApplyInfo, getMyApply } from "~/api/user/userCertification";
import { ref } from "vue";
import { useRouter } from "vue-router";
import lock from "assets/images/join/lock@3x.png";
import UploadImage from "~/components/form/src/UploadImage.vue";
import { SelectRegion } from "~/components/region";
import { useCommonStore } from "~/store/common";
import { isB2B } from "@/utils/util";
import step from "./src/step.vue";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from '~/components/common/TopJumpBar.vue'

const currentStep = ref(1);
const { t } = useI18n();
definePageMeta({
    middleware: "auth"
});
const commonStore = useCommonStore();
const loading = ref(true);
const router = useRouter();

if (!isB2B()) {
    router.replace({ path: "/404" });
}
onMounted(async () => {
    await _getMyApply();
});
//表单主体信息
const formStateRef = shallowRef();
const formState: any = ref({
    type: commonStore.companyDataType == 1 ? 2 : 1,
    status: 0,
    companyData: {
        certificateValidityPeriod: 1,
        operatingTermType: 1,
        frontOfPhoto: [],
        backOfPhoto: [],
        licenseAddrProvince: [],
        supplementaryInformation: [],
        businessLicenseImg: []
    }
});

const apply = ref({
    id: 0,
    status: 0,
    type: 0
});
const _getMyApply = async () => {
    try {
        loading.value = false;
        const result = await getMyApply();
        if (result && result.id) {
            Object.assign(apply.value, result);
            await getData(apply.value.id);
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = true;
    }
};
const getData = async (id: number) => {
    try {
        loading.value = false;
        const result = await getApplyInfo(id);
        Object.assign(formState.value, result);
        if (formState.value.statue == 0) {
            currentStep.value = 1;
        } else {
            currentStep.value = 2;
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = true;
    }
};
//日期选择校验重置
watch(
    () => formState.value.companyData.certificateValidityPeriod,
    (val) => {
        if (val == 1) {
            formStateRef.value?.clearValidate("companyData.certificateValidityPeriodEnd");
        } else {
            formStateRef.value?.clearValidate("companyData.certificateValidityPeriodDate");
        }
    },
    { deep: true }
);

watch(
    () => formState.value.companyData.operatingTermType,
    (val) => {
        if (val == 1) {
            formStateRef.value?.clearValidate("companyData.operatingTermTypeEnd");
        } else {
            formStateRef.value?.clearValidate("companyData.operatingTermTypeDate");
        }
    },
    { deep: true }
);
const onLicenseAddrProvince = () => {
    formStateRef.value.validateField("companyData.licenseAddrProvince");
};
//身份证自动填充
watch(
    () => formState.value.companyData.documentNumber,
    (val) => {
        if (formState.value.companyData.documentType == 1 && val.length >= 18) {
            let temp = val.substr(6, 8);
            formState.value.companyData.birthday = temp.slice(0, 4) + "-" + temp.slice(4, 6) + "-" + temp.slice(6, 8);
            let genderIndicator = parseInt(val.substr(val.length - 2, 1), 10);
            // formState.value.companyData.sex = genderIndicator % 2 === 0 ? 2 : 1;
        }
    },
    { deep: true }
);

//提交申请
const onSubmit = async () => {
    await formStateRef.value.validate();
    try {
        const result: any = await applyApply({ ...formState.value });
        message.success(t("申请成功"));
        await _getMyApply();
        //成功后获取最新数据
    } catch (error: any) {
        message.error(error.message);
    }
};
//证件类型
const documentTypeOptions = ref([
    {
        value: 1,
        label: "大陆身份证"
    },
    {
        value: 2,
        label: "护照（限境外人士）"
    },
    {
        value: 3,
        label: "来往内地通行证（中国香港居民）"
    },
    {
        value: 4,
        label: "来往内地通行证（中国澳门居民）"
    },
    {
        value: 5,
        label: "来往大陆通行证（中国台湾居民）"
    }
]);

//重新申请
const review = () => {
    formState.value.status = 0;
    currentStep.value = 1;
};

//特殊方法
function maskString(str: any) {
    if (str) {
        if (str.length < 7) {
            return str;
        } else {
            const firstTwo = str.substring(0, 2);
            const lastFour = str.substring(str.length - 4, str.length);
            const masked = "*".repeat(str.length - 6);
            return firstTwo + masked + lastFour;
        }
    }
}

function maskStringOne(str: any) {
    if (str) {
        return str.slice(0, 1) + "*".repeat(str.length - 1);
    }
}

//预览图片
const dialogImageUrl = ref("");
const dialogVisible = ref(false);
const showImage = (url: string) => {
    dialogImageUrl.value = url;
    dialogVisible.value = true;
};
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";
.info-row {
    border: 0;
    padding: 0px 32px;
}
.xieyi {
    display: flex;
    justify-content: center;
    align-items: center;

    .url {
        font-size: 14px;
        cursor: pointer;
    }
}

.page-info {
    display: flex;
    align-items: flex-start;
    flex-direction: column;
    background-color: #ffffff;
    .card {
        display: flex;
        padding: 10px 0px;
        width: 100%;
        flex-direction: column;
        align-items: center;

        .form-info {
            display: flex;
            flex-direction: column;
            width: 100%;
            margin: 30px 0;

            .title-item {
                font-weight: 500;
                font-size: 16px;
                line-height: 20px;
                padding-bottom: 10px;
                border-bottom: 1px solid #e0e0e0;
                color: #1a1a1a;
                margin-top: 20px;
            }

            .sub-title-item {
                font-size: 14px;
                margin: 20px 0;
                font-weight: 500;
                line-height: 20px;
                padding-bottom: 10px;
                color: #1a1a1a;
            }
        }
    }
}

.form-item {
    margin: 20px 20px;

    .input {
        width: 100%;
        font-size: 12px;
    }
}

.item-width {
    width: 280px;
    font-size: 14px !important;
    :deep(.el-input__inner) {
        font-size: 14px;
    }
}

.info-tips {
    font-size: 12px;
    color: #969799;
    vertical-align: baseline;
    cursor: pointer;
    font-weight: 400;
}

.fixed-bottom {
    width: 100%;
    margin-bottom: 30px;
    padding-bottom: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #ffffff;
}

.remind-box {
    margin: 20px 0;
    background: none repeat scroll 0 0 #fffdee;
    border: 1px solid #edd28b;
    padding: 12px 16px;
    font-size: 14px;
    color: #666;
}

.one-item {
    display: flex;
    flex-direction: column;
    gap: 20px;

    .to-time {
        display: flex;
        align-items: center;
        gap: 20px;
    }
}

.image-size {
    width: 120px;
    height: 120px;
    border: 1px solid #e0e0e0;
    cursor: pointer;
}

:deep(.el-form-item__label) {
    font-size: 14px;
    color: #323233;
}

:deep(.el-form-item__content) {
    .input {
        font-size: 14px;
        color: #323233;
    }
}

.image {
    width: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
}

.required {
    color: var(--el-color-danger);
    margin-right: 4px;
}
</style>
