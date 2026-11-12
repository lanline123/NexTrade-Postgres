<template>
    <div v-if="isIndex" @click="handelToService">
        <slot v-if="showBtn"></slot>
    </div>
    <div v-else class="ele-menu">
        <div class="card" @click="handelToService" v-if="showBtn">
            <div class="feedback-menu">
                <div class="wrap-btn">
                    <i class="iconfont-pc icon-kefu"></i>
                    {{ $t("客服") }}
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { getHomeCustomerServiceConfig } from "~/api/home/home";
import { toLogin, X_CLIENT_TYPE } from "@/utils/request";
import { useUserStore } from "~/store/user";
import type { Action } from "element-plus";

const { t } = useI18n();

const userStore = useUserStore();

const props = defineProps({
    shopId: {
        type: [Number, String],
        default: 0
    },
    productId: {
        type: [Number, String],
        default: 0
    },
    orderId: {
        type: [Number, String],
        default: 0
    },
    isIndex: {
        type: Boolean,
        default: false
    },
    phone: {
        type: String,
        default: ""
    },
    showService: {
        type: [Boolean, null],
        default: null
    }
});
const customerServiceConfig = ref({
    h5Domain: "",
    url: "",
    openType: 0,
    show: "",
    serviceType: 0
});
const showBtn = computed(() => {
    if (props.showService !== null) {
        return props.showService;
    }else{
        return customerServiceConfig.value.show;
    }
});
const __getServiceConfig = async () => {
    try {
        const result = await getHomeCustomerServiceConfig();
        customerServiceConfig.value = result;
    } catch (error) {
        console.error(error);
    }
};
__getServiceConfig();

const domain = computed(() => {
    if (customerServiceConfig.value.h5Domain) {
        return customerServiceConfig.value.h5Domain;
    }
    return window.location.origin + "/mobile";
});
const handelToService = async () => {
    const serviceType = customerServiceConfig.value.serviceType;
    if (serviceType === 0) return;
    const openType = customerServiceConfig.value.openType;
    const width = 500;
    const height = 700;
    let url = customerServiceConfig.value.url;
    const top = screen.height / 2 - height / 2;
    if (serviceType !== 4) {
        if (!isPro() && Number(props.shopId) > 0) {
            if (!props.phone) {
                return message.warning(t("暂无客服"));
            }
            ElMessageBox.alert(props.phone, t("客服电话"), {
                confirmButtonText: t("复制"),
                callback: (action: Action) => {
                    if (action === "confirm") {
                        copyText(props.phone, () => {
                            message.success(t("复制成功"));
                        });
                    }
                }
            });
            return;
        }

        if (openType == 1) {
            window.open(url, "_blank", `width=${width},height=${height},top=${top}`);
        } else {
            window.open(url, "_blank");
        }
    } else {
        if (!userStore.isAuthenticated) {
            return toLogin();
        }
        url = `/chat?shopId=${props.shopId}`;
        Number(props.productId) > 0 && (url += `&productId=${props.productId}`);
        Number(props.orderId) > 0 && (url += `&orderId=${props.orderId}`);
        window.open(url, "_blank", `width=${1200},height=${800},top=${top}`);
    }
};
</script>

<style lang="less" scoped>
.ele-menu {
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    position: fixed;
    right: 2vw;
    bottom: 328px;
    z-index: 11;
    gap: 8px;

    .to-top {
        height: 68px;
        width: 68px;
    }

    .card {
        font-size: 12px;
        position: relative;
        padding: 0;
        border-radius: 6px;
        overflow: hidden;
        box-shadow: 0 4px 10px 0 rgba(0, 0, 0, 0.08);

        .feedback-menu {
            height: 68px;
            width: 68px;
            box-shadow: 0 4px 10px 0 rgba(0, 0, 0, 0.08);

            .wrap-btn {
                height: 68px;
                width: 68px;
                padding: 0 4px;
                box-sizing: border-box;
                background-color: #fff;
                color: #666666;
                display: flex;
                flex-direction: column;
                justify-content: center;
                align-items: center;
                cursor: pointer;
                border-bottom: 1px solid #e8e8e8;
                transition: background-color 0.3s;
                /* 文案换行时逐行居中，长单词允许断行避免溢出卡片 */
                text-align: center;
                line-height: 1.3;
                overflow-wrap: break-word;

                & > i {
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    height: 28px;
                    aspect-ratio: auto 28 / 28;
                    width: 28px;
                }
            }

            .wrap-btn:hover {
                color: var(--general);
                transition: 0.3s;
            }

            .check-item {
                color: var(--general);
                transition: 0.3s;
            }
        }
    }
}
</style>
