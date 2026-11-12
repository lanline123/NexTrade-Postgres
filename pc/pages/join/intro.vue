<!--
  商户入驻申请页面
  - 支持个人、企业和个体工商户三种入驻类型
  - 分步骤填写入驻信息：基本信息、联系人/结算账户/商户/店铺信息、企业信息（企业类型）、提交审核
  - 包含表单验证、证件照上传、地区选择等功能
  - 支持身份证号码自动识别生日和性别
  - 显示审核状态和审核结果，支持重新申请
  - 入驻成功后显示商户后台账号密码
-->
<template>
    <div>
        <CommonHeader title="招商入驻"></CommonHeader>
        <CommonPageHeader></CommonPageHeader>
        <template v-if="flow == 1 && loading">
                <introTable v-model:type="type">
                    <div class="xieyi">
                        <el-checkbox v-model="agreement" size="large"> {{ $t("我已同意") }} </el-checkbox>
                        <agreementWin>
                            <!-- <span class="font-color url"> 《NexTrade{{ $t("商户入驻协议") }}》 </span> -->
                            <span class="font-color url"> 《{{ $t("{0}商户入驻协议", [commonStore.shopName]) }}》 </span>
                        </agreementWin>
                    </div>
                    <el-button size="large" type="primary" @click="startApplying()"> {{ $t("材料准备就绪，开始入驻申请") }} </el-button>
                </introTable>
            </template>
            <template v-else-if="flow == 2 && loading">
                <div class="page-info">
                    <div v-if="loading" class="card container">
                        <step :modelValue="currentStep" :type="formState.baseData.type"></step>
                        <div data-form-rev="MjI4MmVhZ3M" class="form-info">
                            <el-form ref="formStateRef" :model="formState" class="form-body" label-suffix="：" label-width="auto">
                                <template v-if="currentStep == 1">
                                    <div class="title-item">{{ $t("主体类型") }}</div>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请选择主体类型') }]"
                                        class="form-item"
                                        :label="$t('主体类型')"
                                        prop="baseData.type"
                                    >
                                        <div class="input">
                                            <el-radio-group v-model="formState.baseData.type" disabled>
                                                <el-radio v-if="commonStore.personApplyEnabled == 1" :value="1">{{ $t("个人") }}</el-radio>
                                                <el-radio :value="2">{{ $t("企业") }}</el-radio>
                                                <el-radio :value="3">{{ $t("个体工商户") }}</el-radio>
                                            </el-radio-group>
                                        </div>
                                        <div class="info-tips">
                                            <span class="font-color" @click="toUpPage()">{{ $t("刷新页面修改类型") }}</span>
                                        </div>
                                    </el-form-item>
                                    <div class="title-item">
                                        {{ formState.baseData.type == 1 ? $t("个人信息") : $t("法定代表人信息信息") }}
                                    </div>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请选择证件类型') }]"
                                        class="form-item"
                                        :label="$t('证件类型')"
                                        prop="baseData.documentType"
                                    >
                                        <div class="input">
                                            <el-select v-model="formState.baseData.documentType" class="item-width" :placeholder="$t('请选择证件类型')">
                                                <el-option v-for="item in documentTypeOptions" :key="item.value" :label="$t(item.label)" :value="item.value" />
                                            </el-select>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :label="formState.baseData.type == 1 ? $t('姓名') : $t('法定代表人姓名')"
                                        :rules="[
                                            {
                                                required: true,
                                                message: $t('请输入') + (formState.baseData.type == 1 ? $t('姓名') : $t('法定代表人姓名')) + '!'
                                            }
                                        ]"
                                        class="form-item"
                                        prop="baseData.corporateName"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.baseData.corporateName" class="item-width"></el-input>
                                        </div>
                                        <div v-if="formState.baseData.documentType == 2" class="info-tips">
                                            {{ $t("填写中文名（英文名），如“张三（Zhang San）”") }}
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入证件号码') }]"
                                        class="form-item"
                                        :label="$t('证件号码')"
                                        prop="baseData.documentNumber"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.baseData.documentNumber" class="item-width"></el-input>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请选择出生日期') }]"
                                        class="form-item"
                                        :label="$t('出生日期')"
                                        prop="baseData.birthday"
                                    >
                                        <div class="input">
                                            <el-date-picker
                                                v-model="formState.baseData.birthday"
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
                                                <el-radio-group v-model="formState.baseData.certificateValidityPeriod">
                                                    <el-radio :value="1">{{ $t("区间有效") }}</el-radio>
                                                </el-radio-group>
                                                <el-form-item
                                                    :rules="
                                                        formState.baseData.certificateValidityPeriod === 1
                                                            ? [{ required: true, message: $t('请选择日期') }]
                                                            : []
                                                    "
                                                    label=""
                                                    prop="baseData.certificateValidityPeriodDate"
                                                >
                                                    <el-date-picker
                                                        v-model="formState.baseData.certificateValidityPeriodDate"
                                                        :clearable="false"
                                                        :disabled="formState.baseData.certificateValidityPeriod === 2"
                                                        :end-placeholder="$t('结束日期')"
                                                        range-separator="～"
                                                        :start-placeholder="$t('开始日期')"
                                                        type="daterange"
                                                        value-format="YYYY-MM-DD"
                                                    />
                                                </el-form-item>
                                            </div>
                                            <div class="to-time">
                                                <el-radio-group v-model="formState.baseData.certificateValidityPeriod">
                                                    <el-radio :value="2">{{ $t("长期有效") }}</el-radio>
                                                </el-radio-group>
                                                <el-form-item
                                                    :rules="
                                                        formState.baseData.certificateValidityPeriod === 2
                                                            ? [{ required: true, message: $t('请选择日期') }]
                                                            : []
                                                    "
                                                    label=""
                                                    prop="baseData.certificateValidityPeriodEnd"
                                                >
                                                    <el-date-picker
                                                        v-model="formState.baseData.certificateValidityPeriodEnd"
                                                        :clearable="false"
                                                        :disabled="formState.baseData.certificateValidityPeriod === 1"
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
                                        prop="baseData.residentialAddress"
                                    >
                                        <div class="input">
                                            <el-input
                                                :autosize="{ minRows: 2, maxRows: 6 }"
                                                v-model="formState.baseData.residentialAddress"
                                                class="item-width"
                                                :placeholder="$t('请输入居住地址')"
                                                type="textarea"
                                            ></el-input>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        class="form-item"
                                        :label="$t('性别')"
                                        prop="baseData.sex"
                                        :rules="[{ required: true, message: $t('请选择性别') }]"
                                    >
                                        <div class="input">
                                            <el-radio-group v-model="formState.baseData.sex">
                                                <el-radio :value="1">{{ $t("男") }}</el-radio>
                                                <el-radio :value="2">{{ $t("女") }}</el-radio>
                                            </el-radio-group>
                                        </div>
                                    </el-form-item>
                                    <div class="title-item">
                                        {{ formState.baseData.type == 1 ? $t("个人证件照") : $t("法定代表人证件照") }}
                                    </div>
                                    <el-form-item
                                        :rules="[{ required: true, type: 'array', message: $t('请上传证件照正面') }]"
                                        class="form-item"
                                        :label="$t('证件照正面')"
                                        prop="baseData.frontOfPhoto"
                                    >
                                        <div class="input">
                                            <UploadImage
                                                v-if="loading"
                                                v-model:uploadList="formState.baseData.frontOfPhoto"
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
                                        :rules="[{ required: true, type: 'array', message: $t('请上传证件照反面') }]"
                                        class="form-item"
                                        :label="$t('证件照反面')"
                                        prop="baseData.backOfPhoto"
                                    >
                                        <div class="input">
                                            <UploadImage
                                                v-if="loading"
                                                v-model:uploadList="formState.baseData.backOfPhoto"
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
                                </template>
                                <template v-else-if="currentStep == 2">
                                    <div class="title-item">{{ $t("联系人信息") }}</div>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入联系人姓名') }]"
                                        class="form-item"
                                        :label="$t('联系人姓名')"
                                        prop="merchantData.contactName"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.merchantData.contactName" class="item-width"></el-input>
                                        </div>
                                        <div class="info-tips">{{ $t("请填写联系人姓名") }}</div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入联系人手机') }]"
                                        class="form-item"
                                        :label="$t('联系人手机')"
                                        prop="merchantData.contactPhone"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.merchantData.contactPhone" class="item-width"></el-input>
                                        </div>
                                        <div class="info-tips">{{ $t("非常重要，此手机号将作为后续登录商户系统的账号") }}</div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入联系人邮箱') }]"
                                        class="form-item"
                                        :label="$t('联系人邮箱')"
                                        prop="merchantData.contactEmail"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.merchantData.contactEmail" class="item-width"></el-input>
                                        </div>
                                    </el-form-item>
                                    <div class="title-item">{{ $t("结算账户信息") }}</div>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入开户银行') }]"
                                        class="form-item"
                                        :label="$t('开户银行')"
                                        prop="merchantData.bankDeposit"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.merchantData.bankDeposit" class="item-width"></el-input>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入支行名称') }]"
                                        class="form-item"
                                        :label="$t('支行名称')"
                                        prop="merchantData.bankBranch"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.merchantData.bankBranch" class="item-width"></el-input>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :label="formState.baseData.type == 2 ? $t('对公银行账户') : $t('银行卡号')"
                                        :rules="[
                                            {
                                                required: true,
                                                message: $t('请输入') + (formState.baseData.type == 2 ? $t('对公银行账户') : $t('银行卡号')) + '!'
                                            }
                                        ]"
                                        class="form-item"
                                        prop="merchantData.bankCard"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.merchantData.bankCard" class="item-width"></el-input>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        v-if="formState.baseData.type == 2"
                                        :rules="[{ required: true, type: 'array', message: $t('请上传开户许可证') }]"
                                        class="form-item"
                                        :label="$t('开户许可证')"
                                        prop="merchantData.accountOpeningPermit"
                                    >
                                        <div class="input">
                                            <UploadImage
                                                v-if="loading"
                                                v-model:uploadList="formState.merchantData.accountOpeningPermit"
                                                :fileTypes="['jpeg', 'png', 'jpeg']"
                                                :limit="1"
                                            ></UploadImage>
                                        </div>
                                        <div class="info-tips">
                                            <p>
                                                {{
                                                    $t(
                                                        "开户许可证/基本存款账户任选其一上传，信息需与填写的结算账户信息一致，如无法提供，上传银行对公账户开户业务办理回执，或网银截图，或企业盖章的对公账户信息文件即可"
                                                    )
                                                }}
                                            </p>
                                        </div>
                                    </el-form-item>
                                    <div class="title-item">{{ $t("商户信息") }}</div>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入商户名称') }]"
                                        class="form-item"
                                        :label="$t('商户名称')"
                                        prop="merchantData.merchantName"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.merchantData.merchantName" class="item-width"></el-input>
                                        </div>
                                        <div class="info-tips">{{ $t("该名称将在微信支付、支付宝支付时向买家展示") }}</div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入客服电话') }]"
                                        class="form-item"
                                        :label="$t('客服电话')"
                                        prop="merchantData.customerServicePhone"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.merchantData.customerServicePhone" class="item-width"></el-input>
                                        </div>
                                        <div class="info-tips">
                                            {{ $t("请注意填写格式，举例（座机：0755-88880000；手机：13688880000；400电话：4008880000）") }}
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请选择经营地址') }]"
                                        class="form-item"
                                        :label="$t('经营地址')"
                                        prop="merchantData.businessAddress"
                                    >
                                        <div class="input">
                                            <SelectRegion
                                                v-if="loading"
                                                v-model="formState.merchantData.businessAddress"
                                                @change="onBusinessAddress"
                                            ></SelectRegion>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入详细地址') }]"
                                        class="form-item"
                                        :label="$t('详细地址')"
                                        prop="merchantData.detailedAddress"
                                    >
                                        <div class="input">
                                            <el-input
                                                v-model="formState.merchantData.detailedAddress"
                                                class="item-width"
                                                :placeholder="$t('请输入详细地址')"
                                                type="textarea"
                                            ></el-input>
                                        </div>
                                        <div class="info-tips">{{ $t("请填写详细的经营场所地址，如有多个场所，选择一个主要场所填写即可") }}</div>
                                    </el-form-item>
                                    <el-form-item
                                        v-if="isBusinessSubject"
                                        :rules="[{ required: true, type: 'array', message: $t('请上传营业执照') }]"
                                        class="form-item"
                                        :label="$t('营业执照电子版')"
                                        prop="merchantData.businessLicenseImg"
                                    >
                                        <div class="input">
                                            <UploadImage
                                                v-if="loading"
                                                v-model:uploadList="formState.merchantData.businessLicenseImg"
                                                :fileTypes="['jpeg', 'png', 'bmp', 'pdf']"
                                                accept=".jpg,.jpeg,.png,.bmp,.pdf"
                                                :max-size="10"
                                                :limit="1"
                                                api-url="merchant/merchant/uploadLicense"
                                                response-data-type="url"
                                                @uploading-change="licenseUploading = $event"
                                            ></UploadImage>
                                        </div>
                                        <div class="info-tips">{{ $t("支持 JPG、JPEG、PNG、BMP、PDF，单个文件不超过10MB；PDF最多10页且须未加密，超大页面请缩小后上传。若为复印件，需加盖公司红章。") }}</div>
                                    </el-form-item>
                                    <div class="title-item">{{ shopInfoLabel }}</div>
                                    <el-form-item
                                        :rules="[{ required: true, message: shopNameRequiredMessage }]"
                                        class="form-item"
                                        :label="shopNameLabel"
                                        prop="shopData.shopTitle"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.shopData.shopTitle" class="item-width"></el-input>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: shopLogoRequiredMessage }]"
                                        class="form-item"
                                        :label="shopLogoLabel"
                                        prop="shopData.shopLogo"
                                    >
                                        <div class="input">
                                            <UploadImage
                                                v-if="loading"
                                                v-model:uploadList="formState.shopData.shopLogo"
                                                :fileTypes="['jpeg', 'png', 'jpeg']"
                                                :limit="1"
                                            ></UploadImage>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入联系电话') }]"
                                        class="form-item"
                                        :label="$t('联系电话')"
                                        prop="shopData.contactMobile"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.shopData.contactMobile" class="item-width"></el-input>
                                        </div>
                                    </el-form-item>
                                    <el-form-item class="form-item" :label="shopDescriptionLabel" prop="shopData.description">
                                        <div class="input">
                                            <el-input
                                                :autosize="{ minRows: 2, maxRows: 6 }"
                                                v-model="formState.shopData.description"
                                                class="item-width"
                                                type="textarea"
                                            ></el-input>
                                        </div>
                                    </el-form-item>
                                    <template v-if="formState.baseData.type == 1">
                                        <div class="title-item">{{ $t("补充信息") }}</div>
                                        <el-form-item class="form-item" :label="$t('补充信息')">
                                            <div class="input">
                                                <UploadImage
                                                    v-if="loading"
                                                    v-model:uploadList="formState.baseData.supplementaryInformation"
                                                    :fileTypes="['jpeg', 'png', 'jpeg']"
                                                    :limit="5"
                                                ></UploadImage>
                                            </div>
                                            <div class="info-tips">
                                                <p>{{ $t("在整个申请流程中，遇到需提供资料但无上传入口的情况，都可在此处上传。") }}</p>
                                                <p>{{ $t("必须为彩色图片（文档请截图后上传），最多5张，单张小于2M。") }}</p>
                                            </div>
                                        </el-form-item>
                                    </template>
                                </template>
                                <template v-else-if="currentStep == 3 && isBusinessSubject">
                                    <div class="title-item">{{ $t("企业信息") }}</div>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入企业名称') }]"
                                        class="form-item"
                                        :label="$t('企业名称')"
                                        prop="baseData.companyName"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.baseData.companyName" class="item-width"></el-input>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请选择注册地址') }]"
                                        class="form-item"
                                        :label="$t('注册地址')"
                                        prop="baseData.licenseAddrProvince"
                                    >
                                        <div class="input">
                                            <SelectRegion
                                                v-if="loading"
                                                v-model="formState.baseData.licenseAddrProvince"
                                                @change="onLicenseAddrProvince"
                                            ></SelectRegion>
                                        </div>
                                    </el-form-item>
                                    <el-form-item
                                        :rules="[{ required: true, message: $t('请输入详细地址') }]"
                                        class="form-item"
                                        :label="$t('详细地址')"
                                        prop="baseData.businessLicenseAddress"
                                    >
                                        <div class="input">
                                            <el-input
                                                :autosize="{ minRows: 2, maxRows: 6 }"
                                                v-model="formState.baseData.businessLicenseAddress"
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
                                        prop="baseData.businessScope"
                                    >
                                        <div class="input">
                                            <el-input
                                                :autosize="{ minRows: 2, maxRows: 6 }"
                                                v-model="formState.baseData.businessScope"
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
                                        prop="baseData.businessLicenseId"
                                    >
                                        <div class="input">
                                            <el-input v-model="formState.baseData.businessLicenseId" class="item-width"></el-input>
                                        </div>
                                        <div class="info-tips">{{ $t("请输入营业执照上18位统一社会信用代码") }}</div>
                                    </el-form-item>
                                    <el-form-item class="form-item">
                                        <template #label> <span class="required">*</span> {{ $t("营业期限") }}： </template>
                                        <div class="one-item">
                                            <div class="to-time">
                                                <el-radio-group v-model="formState.baseData.operatingTermType">
                                                    <el-radio :value="1">{{ $t("区间有效") }}</el-radio>
                                                </el-radio-group>
                                                <el-form-item
                                                    :rules="formState.baseData.operatingTermType === 1 ? [{ required: true, message: $t('请选择日期') }] : []"
                                                    label=""
                                                    prop="baseData.operatingTermTypeDate"
                                                >
                                                    <el-date-picker
                                                        v-model="formState.baseData.operatingTermTypeDate"
                                                        :clearable="false"
                                                        :disabled="formState.baseData.operatingTermType === 2"
                                                        :end-placeholder="$t('结束日期')"
                                                        range-separator="～"
                                                        :start-placeholder="$t('开始日期')"
                                                        type="daterange"
                                                        value-format="YYYY-MM-DD"
                                                    />
                                                </el-form-item>
                                            </div>
                                            <div class="to-time">
                                                <el-radio-group v-model="formState.baseData.operatingTermType">
                                                    <el-radio :value="2">{{ $t("长期有效") }}</el-radio>
                                                </el-radio-group>
                                                <el-form-item
                                                    :rules="formState.baseData.operatingTermType === 2 ? [{ required: true, message: $t('请选择日期') }] : []"
                                                    label=""
                                                    prop="baseData.operatingTermTypeEnd"
                                                >
                                                    <el-date-picker
                                                        v-model="formState.baseData.operatingTermTypeEnd"
                                                        :clearable="false"
                                                        :disabled="formState.baseData.operatingTermType === 1"
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
                                    <template v-if="isBusinessSubject">
                                        <div class="title-item">{{ $t("补充信息") }}</div>
                                        <el-form-item class="form-item" :label="$t('补充信息')">
                                            <div class="input">
                                                <UploadImage
                                                    v-if="loading"
                                                    v-model:uploadList="formState.baseData.supplementaryInformation"
                                                    :fileTypes="['jpeg', 'png', 'jpeg']"
                                                    :limit="5"
                                                ></UploadImage>
                                            </div>
                                            <div class="info-tips">
                                                <p>{{ $t("在整个申请流程中，遇到需提供资料但无上传入口的情况，都可在此处上传。") }}</p>
                                                <p>{{ $t("必须为彩色图片（文档请截图后上传），最多5张，单张小于2M。") }}</p>
                                            </div>
                                        </el-form-item>
                                    </template>
                                </template>
                                <template v-else-if="(currentStep == 3 && formState.baseData.type == 1) || (currentStep == 4 && isBusinessSubject)">
                                    <el-form class="form-body" label-suffix="：" label-width="auto">
                                        <div v-if="formState.status == 1" class="remind-box">
                                            {{ $t("申请已受理") }}（{{ formState.addTime }}），{{ $t("将在1-3个工作日内完成审核。以下为本次申请内容") }}：
                                        </div>
                                        <!-- <div v-if="formState.status == 10" class="remind-box">
                                            {{ $t("恭喜您，您的申请审核") }}<span class="green">{{ $t("已通过") }}</span
                                            >。
                                            <br />
                                            <div v-if="formState.auditRemark">{{ $t("备注") }}：{{ formState.auditRemark }}</div>
                                        </div> -->
                                        <div v-if="formState.status == 20" class="remind-box">
                                            {{ $t("很遗憾，您的申请审核") }}<span class="red">{{ $t("未通过") }}</span
                                            >。
                                            <el-button class="ml10" type="primary" @click="review">{{ $t("重新审核") }}</el-button>
                                            <br />
                                            <div v-if="formState.auditRemark">{{ $t("备注") }}：{{ formState.auditRemark }}</div>
                                        </div>
                                        <div class="title-item">{{ $t("基本信息") }}</div>
                                        <div class="sub-title-item">{{ $t("联系人信息") }}</div>
                                        <el-form-item class="form-item" :label="$t('联系人姓名')">
                                            <div class="input">
                                                {{ formState.merchantData.contactName }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('联系人手机号')">
                                            <div class="input">
                                                {{ maskString(formState.merchantData.contactPhone) }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('联系人邮箱')">
                                            <div class="input">
                                                {{ maskString(formState.merchantData.contactEmail) }}
                                            </div>
                                        </el-form-item>
                                        <div class="sub-title-item">{{ $t("结算账户信息") }}</div>
                                        <el-form-item :label="formState.baseData.type == 1 ? $t('持卡人') : $t('对公账户')" class="form-item">
                                            <div class="input">
                                                <template v-if="formState.baseData.type == 1">
                                                    {{ maskStringOne(formState.baseData.corporateName) }}
                                                </template>
                                                <template v-else>
                                                    {{ maskStringOne(formState.baseData.companyName) }}
                                                </template>
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('开户银行')">
                                            <div class="input">
                                                {{ formState.merchantData.bankDeposit }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('开户支行')">
                                            <div class="input">
                                                {{ formState.merchantData.bankBranch }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item :label="formState.baseData.type == 2 ? $t('对公银行账号') : $t('银行卡号')" class="form-item">
                                            <div class="input">
                                                {{ maskString(formState.merchantData.bankCard) }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item v-if="formState.baseData.type == 2" class="form-item" :label="$t('开户许可证')">
                                            <div class="input">
                                                <template v-if="formState.merchantData.accountOpeningPermit?.length > 0">
                                                    <el-image
                                                        :src="imageFormat(formState.merchantData.accountOpeningPermit[0].picUrl)"
                                                        class="image-size"
                                                        fit="cover"
                                                        @click="showImage(imageFormat(formState.merchantData.accountOpeningPermit[0].picUrl))"
                                                    ></el-image>
                                                </template>
                                            </div>
                                        </el-form-item>
                                        <div class="title-item">{{ $t("经营信息") }}</div>
                                        <div class="sub-title-item">{{ $t("商户信息") }}</div>
                                        <el-form-item class="form-item" :label="$t('商户名称')">
                                            <div class="input">
                                                {{ formState.merchantData.merchantName }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('客服电话')">
                                            <div class="input">
                                                {{ maskString(formState.merchantData.customerServicePhone) }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('经营地址')">
                                            <div class="input">
                                                {{ formState.merchantData.businessAddressName }}{{ formState.merchantData.detailedAddress }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item v-if="isBusinessSubject" class="form-item" :label="$t('营业执照')">
                                            <div class="input">
                                                <template v-if="formState.merchantData.businessLicenseImg?.length > 0">
                                                    <el-image
                                                        :src="imageFormat(formState.merchantData.businessLicenseImg[0].picUrl)"
                                                        class="image-size"
                                                        fit="cover"
                                                        @click="showImage(imageFormat(formState.merchantData.businessLicenseImg[0].picUrl))"
                                                    ></el-image>
                                                </template>
                                            </div>
                                        </el-form-item>
                                        <div class="sub-title-item">{{ shopInfoLabel }}</div>
                                        <el-form-item class="form-item" :label="shopNameLabel">
                                            <div class="input">
                                                {{ formState.shopData.shopTitle }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="shopLogoLabel">
                                            <div class="input">
                                                <template v-if="formState.shopData.shopLogo?.length > 0">
                                                    <el-image
                                                        :src="imageFormat(formState.shopData.shopLogo[0].picUrl)"
                                                        class="image-size"
                                                        fit="cover"
                                                        @click="showImage(imageFormat(formState.shopData.shopLogo[0].picUrl))"
                                                    ></el-image>
                                                </template>
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('联系电话')">
                                            <div class="input">
                                                {{ maskString(formState.shopData.contactMobile) }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item v-if="formState.shopData.description" class="form-item" :label="shopDescriptionLabel">
                                            <div class="input">
                                                {{ formState.shopData.description }}
                                            </div>
                                        </el-form-item>

                                        <div class="title-item">{{ $t("主体信息") }}</div>
                                        <div class="sub-title-item">
                                            {{ formState.baseData.type == 1 ? $t("个人信息") : $t("法定代表人信息") }}
                                        </div>
                                        <el-form-item :label="formState.baseData.type == 1 ? $t('姓名') : $t('法定代表人姓名')" class="form-item">
                                            <div class="input">
                                                {{ maskStringOne(formState.baseData.corporateName) }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('证件类型')">
                                            <div class="input">
                                                {{ $t(documentTypeOptions.find((option) => option.value === formState.baseData.documentType)?.label || "") }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('证件号码')">
                                            <div class="input">
                                                {{ maskString(formState.baseData.documentNumber) }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('证件有效期')">
                                            <div class="input">
                                                <template v-if="formState.baseData.certificateValidityPeriod == 1">
                                                    {{
                                                        formState.baseData.certificateValidityPeriodDate[0] +
                                                        " ～ " +
                                                        formState.baseData.certificateValidityPeriodDate[1]
                                                    }}
                                                </template>
                                                <template v-else> {{ formState.baseData.certificateValidityPeriodEnd }} ～ {{ $t("长期") }} </template>
                                            </div>
                                        </el-form-item>
                                        <el-form-item v-if="formState.baseData.sex" class="form-item" :label="$t('性别')">
                                            <div class="input">
                                                {{ formState.baseData.sex == 1 ? $t("男") : $t("女") }}
                                            </div>
                                        </el-form-item>
                                        <el-form-item class="form-item" :label="$t('居住地址')">
                                            <div class="input">
                                                {{ formState.baseData.residentialAddress }}
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
                                        <template v-if="isBusinessSubject">
                                            <div class="sub-title-item">{{ $t("企业信息") }}</div>
                                            <el-form-item class="form-item" :label="$t('企业名称')">
                                                <div class="input">
                                                    {{ formState.baseData.companyName }}
                                                </div>
                                            </el-form-item>
                                            <el-form-item class="form-item" :label="$t('注册地址')">
                                                <div class="input">
                                                    {{ formState.baseData.licenseAddrProvinceName }} {{ formState.baseData.businessLicenseAddress }}
                                                </div>
                                            </el-form-item>
                                            <el-form-item class="form-item" :label="$t('经营范围')">
                                                <div class="input">
                                                    {{ formState.baseData.businessScope }}
                                                </div>
                                            </el-form-item>
                                            <el-form-item class="form-item" :label="$t('统一社会信用代码')">
                                                <div class="input">
                                                    {{ formState.baseData.businessLicenseId }}
                                                </div>
                                            </el-form-item>
                                            <el-form-item class="form-item" :label="$t('营业期限')">
                                                <div class="input">
                                                    <template v-if="formState.baseData.operatingTermType == 1">
                                                        {{ formState.baseData.operatingTermTypeDate[0] + " ～ " + formState.baseData.operatingTermTypeDate[1] }}
                                                    </template>
                                                    <template v-else> {{ formState.baseData.operatingTermTypeEnd }} ～{{ $t("长期") }} </template>
                                                </div>
                                            </el-form-item>
                                        </template>
                                        <el-form-item v-if="formState.baseData.supplementaryInformation?.length > 0" class="form-item" :label="$t('补充信息')">
                                            <div class="input">
                                                <template v-for="item in formState.baseData.supplementaryInformation">
                                                    <el-image
                                                        :src="imageFormat(item.picUrl)"
                                                        class="image-size"
                                                        fit="cover"
                                                        @click="showImage(imageFormat(item.picUrl))"
                                                    ></el-image>
                                                </template>
                                            </div>
                                        </el-form-item>
                                    </el-form>
                                </template>
                            </el-form>
                            <template v-if="(formState.baseData.type === 1 && currentStep == 4) || (isBusinessSubject && currentStep == 5)">
                                <div class="result_success">
                                    <div class="icon_img"></div>
                                    <div class="desc">{{ $t("完成入驻") }}</div>
                                    <div class="sub_tips" v-if="formState.initialUserInfo?.mobile && formState.initialUserInfo?.initialPassword">
                                        {{
                                            $t("您的账号：{0}，初始密码：{1}，请尽快修改初始密码", [
                                                formState.initialUserInfo?.mobile ?? "",
                                                formState.initialUserInfo?.initialPassword ?? ""
                                            ])
                                        }}
                                    </div>
                                    <div class="bootom_btn_wapper">
                                        <el-button size="large" @click="goTo('home')">{{ $t("返回首页") }}</el-button>
                                        <el-button size="large" type="primary" @click="goTo()">{{ $t("商户后台") }}</el-button>
                                    </div>
                                </div>
                            </template>
                        </div>
                    </div>
                </div>

                <!-- <div v-if="currentStep != (formState.baseData.type == 1 ? 3 : 4)" class="fixed-bottom"></div> -->
                <div v-if="isShowStep" class="fixed-bottom">
                    <el-button v-if="currentStep > 1" size="large" @click="toPage(0)">{{ $t("上一步") }}</el-button>
                    <el-button v-if="currentStep < (formState.baseData.type == 1 ? 3 : 4) - 1" size="large" type="primary" @click="toPage(1)">{{
                        $t("下一步")
                    }}</el-button>
                    <el-button
                        v-if="currentStep == (formState.baseData.type == 1 ? 3 : 4) - 1"
                        size="large"
                        type="primary"
                        :loading="submitting"
                        :disabled="submitting"
                        @click="toPage(9)"
                    >{{
                        $t("提交")
                    }}</el-button>
                </div>
                <el-dialog v-model="dialogVisible">
                    <div class="image">
                        <el-image :src="dialogImageUrl" alt="" fit="cover" />
                    </div>
                </el-dialog>
            </template>
    </div>
    <CommonPageFooter :mt30="false"></CommonPageFooter>
    <template v-if="flow == 2">
        <div style="padding-top: 40px"></div>
    </template>
</template>
<script lang="ts" setup>
import { message } from "ant-design-vue";
import { applyMerchant, getMerchantInfo, getMyMerchant } from "~/api/merchant/merchant";
import { ref } from "vue";
import introTable from "~/pages/join/src/introTable.vue";
import { useRouter } from "vue-router";
import lock from "assets/images/join/lock@3x.png";
import UploadImage from "~/components/form/src/UploadImage.vue";
import { SelectRegion } from "~/components/region";
import Step from "~/pages/join/src/step.vue";
import AgreementWin from "~/pages/join/src/agreementWin.vue";
import { normalizeMerchantApplication, selectMerchantType, merchantReviewStep } from "~/utils/merchantApply.mjs";
import { useCommonStore } from "~/store/common";
import { isOverseas, isDemo, isO2o } from "@/utils/util";
const { t } = useI18n();
const shopInfoLabel = computed(() => (isO2o() ? t("门店信息") : t("店铺信息")));
const shopNameLabel = computed(() => (isO2o() ? t("门店名称") : t("店铺名称")));
const shopLogoLabel = computed(() => (isO2o() ? t("门店logo") : t("店铺logo")));
const shopDescriptionLabel = computed(() => (isO2o() ? t("门店简介") : t("店铺简介")));
const shopNameRequiredMessage = computed(() => (isO2o() ? t("请输入门店名称") : t("请输入店铺名称")));
const shopLogoRequiredMessage = computed(() => (isO2o() ? t("请上传门店logo") : t("请上传店铺logo")));
definePageMeta({
    middleware: "auth"
});
const commonStore = useCommonStore();
const type = ref(commonStore.personApplyEnabled == 1 ? 1 : 2);
const agreement = ref(false);
const loading = ref(false);
const submitting = ref(false);
const licenseUploading = ref(false);
const router = useRouter();
const flow = ref(1);
const currentStep = ref(1);

onMounted(async () => {
    await _getMyMerchant();
    if (apply.value.merchantApplyId) {
        //获取数据
        await getData(apply.value.merchantApplyId);
        flow.value = 2;
    }
});
//表单主体信息
const formStateRef = shallowRef();
const formState: any = ref({
    baseData: {
        certificateValidityPeriod: 1,
        operatingTermType: 1,
        frontOfPhoto: [],
        backOfPhoto: [],
        licenseAddrProvince: [],
        supplementaryInformation: []
    },
    merchantData: {
        businessAddress: [],
        accountOpeningPermit: [],
        businessLicenseImg: []
    },
    shopData: {
        shopLogo: []
    }
});

const isBusinessSubject = computed(() => [2, 3].includes(formState.value.baseData.type));

watch(type, (value) => {
    formState.value = selectMerchantType(formState.value, value);
    formStateRef.value?.clearValidate();
});

const validatePhoto = (rule: any, value: any, callback: any) => {
    if (!value || value.length < 1) {
        callback(new Error(t("请上图片")));
    }
    callback();
};

const apply = ref({
    merchantApplyId: 0,
    status: 0,
    type: 0
});
const _getMyMerchant = async () => {
    try {
        loading.value = false;
        const result = await getMyMerchant();
        if (result) {
            Object.assign(apply.value, result);
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
        const result = await getMerchantInfo(id);
        formState.value = normalizeMerchantApplication(result);
        type.value = formState.value.baseData.type;
        currentStep.value = merchantReviewStep(type.value, result.status);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = true;
    }
};
//开始申请
const startApplying = () => {
    if (!agreement.value) {
        message.error(t("请同意开通协议"));
        return;
    }
    loading.value = false;
    flow.value = 2;
    formState.value = selectMerchantType(formState.value, type.value);
    currentStep.value = 1;
    loading.value = true;
};

const toPage = async (type: number) => {
    if (type == 9 && submitting.value) return;
    if (type == 1 || type == 9) {
        try {
            // 提交时尽早加锁，避免校验等待期间重复点击
            if (type == 9) submitting.value = true;
            await formStateRef.value.validate(); // 校验当前页面，无论何种情况都会进行
            if (type == 1) {
                currentStep.value = currentStep.value + 1; // 当 type 为1时，无论表单类型为何种都需要执行的操作
            } else if (type == 9) {
                await onSubmit();
            }
        } catch {
            // 表单校验未通过
            if (type == 9) submitting.value = false;
        }
    } else {
        currentStep.value = currentStep.value - 1;
    }
};

const toUpPage = () => {
    flow.value = 1;
};

//日期选择校验重置
watch(
    () => formState.value.baseData.certificateValidityPeriod,
    (val) => {
        if (val == 1) {
            formStateRef.value?.clearValidate("baseData.certificateValidityPeriodEnd");
        } else {
            formStateRef.value?.clearValidate("baseData.certificateValidityPeriodDate");
        }
    },
    { deep: true }
);

watch(
    () => formState.value.baseData.operatingTermType,
    (val) => {
        if (val == 1) {
            formStateRef.value?.clearValidate("baseData.operatingTermTypeEnd");
        } else {
            formStateRef.value?.clearValidate("baseData.operatingTermTypeDate");
        }
    },
    { deep: true }
);
// ====
//地区选择校验
const onBusinessAddress = () => {
    formStateRef.value.validateField("businessAddress");
};
const onLicenseAddrProvince = () => {
    formStateRef.value.validateField("baseData.licenseAddrProvince");
};
//=====
//身份证自动填充
watch(
    () => formState.value.baseData.documentNumber,
    (val) => {
        if (formState.value.baseData.documentType == 1 && val?.length >= 18) {
            let temp = val.substr(6, 8);
            formState.value.baseData.birthday = temp.slice(0, 4) + "-" + temp.slice(4, 6) + "-" + temp.slice(6, 8);
            let genderIndicator = parseInt(val.substr(val.length - 2, 1), 10);
            formState.value.baseData.sex = genderIndicator % 2 === 0 ? 2 : 1;
        }
    },
    { deep: true }
);

//提交申请
const onSubmit = async () => {
    if (licenseUploading.value) {
        submitting.value = false;
        return message.warning(t("正在上传..."));
    }
    if (isDemo()) {
        submitting.value = false;
        return message.error(t("权限不足"));
    }

    try {
        const result: any = await applyMerchant(formState.value);
        message.success(t("申请成功"));
        currentStep.value = formState.value.baseData.type == 1 ? 3 : 4;
        await getData(result.merchantApplyId);
        //成功后获取最新数据
    } catch (error: any) {
        message.error(error.message);
    } finally {
        submitting.value = false;
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
    flow.value = 1;
    type.value = formState.value.baseData.type;
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

// 入驻完成底部按钮页面跳转事件
const goTo = (type?: string) => {
    let url = commonStore.adminDomain ? commonStore.adminDomain : `${location.origin}/admin/`;
    if (!type) {
        window.open(url);
    } else {
        // 回首页，navigateTo 会自动带上 app.baseURL，兼容子路径部署
        navigateTo("/");
    }
};

const isShowStep = computed(() => {
    if (formState.value.baseData.type == 1) {
        return currentStep.value < 3;
    } else if (isBusinessSubject.value) {
        return currentStep.value < 4;
    }
});
</script>
<style lang="less" scoped>
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
    gap: 26px;
    align-items: center;
    flex-direction: column;
    padding: 26px 0;

    .card {
        background-color: #ffffff;
        display: flex;
        padding: 24px;
        flex-direction: column;
        align-items: center;

        .form-info {
            display: flex;
            flex-direction: column;
            width: 100%;
            margin: 30px 0;

            .title-item {
                font-weight: 500;
                font-size: 14px;
                line-height: 20px;
                padding-bottom: 10px;
                border-bottom: 1px solid #e0e0e0;
                color: #333;
            }

            .sub-title-item {
                margin: 20px 0;
                font-weight: 500;
                font-size: 14px;
                line-height: 20px;
                padding-bottom: 10px;
                color: #333;
            }

            .result_success {
                display: flex;
                flex-direction: column;
                justify-content: center;
                align-items: center;
                width: 100%;
                height: fit-content;

                .icon_img {
                    width: 110px;
                    height: 150px;
                    margin-top: 60px;
                    background: url("/assets/images/join/join_success.png");
                    background-size: 100%;
                    background-repeat: no-repeat;
                }

                .desc {
                    margin-top: 20px;
                    font-size: 20px;
                    color: #000;
                }

                .sub_tips {
                    margin-top: 10px;
                }

                .bootom_btn_wapper {
                    display: flex;
                    column-gap: 15px;
                    margin-top: 20px;
                }
            }
        }
    }
}

.form-item {
    margin: 20px 60px;

    .input {
        width: 100%;
        font-size: 12px;
    }
}

.item-width {
    width: 280px;
}

.info-tips {
    font-size: 12px;
    color: #969799;
    vertical-align: baseline;
    cursor: pointer;
    font-weight: 400;
}

.fixed-bottom {
    position: fixed;
    bottom: 0;
    width: 100%;
    padding: 20px 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #ffffff;
    z-index: 9;
    transition: right 0.5s;
    box-shadow: 0 -3px 5px #eee;
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
