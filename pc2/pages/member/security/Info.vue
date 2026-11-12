<template>
    <!-- <CommonHeader title="账户安全"></CommonHeader> -->
    <CommonPageHeaderNew :isWhite="true"></CommonPageHeaderNew>
    <CommonHeader title="账户安全"></CommonHeader>
    <TopJumpBar tltle="账户安全"></TopJumpBar>
    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-rowSecuity">
            <div class="title-or-tabs">
                <span>{{ $t("账户安全") }}</span>
            </div>
            <div class="security-info">
                <ul class="table">
                    <li class="col" @click="openVerification(formState.mobile ? 1 : 3)">
                        <div class="title">{{ $t("手机") }}</div>
                        <!-- <div class="input-box" v-if="formState.mobile">
                            <div class="account">{{ formState.mobile ? phoneEncrypt(formState.mobile) : "" }}</div>
                            <VerificationSecurity
                                ref="verificationRef"
                                :params="{ title: $t('手机换绑'), mobile: formState.mobile, type: 1 }"
                                @loadFilter="getUserInfo"
                            >
                                <div class="action-box block-color">{{ $t("换绑") }}</div>
                            </VerificationSecurity>
                            <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                        </div>
                        <div v-else class="input-box">
                            <div class="account">{{ formState.mobile ? phoneEncrypt(formState.mobile) : "" }}</div>
                            <VerificationSecurity
                                ref="verificationRef"
                                :params="{ title: $t('手机绑定'), mobile: formState.mobile, type: 3 }"
                                @loadFilter="getUserInfo"
                            >
                                <div class="action-box block-color">{{ $t("绑定") }}</div>
                            </VerificationSecurity>
                            <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                        </div> -->
                        <div class="input-box">
                            <div class="account">{{ formState.mobile ? phoneEncrypt(formState.mobile) : "" }}</div>
                            <VerificationSecurity
                                :ref="(el: InstanceType<typeof VerificationSecurity>) => setVerificationRef(formState.mobile ? 1 : 3, el)"
                                :params="{ title: $t(formState.mobile ? '手机换绑' : '手机绑定'), mobile: formState.mobile, type: formState.mobile ? 1 : 3 }"
                                @loadFilter="getUserInfo"
                            >
                                <div class="action-box block-color">{{ $t(formState.mobile ? "换绑" : "绑定") }}</div>
                            </VerificationSecurity>
                            <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                        </div>
                    </li>
                    <li class="col" @click="openVerification(formState.email ? 4 : 5)">
                        <div class="title">{{ $t("邮箱") }}</div>
                        <!-- <div class="input-box" v-if="formState.email">
                            <div class="account">{{ formState.email }}</div>
                            <VerificationSecurity
                                ref="verificationRef"
                                :params="{ title: $t('邮箱换绑'), email: formState.email, type: 4 }"
                                @loadFilter="getUserInfo"
                            >
                                <div class="action-box block-color">{{ $t("换绑") }}</div>
                            </VerificationSecurity>
                            <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                        </div>
                        <div v-else class="input-box">
                            <div class="account">{{ formState.email }}</div>
                            <VerificationSecurity
                                ref="verificationRef"
                                :params="{ title: $t('邮箱绑定'), email: formState.email, type: 5 }"
                                @loadFilter="getUserInfo"
                            >
                                <div class="action-box block-color">{{ $t("绑定") }}</div>
                            </VerificationSecurity>
                            <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                        </div> -->
                        <div class="input-box">
                            <div class="account">{{ formState.email ? phoneEncrypt(formState.email) : "" }}</div>
                            <VerificationSecurity
                                :ref="(el: InstanceType<typeof VerificationSecurity>) => setVerificationRef(formState.email ? 4 : 5, el)"
                                :params="{ title: $t(formState.email ? '邮箱换绑' : '邮箱绑定'), email: formState.email, type: formState.email ? 4 : 5 }"
                                @loadFilter="getUserInfo"
                            >
                                <div class="action-box block-color">{{ $t(formState.email ? "换绑" : "绑定") }}</div>
                            </VerificationSecurity>
                            <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                        </div>
                    </li>
                    <li class="col" @click="openVerification(2)">
                        <div class="title">{{ $t("登录密码") }}</div>
                        <!-- <div class="input-box" v-if="formState.mobile">
                            <div class="account"></div>
                            <VerificationSecurity
                                ref="verificationRef"
                                :params="{ title: $t('密码修改'), mobile: formState.mobile, type: 2 }"
                                @loadFilter="getUserInfo"
                            >
                                <div class="action-box block-color">{{ $t("修改") }}</div>
                            </VerificationSecurity>
                            <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                        </div> -->
                        <div class="input-box">
                            <div class="account"></div>
                            <VerificationSecurity
                                :ref="(el: InstanceType<typeof VerificationSecurity>) => setVerificationRef(2, el)"
                                :params="{ title: $t('密码修改'), mobile: formState.mobile, type: 2 }"
                                @loadFilter="getUserInfo"
                            >
                                <div class="action-box block-color">{{ $t("修改") }}</div>
                            </VerificationSecurity>
                            <SvgIcon name="rightIcon" width="24px" height="24px"></SvgIcon>
                        </div>
                    </li>
                </ul>
            </div>
            <div class="security-info" style="margin-top: 20px;">
                <div class="bordered-div">
                    <div class="title">{{ $t("安全服务提示") }}</div>
                    <p>{{ $t("1. 注意防范进入钓鱼网站，不要轻信各种即时通讯工具发送的商品或支付链接，谨防网购诈骗") }}。</p>
                    <p>{{ $t("2. 建议您安装杀毒软件，并定期更新操作系统等软件补丁，确保账户及交易安全") }}。</p>
                </div>
            </div>
        </div>
    </CommonPageContainer>
    <!-- <CommonPageFooter></CommonPageFooter> -->
</template>
<script lang="ts" setup>
/**
 * @description 账户安全中心页面
 * - 显示当前绑定的手机号、邮箱等安全信息
 * - 提供修改密码、换绑手机、换绑邮箱等安全操作入口
 * - 各安全操作通过弹窗方式进行多步骤验证
 */
import { ref } from "vue";
import type { UserFormState } from "~/types/user/user";
import { getUser } from "~/api/user/user";
import VerificationSecurity from "~/pages/member/security/src/VerificationSecurity.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
definePageMeta({
    middleware: "auth"
});
const formState = ref<UserFormState>({} as UserFormState);
const verificationRefs = ref<Record<number, InstanceType<typeof VerificationSecurity>>>({});
const getUserInfo = async () => {
    try {
        const result = await getUser();
        Object.assign(formState.value, result);
    } catch (error: any) {
        message.error(error.message);
    }
};

const phoneEncrypt = (phone: string) => {
    return phone.replace(/^(\d{3})\d{4}(\d{4})$/, "$1****$2").replace(/^(\d{3})\d{4}(\d{4})$/, "$1****$2");
};

const setVerificationRef = (type: number, el: InstanceType<typeof VerificationSecurity>) => {
    if (el) {
        verificationRefs.value[type] = el;
    }
};
const openVerification = (type: number) => {
    verificationRefs.value[type]?.showDialog();
};

const { t } = useI18n();
onMounted(() => {
    getUserInfo();
});
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.security-info {
    background: #fff;
    border: 0;

    .table:last-child {
        display: flex;
        flex-direction: column;

        .col {
            height: 60px;
            border-radius: 8px;
            background-color: rgb(247, 248, 252);
            margin-bottom: 10px;
            display: flex;
            -webkit-box-pack: justify;
            justify-content: space-between;
            -webkit-box-align: center;
            align-items: center;
            cursor: pointer;
            padding: 0 10px;

            .title {
                font-size: 14px;
                width: 120px;
                color: #1a1a1a;
            }

            .input-box {
                width: 100%;
                flex: 1 1 auto;
                display: flex;
                align-items: center;

                .account {
                    flex: 1 1 auto;
                    font-size: 14px;
                    color: #8a919f;
                }

                .action-box {
                    margin-left: 16px;
                    white-space: nowrap;
                    font-weight: bold;
                    font-size: 14px;
                    cursor: pointer;
                }
            }
        }
    }
}

.bordered-div {
    position: relative; /* 设置相对定位 */
    border: 1px solid #ddd; /* 黑色边框 */
    width: 100%; /* 设置 div 宽度 */
    padding: 20px;
    line-height: 3;
    box-sizing: border-box;
}

.bordered-div .title {
    position: absolute; /* 绝对定位标题 */
    top: -1.5em; /* 将标题向上移动，使其部分在 div 外 */
    left: 20px; /* 标题距离左侧 20px */
    background: white; /* 背景色设置为白色，与页面背景相同 */
    // padding: 0 15px; /* 标题内边距 */
    color: #666666;
    font-weight: bold;
    font-size: 12px;
}

/* 使用伪元素来创建顶部边框的断开效果 */

.bordered-div::before {
    content: ""; /* 伪元素内容为空 */
    position: absolute;
    top: 0;
    left: 15px; /* 略小于标题的左边距，确保平滑过渡 */
    width: calc(20px + 10px); /* 标题左边距加上标题内边距的总和 */
    height: 2px; /* 与边框厚度相同 */
    background: white; /* 背景色设置为白色，以覆盖边框 */
}
.info-rowSecuity {
    width: 100%;
    height: fit-content;
    background: #fff;
    padding: 0 32px 16px;
    border-radius: 8px;
    overflow: hidden;
    .title-or-tabs {
        display: flex;
        align-items: center;
        justify-content: space-between;
        height: 72px;
       
        span {
            color: #000;
            font-size: 24px;
            font-weight: 600;
            line-height: 24px;
            letter-spacing: 0;
            display: inline-block;
        }
    }
}
.block-color {
    color: #1a1a1a;
}
</style>
