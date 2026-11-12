<template>
    <div>
        <CommonHeader title="汇付天下"></CommonHeader>
        <CommonLoginHeader :showLogin="false"></CommonLoginHeader>
        <div class="wechat-com">
            <div class="wechat container">
                <div class="message">{{ $t("订单提交成功，请尽快付款") }}！{{ $t("订单号") }}：{{ order.orderSn }}</div>
                <div class="message">
                    {{ $t("应付金额") }}：
                    <FormatPrice v-model="order.unpaidAmount" :fontStyle="{ color: 'var(--price)', fontSize: '18px' }"></FormatPrice>
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
 * 汇付支付页（PC 扫码）
 * - payCode 由 query.payCode 传入：huifuWechat / huifuAlipay，两者 PC 均为扫码 codeUrl
 * - 轮询 checkPayStatus（每2秒，最多50次）
 * - query.type=saleMode 时走预售定金支付
 */
import { creatPay, orderPayInfo, checkPayStatus, createDepositPayment } from "~/api/order/pay";
import QRCode from "qrcode";

const route = useRoute();
const id = ref(route.query.id);
const type = ref(route.query.type ?? "");
// 实际发起支付的 payCode：huifuWechat / huifuAlipay
const payCode = ref((route.query.payCode as string) || "huifuWechat");
const isAlipay = computed(() => payCode.value === "huifuAlipay");
const loading = ref(true);
const qrCodeImage = ref<string>("");
const order = ref<any>({});
const { t } = useI18n();

const loadOrderPayInfo = async () => {
    try {
        const result = await orderPayInfo({ id: id.value });
        order.value = result.order;
        if (result.grouponPromotionRecordId) {
            navigateTo(`/member/group/info?id=${result.grouponPromotionRecordId}`, { replace: true });
        }
    } catch (error: any) {
        message.error(error.message);
        navigateTo("/member/order/list/");
    }
};
loadOrderPayInfo();

const _creatPay = async () => {
    loading.value = true;
    try {
        const result =
            type.value === "saleMode"
                ? await createDepositPayment({ id: id.value, type: payCode.value })
                : await creatPay({ id: id.value, type: payCode.value });
        const info: any = result?.payInfo ?? {};
        if (!info.codeUrl) {
            message.error(t("获取支付二维码失败，请刷新重试"));
            return;
        }
        await generateQRCodeImage(info.codeUrl);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

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
    checkPayStatus({ id: id.value }).then((res: any) => {
        if (res == 1) {
            clearInterval(intervalId!);
            ElMessageBox.confirm(t("订单支付成功"), t("提示"), {
                confirmButtonText: t("查看订单"),
                showClose: false,
                showCancelButton: false,
                closeOnClickModal: false,
                type: "success"
            })
                .then(() => {
                    navigateTo("/member/order/list");
                })
                .catch(() => {});
        }
    });
};

const generateQRCodeImage = async (url: string) => {
    qrCodeImage.value = await QRCode.toDataURL(url, { margin: 1, width: 500, height: 500 });
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
