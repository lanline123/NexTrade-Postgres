<template>
    <div>
        <CommonHeader :title="isAlipay ? $t('支付宝支付') : $t('微信支付')"></CommonHeader>
        <CommonLoginHeader :showLogin="false"></CommonLoginHeader>
        <div class="wechat-com">
            <div class="wechat container">
                <div class="message">{{ $t("订单提交成功，请尽快付款") }}</div>
                <div class="message">
                    {{ $t("应付金额") }}：
                    <FormatPrice v-model="order.amount" :fontStyle="{ color: 'var(--price)', fontSize: '18px' }"></FormatPrice>
                </div>
                <div class="payment">
                    <div class="title">{{ isAlipay ? $t("支付宝支付") : $t("微信支付") }}</div>
                    <div class="image-div">
                        <div class="left-image">
                            <div>
                                {{ $t("如果二维码已过期") }}，<span @click="refreshPage" class="font-color" style="cursor: pointer">{{ $t("刷新") }}</span
                                >{{ $t("重新获取二维码") }}。
                            </div>
                            <div class="ewm" v-if="count <= 50">
                                <el-image class="image" :src="qrCodeImage"></el-image>
                            </div>
                            <div class="ewm-empty" v-else>
                                {{ $t("已超时") }}，<a class="font-color" @click="refreshPage">{{ $t("点击刷新") }}</a>
                            </div>

                            <div class="pw-box-ft">
                                <div>{{ isAlipay ? $t("请使用支付宝扫一扫") : $t("请使用微信扫一扫") }}</div>
                                <div>{{ $t("扫描二维码支付") }}</div>
                            </div>
                        </div>
                        <div class="right-image">
                            <div class="image"></div>
                        </div>
                    </div>
                    <div @click="closeWindow" class="button-title">&lt; {{ $t("选择其他支付方式") }}</div>
                </div>
            </div>
        </div>
        <CommonPageFooter :bg="'white'" :mt30="false" :type="1"></CommonPageFooter>
    </div>
</template>
<script lang="ts" setup>
/**
 * 拉卡拉充值支付页（PC）
 * - payCode 由 query.payCode 传入：lakalaWechat / lakalaAlipay
 * - 支付宝（lakalaAlipay）：PC 主扫，出二维码 codeUrl
 * - 微信（lakalaWechat）：PC/H5 无直连主扫，后端返回聚合收银台跳转 URL（payInfo.url）则跳转；
 *   若后端返回 codeUrl 则兼容出二维码（拉起方式以联调为准）
 * - 轮询 checkUserPayStatus 确认到账
 */
import QRCode from "qrcode";
import { checkUserPayStatus, updateRechargeOrderCreate, updateRechargeOrderPay } from "~/api/user/account";
const route = useRoute();
const id = computed(() => {
    return route.query.id;
});
// 实际发起支付的 payCode：lakalaWechat / lakalaAlipay
const payCode = ref((route.query.payCode as string) || "lakalaWechat");
const isAlipay = computed(() => payCode.value === "lakalaAlipay");
const loading = ref(true);
const qrCodeImage = ref<string>("");
const order = ref({
    amount: 0
});
const loadOrderPayInfo = async () => {
    try {
        const result = await updateRechargeOrderPay({ orderId: id.value });
        order.value = result.order;
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
loadOrderPayInfo();
const _creatPay = async () => {
    loading.value = true;
    try {
        const result = await updateRechargeOrderCreate({
            id: id.value,
            type: payCode.value
        });
        const info: any = result?.payInfo ?? {};
        // 拉卡拉微信 PC/H5 可能返回聚合收银台跳转 URL（无直连主扫），有 url 则跳转
        if (info.url) {
            window.location.href = info.url;
            return;
        }
        await generateQRCodeImage(info.codeUrl);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const { t } = useI18n();
const count = ref(0);
let intervalId: NodeJS.Timeout | null = null;
onMounted(() => {
    _creatPay();
    intervalId = setInterval(() => {
        _checkPayStatus();
        count.value++;
        if (count.value === 50) {
            clearInterval(intervalId!);
            message.error(t("支付已超时，您可以刷新页面重新支付"));
        }
    }, 2000);
});
const _checkPayStatus = () => {
    checkUserPayStatus({
        id: id.value
    }).then((res: any) => {
        if (res == 1) {
            clearInterval(intervalId!);
            ElMessageBox.confirm(t("充值成功"), t("提示"), {
                confirmButtonText: t("查看充值详情"),
                showClose: false,
                showCancelButton: false,
                closeOnClickModal: false,
                type: "success"
            })
                .then(() => {
                    navigateTo("/member/account/detail");
                })
                .catch(() => {});
        }
    });
};
const generateQRCodeImage = async (url: string) => {
    const data = url;
    let opts = {
        margin: 1,
        width: 500,
        height: 500
    };
    qrCodeImage.value = await QRCode.toDataURL(data, opts);
};
const closeWindow = () => {
    window.close();
};
const refreshPage = () => {
    window.location.reload();
};
onBeforeUnmount(() => {
    clearInterval(intervalId!);
});
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.wechat-com {
    display: flex;
    justify-content: center;

    background-color: white;

    .wechat {
        display: flex;
        flex-direction: column;
        gap: 20px;
        padding: 30px 0;

        .message {
            font-size: 14px;
        }

        .right {
            font-size: 12px;
            text-align: right;
        }

        .payment {
            background-color: #fff;
            box-shadow: 0 0 20px rgba(0, 0, 0, 0.12);
            padding: 12px 30px;
            transition: all 0.2s ease-in-out 0s;
            display: flex;
            flex-direction: column;

            .title {
                font-size: 18px;
            }

            .image-div {
                display: flex;
                gap: 30px;

                .left-image {
                    flex: 1;
                    color: #333;
                    display: flex;
                    flex-direction: column;
                    justify-content: center;
                    align-items: flex-end;
                    gap: 20px;

                    .ewm {
                        background: url("/assets/images/wechat/ui-modal-loading.gif") no-repeat scroll center center;
                        width: 300px;
                        height: 300px;
                    }
                    .ewm-empty {
                        width: 300px;
                        height: 300px;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                    }

                    .image {
                        width: 300px;
                        height: 300px;
                        border: 1px solid #ddd;
                    }

                    .pw-box-ft {
                        background: #ff7674 url("/assets/images/wechat/icon-red.png") no-repeat scroll 50px 8px;
                        height: 60px;
                        width: 300px;
                        color: white;
                        font-size: 14px;
                        font-weight: 700;
                        box-sizing: border-box;
                        line-height: 22px;
                        padding: 8px 0 8px 125px;
                    }
                }

                .right-image {
                    flex: 1;
                    display: flex;
                    justify-content: flex-start;

                    .image {
                        background-image: url("/assets/images/wechat/phone-bg.png");
                        width: 330px;
                        height: 420px;
                    }
                }
            }
        }
        .button-title {
            color: var(--general);
            cursor: pointer;
            font-size: 14px;
            height: 60px;
            font-weight: 700;
            margin-left: 100px;
            margin-top: 30px;
        }
    }
}
</style>
