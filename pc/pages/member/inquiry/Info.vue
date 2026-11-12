<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="询价单详情"></CommonHeader>
    <TopJumpBar tltle="询价单详情"></TopJumpBar>

    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="title-or-tabs">
                <span>{{ $t("询价单详情") }}</span>
            </div>
            <div v-loading="loading" class="inquiry-info">
                <div class="card status-card" :class="statusClass">
                    <SvgIcon :name="statusIconName" width="16" height="16" class="status-icon" />
                    <span>{{ statusText }}</span>
                </div>

                <div class="card goods-card">
                    <el-image :src="productImage(formState.product)" class="goods-img" :alt="formState.product?.productName || ''" />
                    <div class="goods-right">
                        <div class="goods-name">{{ formState.product?.productName || "-" }}</div>
                        <div v-if="formState.product?.marketPrice || formState.product?.productPrice" class="price-row">
                            <div v-if="formState.product?.marketPrice" class="market">
                                <span>{{ $t("市场价") }}</span>
                                <FormatPrice
                                    :model-value="formState.product.marketPrice"
                                    :showText="false"
                                    :currencyStyle="{ fontSize: '13px', color: '#8c8c8c' }"
                                    :fontStyle="{ fontSize: '13px', color: '#8c8c8c' }"
                                    :decimalsStyle="{ fontSize: '13px', color: '#8c8c8c' }"
                                />
                            </div>
                            <div v-if="formState.product?.productPrice" class="shop">
                                <span>{{ $t("本站价") }}</span>
                                <FormatPrice
                                    :model-value="formState.product.productPrice"
                                    :showText="false"
                                    :currencyStyle="{ fontSize: '13px', color: '#e1251b', fontWeight: 'bold' }"
                                    :fontStyle="{ fontSize: '13px', color: '#e1251b', fontWeight: 'bold' }"
                                    :decimalsStyle="{ fontSize: '13px', color: '#e1251b', fontWeight: 'bold' }"
                                />
                            </div>
                        </div>
                    </div>
                </div>

                <div class="card section-card">
                    <h3>{{ $t("我的询价") }}</h3>
                    <div class="row">
                        <div class="label">{{ $t("询价内容") }}</div>
                        <div class="value">{{ formState.content || "-" }}</div>
                    </div>
                    <div class="row">
                        <div class="label">{{ $t("询价时间") }}</div>
                        <div class="value">{{ formState.createTime || "-" }}</div>
                    </div>
                    <div class="row">
                        <div class="label">{{ $t("联系人") }}</div>
                        <div class="value">{{ formState.contactName || "-" }}</div>
                    </div>
                    <div class="row">
                        <div class="label">{{ $t("联系方式") }}</div>
                        <div class="value">{{ formState.mobile || "-" }}</div>
                    </div>
                </div>

                <div class="card reply-card">
                    <h3>{{ $t("回复内容") }}</h3>
                    <template v-if="formState.replyRecords && formState.replyRecords.length > 0">
                        <div v-for="(reply, index) in formState.replyRecords" :key="index" class="reply-item">
                            <div class="reply-title" v-if="reply.replyType == 1">{{ $t("商家回复") }}</div>
                            <div class="reply-title" v-else>{{ reply.replyName || "-" }}</div>
                            <div class="reply-time">{{ reply.createTime || "-" }}</div>
                            <div class="reply-text">{{ reply.content || "-" }}</div>
                        </div>
                    </template>
                    <template v-else>
                        <div class="empty">{{ $t("商家暂未回复") }}</div>
                    </template>
                </div>

                <div class="card action-card">
                    <TigButton @click="onDelete" style="width: 120px; height: 40px; font-size: 14px;">{{ $t("删除询价单") }}</TigButton>
                    <TigButton theme type="primary" @click="openReplyDialog" style="width: 120px; height: 40px; font-size: 14px;">{{ $t("回复商家") }}</TigButton>
                </div>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>

    <el-dialog v-model="dialogVisible" :title="$t('回复商家')" width="600">
        <el-form ref="formRef" :model="replyForm" label-suffix="：" label-width="auto">
            <el-form-item :label="$t('回复内容')" prop="remark" :rules="[{ required: true, message: $t('请输入回复内容') }]">
                <el-input v-model="replyForm.remark" type="textarea" :rows="5" :placeholder="$t('请输入回复内容')" />
            </el-form-item>
        </el-form>
        <template #footer>
            <TigButton @click="dialogVisible = false" style="margin-right: 10px">{{ $t("取消") }}</TigButton>
            <TigButton theme type="primary" @click="onSubmitReply">{{ $t("提交") }}</TigButton>
        </template>
    </el-dialog>
</template>

<script lang="ts" setup>
import { computed, ref } from "vue";
import { Modal } from "ant-design-vue";
import { getInquiryDetail, replyInquiry, deleteInquiry } from "~/api/user/inquiry";
import type { InquiryDetail, InquiryReplyFormState } from "~/types/user/inquiry.d";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";

definePageMeta({
    middleware: "auth"
});

const { t } = useI18n();
const route = useRoute();
const loading = ref<boolean>(true);
const formState = ref<InquiryDetail>({});

const dialogVisible = ref(false);
const formRef = shallowRef();
const replyForm = ref<InquiryReplyFormState>({
    remark: ""
});

const inquiryId = computed<number>(() => Number(route.query.id || 0));

const statusText = computed(() => {
    return formState.value.status === 1 ? t("商家已回复") : t("待商家回复");
});

const statusIconName = computed(() => {
    return formState.value.status === 1 ? "inquiry-2" : "inquiry-1";
});

const statusClass = computed(() => {
    return formState.value.status === 1 ? "is-replied" : "is-pending";
});

const fetchDetail = async () => {
    try {
        loading.value = true;
        const result = await getInquiryDetail(inquiryId.value);
        formState.value = result || {};
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

const openReplyDialog = async () => {
    dialogVisible.value = true;
    replyForm.value = { id: inquiryId.value, remark: "" };
    if (formRef.value) await formRef.value.resetFields();
};

const onDelete = async () => {
    Modal.confirm({
        title: t("确认删除该询价单吗"),
        cancelText: t("取消"),
        okText: t("确定"),
        maskClosable: true,
        centered: true,
        onOk: async () => {
            try {
                await deleteInquiry(inquiryId.value);
                message.success(t("删除成功"));
                navigateTo("/member/inquiry/list");
            } catch (error: any) {
                message.error(error.message);
            }
        }
    });
};

const onSubmitReply = async () => {
    await formRef.value.validate();
    try {
        await replyInquiry({
            id: inquiryId.value,
            remark: replyForm.value.remark
        });
        message.success(t("提交成功"));
        dialogVisible.value = false;
        await fetchDetail();
    } catch (error: any) {
        message.error(error.message);
    }
};

onMounted(() => {
    if (!inquiryId.value) {
        navigateTo("/member/inquiry/list");
        return;
    }
    fetchDetail();
});
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.info-row {
    background: transparent !important;
    padding: 0 !important;
}

.title-or-tabs {
    background: transparent !important;
    border-bottom: none !important;
    padding: 0 0 16px 0 !important;
    height: auto !important;
    line-height: 1 !important;
    margin-bottom: 0;

    span {
        font-size: 16px;
        font-weight: bold;
        color: #1a1a1a;
        &::before {
            display: none;
        }
    }
}

.inquiry-info {
    .card {
        background: #fff;
        border: 0;
        padding: 24px;
        margin-bottom: 16px;
        border-radius: 8px;
    }

    .status-card {
        display: flex;
        align-items: center;
        gap: 8px;
        font-weight: bold;
        font-size: 14px;

        &.is-pending {
            color: #5F5F5F;
        }

        &.is-replied {
            color: #2abf5c;
        }

        .status-icon {
            flex-shrink: 0;
        }
    }

    .goods-card {
        display: flex;
        gap: 16px;
        align-items: center;

        .goods-img {
            width: 80px;
            height: 80px;
            border: 1px solid #f0f1f2;
            border-radius: 4px;
        }

        .goods-right {
            .goods-name {
                font-size: 14px;
                font-weight: bold;
                line-height: 1.4;
                margin-bottom: 12px;
                color: #1a1a1a;
            }

            .price-row {
                display: flex;
                flex-direction: column;
                gap: 8px;
                font-size: 13px;

                .market {
                    color: #8c8c8c;
                    display: flex;
                    align-items: center;
                    gap: 4px;
                }

                .shop {
                    color: #e1251b;
                    font-weight: bold;
                    display: flex;
                    align-items: center;
                    gap: 4px;
                }
            }
        }
    }

    .section-card,
    .reply-card {
        h3 {
            font-size: 16px;
            font-weight: bold;
            line-height: 1.2;
            margin-bottom: 24px;
            color: #1a1a1a;
        }
    }

    .section-card {
        .row {
            display: flex;
            flex-direction: column;
            gap: 8px;
            padding-bottom: 16px;
            margin-bottom: 16px;
            border-bottom: 1px solid #f0f1f2;

            &:last-child {
                border-bottom: none;
                padding-bottom: 0;
                margin-bottom: 0;
            }

            .label {
                color: #1a1a1a;
                font-weight: bold;
                font-size: 14px;
            }
            
            .value {
                color: #333;
                font-size: 14px;
                line-height: 1.5;
            }
        }
    }

    .reply-card {
        .reply-item {
            padding-bottom: 16px;
            margin-bottom: 16px;
            border-bottom: 1px solid #f0f1f2;

            &:last-child {
                border-bottom: none;
                padding-bottom: 0;
                margin-bottom: 0;
            }
        }

        .reply-title {
            font-size: 14px;
            font-weight: bold;
            margin-bottom: 8px;
            color: #1a1a1a;
        }

        .reply-time {
            color: #8c8c8c;
            font-size: 12px;
            margin-bottom: 12px;
        }

        .reply-text {
            line-height: 1.6;
            color: #333;
            font-size: 14px;
        }

        .empty {
            color: #8c8c8c;
            font-size: 14px;
        }
    }

    .action-card {
        display: flex;
        justify-content: center;
        gap: 16px;
        padding: 24px;
    }
}
</style>
