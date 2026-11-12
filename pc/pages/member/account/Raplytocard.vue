<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>

    <CommonHeader title="提现"></CommonHeader>
    <TopJumpBar tltle="提现"></TopJumpBar>

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
            <div class="account-raplytocard-content">
                <div class="form-container">
                    <el-form ref="formRef" label-width="100px" :model="formState" class="form-body" label-suffix="：">
                        <el-form-item :label="$t('可用余额')">
                            <FormatPrice v-model="userInfo.balance" :fontStyle="{ color: 'var(--price)', fontSize: '16px' }" :showText="false"></FormatPrice>
                        </el-form-item>
                        <el-form-item :label="$t('提现方式')">
                            <div v-if="hasWithdrawalMethod" style="display: flex; gap: 10px">
                                <div
                                    v-for="m in methodList"
                                    :key="m.accountType"
                                    :class="{ selected: formState.accountData.accountType === m.accountType }"
                                    class="card"
                                    @click="onClick(m.accountType)"
                                >
                                    <div>{{ $t(m.title) }}</div>
                                </div>
                            </div>
                            <div v-else class="empty-method">{{ $t("暂无可用提现方式，请联系客服") }}</div>
                        </el-form-item>
                        <el-form-item
                            :rules="[{ required: true, message: $t('请选择') + accountPlaceholder + $t('账号') }]"
                            :label="$t('提现至')"
                            prop="accountData.accountNo"
                        >
                            <el-select
                                v-model="formState.accountData.accountNo"
                                size="large"
                                :placeholder="$t('请选择') + accountPlaceholder + $t('账号')"
                                @change="selectNo"
                            >
                                <el-option
                                    v-for="item in formState.accountData.accountType === 1
                                        ? yhkList
                                        : formState.accountData.accountType === 2
                                          ? zfbList
                                          : wxList"
                                    :key="item.accountId"
                                    :label="item.accountNo"
                                    :value="item.accountNo"
                                />
                            </el-select>
                        </el-form-item>
                        <InfoCard :page="0" :type="formState.accountData.accountType" act="add" @loadFilter="saveOver">
                            <div class="tips">
                                {{ $t("当前{0}账号已不用", [accountPlaceholder]) }}？<span class="font-color" style="cursor: pointer">{{
                                    $t("点击这里，去添加新的")
                                }}</span
                                >{{ accountPlaceholder }}{{ $t("账号") }}。
                            </div>
                        </InfoCard>
                        <el-form-item :label="$t('提现姓名')" prop="accountData.accountName">
                            <el-input
                                v-model="formState.accountData.accountName"
                                size="large"
                                :placeholder="$t('请选择') + accountPlaceholder + $t('账号')"
                                disabled
                            />
                        </el-form-item>
                        <el-form-item v-if="formState.accountData.accountType === 1" :label="$t('银行卡详情')" prop="accountData.bankName">
                            <el-input
                                v-model="formState.accountData.bankName"
                                size="large"
                                :rows="4"
                                disabled
                                :placeholder="$t('银行卡详情')"
                                type="textarea"
                            />
                        </el-form-item>
                        <el-form-item :rules="amountRules" :label="$t('提现金额')" prop="amount">
                            <el-input v-model="formState.amount" size="large" :placeholder="$t('提现金额')" />
                        </el-form-item>
                        <el-form-item v-if="withdrawalDescriptionHtml" class="withdraw-desc-item" :label="$t('提现说明')">
                            <ClientOnly>
                                <div class="withdraw-desc withdraw-note-bjIyODJla3c" v-html="withdrawalDescriptionHtml"></div>
                            </ClientOnly>
                        </el-form-item>
                        <el-form-item label="">
                            <el-button type="primary" size="large" :disabled="!hasWithdrawalMethod" @click="onSubmit">{{ $t("提交") }}</el-button>
                        </el-form-item>
                    </el-form>
                </div>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 余额提现页面
 * - 选择提现方式（银行卡/支付宝/微信等）
 * - 输入提现金额，显示可提现余额
 * - 提交提现申请
 */
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useCommonStore } from "~/store/common";
import { getMemberCenter } from "~/api/user/user";
import type { UserFormState } from "~/types/user/user";
import type { AccountFormState, AccountInfo } from "~/types/user/account";
import { getAccountNoList, updateWithdrawApply } from "~/api/user/account";
import InfoCard from "~/pages/member/account/src/InfoCard.vue";
import { formatRichText, isDemo } from "~/utils/util";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
definePageMeta({
    middleware: "auth"
});
const userInfo = ref<UserFormState>({} as UserFormState);
const getUserInfo = async () => {
    try {
        const result = await getMemberCenter();
        Object.assign(userInfo.value, result);
    } catch (error: any) {
        message.error(error.message);
    }
};
const formRef = shallowRef();
const onClick = async (number: number) => {
    formState.value.accountData.accountType = number;
    await formRef.value.resetFields();
};

onMounted(() => {
    // 会员余额提现关闭时禁止直接访问提现页（D1107），后端 apply 亦会兜底拦截
    if (commonStore.memberWithdrawalEnabled !== 1) {
        message.warning(t("会员余额提现功能已关闭"));
        router.replace("/member/account/detail");
        return;
    }
    // 提现方式按后台勾选项展示（D1107），默认选中第一个已开启方式
    formState.value.accountData.accountType = methodList.value[0]?.accountType ?? 1;
    getUserInfo();
    methodList.value.forEach((m: any) => loadFilter(m.accountType));
});
const saveOver = () => {
    loadFilter(Number(formState.value.accountData.accountType));
};

const yhkList = reactive<AccountInfo[]>([]);
const zfbList = reactive<AccountInfo[]>([]);
const wxList = reactive<AccountInfo[]>([]);

const loadFilter = async (type: number) => {
    try {
        let temp: any = {
            accountType: type
        };
        const result = await getAccountNoList({ ...temp });
        if (type == 1) {
            yhkList.length = 0;
            yhkList.push(...result.records);
        } else if (type === 2) {
            zfbList.length = 0;
            zfbList.push(...result.records);
        } else {
            wxList.length = 0;
            wxList.push(...result.records);
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};

const formState = ref<AccountFormState>({
    accountData: {
        accountType: 1
    }
} as AccountFormState);

const selectNo = (value: any) => {
    const list = formState.value.accountData.accountType === 1 ? yhkList : formState.value.accountData.accountType === 2 ? zfbList : wxList;
    let selectedItem = list.find((item) => item.accountNo === value);
    formState.value.accountData.accountNo = selectedItem?.accountNo as string;
    formState.value.accountData.accountName = selectedItem?.accountName as string;
    formState.value.accountData.bankName = selectedItem?.bankName as string;
};

const accountPlaceholder = computed(() => {
    switch (formState.value.accountData.accountType) {
        case 1:
            return t("银行卡");
        case 2:
            return t("支付宝");
        case 3:
            return t("微信");
        default:
            return "";
    }
});
const { t } = useI18n();
const amountRules = computed(() => {
    return [
        { required: true, message: t("请输入提现金额"), trigger: "blur" }, // 必填验证
        {
            validator: (rule: any, value: any, callback: any) => {
                // 正则表达式验证金额是否为0-12300的两位小数
                const regExp = /^(0|\d{0,9})(\.\d{1,2})?$/;
                if (!regExp.test(value)) {
                    callback(new Error(t("提现金额必须大于0，且最多保留两位小数")));
                } else if (value > Number(userInfo.value.balance)) {
                    callback(new Error(t("提现金额不能高于可用余额")));
                } else {
                    callback();
                }
            },
            trigger: "blur" // 触发方式，可以是 blur、change等
        }
    ];
});

const onSubmit = async () => {
    if (isDemo()) return message.error(t("权限不足"));
    await formRef.value.validate();
    try {
        const result = await updateWithdrawApply({ ...formState.value });
        await formRef.value.resetFields();
        message.success(t("提交成功"));
    } catch (error: any) {
    } finally {
    }
};

const router = useRouter();
const commonStore: any = useCommonStore();
const unescapeRichHtml = (raw: string) => {
    let html = raw;
    // 配置里偶发存成 &lt;p&gt; 转义串，先还原一层再按 HTML 渲染
    if (/&lt;\/?[a-zA-Z]/.test(html)) {
        html = html
            .replace(/&lt;/g, "<")
            .replace(/&gt;/g, ">")
            .replace(/&quot;/g, '"')
            .replace(/&#39;/g, "'")
            .replace(/&amp;/g, "&");
    }
    return html;
};
/** 编辑器颜色打在 li 内 span 上，序号用 li 颜色；把文本色提到 li，默认仍是 #333 */
const applyListMarkerColor = (html: string) => {
    return html.replace(/<li(\b[^>]*)>([\s\S]*?)<\/li>/gi, (full, attrs: string, inner: string) => {
        if (/\sstyle\s*=\s*(['"])[^'"]*color\s*:/i.test(attrs)) return full;
        const colorMatch = inner.match(/color\s*:\s*([^;"']+)/i);
        if (!colorMatch) return full;
        const color = colorMatch[1].trim();
        if (!color || /^(inherit|unset|initial)$/i.test(color)) return full;
        if (/\sstyle\s*=/i.test(attrs)) {
            return `<li${attrs.replace(/style\s*=\s*(['"])/i, `style=$1color: ${color}; `)}>${inner}</li>`;
        }
        return `<li${attrs} style="color: ${color}">${inner}</li>`;
    });
};
// 后台「提现说明」HTML；空内容（含 wangEditor 空段落）不展示。仅图片说明也视为有内容（D1197）
const withdrawalDescriptionHtml = computed(() => {
    const html = unescapeRichHtml(String(commonStore.memberWithdrawalDescription || ""));
    if (!html) return "";
    const withMarkerColor = applyListMarkerColor(html);
    if (/<img\b/i.test(html)) return formatRichText(withMarkerColor);
    const text = html.replace(/<[^>]*>/g, "").replace(/&nbsp;/gi, " ").trim();
    return text ? formatRichText(withMarkerColor) : "";
});
// 提现到账方式按后台「提现设置」勾选项展示（D1107），未配置则全显示
const methodList = computed<any>(() => {
    const all = [
        { title: "银行卡", accountType: 1 },
        { title: "支付宝", accountType: 2 },
        { title: "微信", accountType: 3 }
    ];
    const enabled = commonStore.memberWithdrawalReceiptMethod;
    // 配置字段缺失（后端未下发）才兜底全显示；空数组=后台全部取消勾选=一个都不展示
    if (!Array.isArray(enabled)) return all;
    return all.filter((m) => enabled.includes(m.accountType));
});
// 后台全部取消勾选（空数组）时无可用提现方式，用于空态提示与禁用提交（D1107）
const hasWithdrawalMethod = computed<boolean>(() => methodList.value.length > 0);
const splitStr = (str: string) => {
    return str.split("?")[0];
};
// 会员余额提现开关（D1107）：关闭时隐藏“提现”入口，不影响分销佣金提现
const menuList = computed<any>(() =>
    [
        { value: "账户明细", url: "/member/account/detail", size: 0 },
        { value: "申请记录", url: "/member/account/log", size: 0 },
        { value: "充值", url: "/member/account/deposit", size: 0 },
        { value: "提现", url: "/member/account/raplytocard", size: 0 },
        { value: "卡管理", url: "/member/account/cardManagement", size: 0 }
    ].filter((item) => item.url !== "/member/account/raplytocard" || commonStore.memberWithdrawalEnabled === 1)
);
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.account-raplytocard-content {
    background: #fff;
    border: 0;
    padding: 20px;
    box-sizing: border-box;
    font-size: 14px;
    display: flex;
    justify-content: center;
    align-items: center;
    color: #1a1a1a;

    .tips {
        font-size: 12px;
        color: #a5a5a5;
        margin-left: 100px;
        margin-bottom: 20px;
    }

    .empty-method {
        font-size: 13px;
        color: #a5a5a5;
    }

    .card {
        width: 120px;
        height: 36px;
        line-height: 36px;
        position: relative; /* 为了绝对定位内部的三角形 */
        cursor: pointer; /* 可选，鼠标悬停时显示为指针 */
        color: #1a1a1a;
        text-align: center;
        border-radius: 4px;
        overflow: hidden;
        border: 1px solid #ddd;
        display: flex;
        flex-direction: column;
        box-sizing: border-box;
    }

    .selected {
        border-color: var(--general); /* 选中状态下边框变红 */
    }

    .selected::after {
        content: "";
        position: absolute;
        bottom: 0; /* 三角形在右下角，但留有一点间距 */
        right: 0;
        width: 0;
        height: 0;
        border-left: 16px solid transparent;
        border-top: 16px solid transparent;
        border-bottom: 16px solid var(--general); /* 三角形大小和颜色 */
        text-align: center;
        transform: rotate(-0deg); /* 调整勾的位置和角度 */
    }

    .selected::before {
        content: "✔";
        color: var(--main-text); /* 勾的颜色 */
        font-size: 8px; /* 勾的大小 */
        position: absolute;
        bottom: 5px; /* 调整勾在三角形内的垂直位置 */
        right: 0; /* 调整勾在三角形内的水平位置 */
        transform: translateX(0%) translateY(50%); /* 微调勾的位置 */
        z-index: 100;
    }

    .form-body {
        width: 440px;
        font-size: 14px;
    }

    .withdraw-desc-item {
        align-items: flex-start;

        :deep(.el-form-item__label) {
            height: auto;
            line-height: 1.6;
        }

        :deep(.el-form-item__content) {
            display: block;
            line-height: 1.6;
            align-items: stretch;
        }
    }

    .withdraw-desc {
        display: block;
        width: 100%;
        font-size: 14px;
        color: #333;
        line-height: 1.6;
        word-break: break-word;
        white-space: normal;

        :deep(h1),
        :deep(h2),
        :deep(h3),
        :deep(h4),
        :deep(h5),
        :deep(h6) {
            margin: 0 0 12px;
            line-height: 1.3;
            font-weight: 700;
        }

        :deep(h1) {
            font-size: 28px;
        }

        :deep(h2) {
            font-size: 22px;
        }

        :deep(h3) {
            font-size: 18px;
        }

        :deep(h4),
        :deep(h5),
        :deep(h6) {
            font-size: 16px;
        }

        :deep(p),
        :deep(div) {
            margin: 0 0 8px;
            line-height: 1.6;
            white-space: normal;
        }

        :deep(p:last-child),
        :deep(div:last-child) {
            margin-bottom: 0;
        }

        :deep(ul),
        :deep(ol) {
            margin: 0;
            padding-left: 1.6em;
        }

        :deep(ul) {
            list-style: disc;
        }

        :deep(ol) {
            list-style: decimal;
        }

        :deep(li) {
            display: list-item;
            margin: 0 0 6px;
            line-height: 1.6;
        }

        :deep(li::marker) {
            color: inherit;
        }

        :deep(ul > li) {
            list-style: disc;
        }

        :deep(ol > li) {
            list-style: decimal;
        }

        :deep(strong),
        :deep(b) {
            font-weight: 700;
        }

        :deep(em),
        :deep(i) {
            font-style: italic;
        }

        :deep(u) {
            text-decoration: underline;
        }

        :deep(a) {
            color: var(--general);
        }

        :deep(img) {
            max-width: 100%;
            height: auto;
            object-fit: contain;
            display: block;
            margin: 8px 0;
        }

        :deep(table) {
            width: 100%;
            border-collapse: collapse;
        }
    }
    :deep(.el-form-item__label) {
        font-size: 14px;
        color: #1a1a1a;
    }
    :deep(.el-input__inner) {
        font-size: 14px;
        color: #1a1a1a;
    }
    :deep(.el-textarea__inner) {
        font-size: 14px;
        color: #1a1a1a;
    }
    :deep(.el-select__wrapper) {
        font-size: 14px;
        color: #1a1a1a;
    }
}
</style>
