<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <CommonHeader title="个人资料"></CommonHeader>
    <div class="flex">
        <!-- <div class="menu">
            <MemberNav></MemberNav>
        </div> -->
        <div class="info-rowMember">
            <!-- <div class="title-or-tabs">
                <span>{{ $t("用户信息") }}</span>
            </div> -->
            <div data-form-rev="cWlxaTIyODI" class="profile-info">
                <!-- <div class="title">
                    <h4>{{ $t("请在这里填写准确的个人信息") }}</h4>
                </div> -->

                <el-form ref="formRef" label-position="top" label-width="auto" :model="formState" class="form-body" label-suffix="：">
                    <el-form-item>
                        <div class="image-div">
                            <el-upload
                                :action="requestPrefix + '/user/user/modifyAvatar'"
                                :headers="uploadHeaders"
                                :on-success="handleAvatarSuccess"
                                :show-file-list="false"
                                class="avatar-uploader"
                            >
                                <el-image :src="imageFormat(formState.avatar)" fit="cover" class="el-image" />
                            </el-upload>
                        </div>
                    </el-form-item>
                    <el-form-item :rules="[{ required: true, message: $t('请选择生日') }]" :label="$t('生日')" prop="birthday">
                        <el-date-picker
                            style="width: 100%"
                            v-model="formState.birthday"
                            clearable
                            :placeholder="$t('请选择生日')"
                            type="date"
                            value-format="YYYY-MM-DD"
                            size="large"
                        />
                    </el-form-item>
                    <el-form-item :rules="[{ required: true, message: $t('请输入昵称') }]" :label="$t('昵称')" prop="nickname">
                        <el-input v-model="formState.nickname" clearable :placeholder="$t('请输入昵称')" size="large" />
                    </el-form-item>
                    <!-- <el-form-item :label="$t('手机')">
                        <span style="font-size: 12px">
                            <template v-if="formState.mobile">
                                {{ getMaskedMobile(formState.mobile) }}
                                <NuxtLink class="a-btn" to="/member/security/info">[{{ $t("修改") }}]</NuxtLink>
                            </template>
                            <template v-else>
                                {{ $t("未绑定") }}
                                <NuxtLink class="a-btn" to="/member/security/info">[{{ $t("去绑定") }}]</NuxtLink>
                            </template>
                        </span>
                    </el-form-item>
                    <el-form-item :label="$t('邮箱')">
                        <span style="font-size: 12px">
                            <template v-if="formState.email">
                                {{ getMaskedEmail(formState.email) }}
                                <NuxtLink class="a-btn" to="/member/security/info">[{{ $t("修改") }}]</NuxtLink>
                            </template>
                            <template v-else>
                                {{ $t("未绑定") }}
                                <NuxtLink class="a-btn" to="/member/security/info">[{{ $t("去绑定") }}]</NuxtLink>
                            </template>
                        </span>
                    </el-form-item> -->
                </el-form>
                <!-- <div v-else>
                    <div v-loading="loading" class="no-resultMember"></div>
                </div> -->
                <div class="flex_jce">
                    <el-button @click="cancelClick" size="large">{{ $t("取消") }}</el-button>
                    <el-button type="primary" @click="onSubmit" size="large" :loading="loading">{{ $t(loading ? "保存" : "确认修改") }}</el-button>
                </div>
            </div>
        </div>
    </div>
    <!-- <CommonPageFooter></CommonPageFooter> -->
</template>
<script lang="ts" setup>
/**
 * 个人资料编辑页
 * - 修改昵称、性别、生日、头像等个人信息
 * - 头像支持上传裁剪
 */
import { ref } from "vue";
import type { ProfileFormState } from "~/types/user/profile";
import { getProfile, updateProfile } from "~/api/user/profile";
import type { UploadProps } from "element-plus";
definePageMeta({
    middleware: "auth"
});
const { t } = useI18n();
const loading = ref<boolean>(true);
const formRef = shallowRef();
const formState = ref<ProfileFormState>({});
const uploadHeaders = requestHeader();
const handleAvatarSuccess: UploadProps["onSuccess"] = (response, uploadFile) => {
    formState.value.avatar = URL.createObjectURL(uploadFile.raw!);
};
const fetchProfile = async () => {
    try {
        const result = await getProfile();
        Object.assign(formState.value, result);
        if (result.birthday == "0000-00-00") {
            formState.value.birthday = "";
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const getMaskedEmail = (email: string | undefined) => {
    if (typeof email === "string") {
        // 使用正则表达式来处理邮箱
        return email.replace(/(.{2})(.*)(@.*)/, "$1*****$3");
    }
    return t("未绑定");
};
const getMaskedMobile = (mobile: string | undefined) => {
    if (typeof mobile === "string") {
        // 检查是否包含国际区号 86
        const internationalPrefix = "86";
        if (mobile.startsWith(internationalPrefix)) {
            // 保留国际区号和手机号的后四位
            return mobile.replace(new RegExp(`^(${internationalPrefix})(\\d{14})\\d*(\\d{4})$`), "$1$2*****$3");
        } else {
            // 保留手机号的前三位和后四位
            return mobile.replace(/(\d{3})\d*(\d{4})$/, "$1*****$2");
        }
    }
    return t("未绑定");
};
onMounted(() => {
    // 获取详情数据
    fetchProfile();
});
const emit = defineEmits(["updateValue"]);
const onSubmit = async () => {
    await formRef.value.validate();
    try {
        loading.value = true;
        await updateProfile({ ...formState.value });
        emit("updateValue");
        await fetchProfile();
        message.success(t("修改成功"));
    } catch (error: any) {
    } finally {
        loading.value = false;
    }
};
/**取消 */
const cancelClick = () => {
    emit("updateValue");
};
</script>
<style></style>
<style lang="less" scoped>
@import "/assets/css/member/member";

.profile-info {
    background: #fff;
    border: 0;
    padding: 20px 20px;
    text-transform: none !important;

    .title {
        display: flex;
        justify-content: space-between;
        align-items: center;

        h4 {
            color: var(--general);
            font-size: 14px;
            font-weight: bold;
        }

        .req {
            color: #999999;

            & > b {
                color: #ff4040;
            }
        }
    }

    .form-body {
        .a-btn {
            color: var(--general);
            margin-left: 6px;
        }
        :deep(.el-form-item__label) {
            font-size: 14px;
            color: #1a1a1a;
        }
        :deep(.el-input__inner) {
            font-size: 14px;
            color: #1a1a1a;
        }

        :deep(.el-select__wrapper) {
            font-size: 14px;
            color: #1a1a1a;
        }
    }
}
.flex_jce {
    display: flex;
    justify-content: flex-end;
}
.no-resultMember {
    width: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    font-size: 16px;
    gap: 10px;
}
.info-rowMember {
    width: 100%;
}
.image-div {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    .el-image {
        width: 150px;
        height: 150px;
        border-radius: 50%;
    }
}
</style>
