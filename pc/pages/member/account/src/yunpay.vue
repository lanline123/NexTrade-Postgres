<template>
    <div>
        <CommonHeader title="YunPay"></CommonHeader>
        <CommonLoginHeader :showLogin="false"></CommonLoginHeader>
        <div class="wechat-com">
            <div class="wechat container">
                <div class="message">{{ $t("订单提交成功，请尽快付款") }}！{{ $t("订单号") }}：{{ order.orderSn }}</div>
                <div class="message">
                    {{ $t("应付金额") }}：
                    <FormatPrice v-model="order.amount" :fontStyle="{ color: 'var(--price)', fontSize: '18px' }"></FormatPrice>
                </div>
                <div class="payment">
                    <div class="title" v-if="type == 'wechat'">{{ $t("微信支付") }}</div>
                    <div class="title" v-if="type == 'alipay'">{{ $t("支付宝支付") }}</div>
                    <div class="image-div">
                        <div class="left-image">
                            <div>
                                {{ $t("如果二维码已过期") }}，{{ $t("请")
                                }}<span @click="refreshPage" class="font-color" style="cursor: pointer">{{ $t("刷新") }}</span
                                >{{ $t("页面重新获取二维码") }}。
                            </div>
                            <div class="ewm" v-if="count <= 50">
                                <el-image class="image" :src="qrCodeImage"></el-image>
                            </div>
                            <div class="ewm-empty" v-else>
                                {{ $t("已超时") }}，<a class="font-color" @click="refreshPage">{{ $t("点击刷新") }}</a>
                            </div>

                            <div class="pw-box-ft">
                                <div v-if="type == 'wechat'">{{ $t("请使用微信扫一扫") }}</div>
                                <div v-if="type == 'alipay'">{{ $t("请使用支付宝扫一扫") }}</div>
                                <div v-if="type == 'yunshanfu'">{{ $t("请使用云闪付扫一扫") }}</div>
                                <div>{{ $t("扫描二维码支付") }}</div>
                            </div>
                        </div>
                        <div class="right-image">
                            <div class="image" v-if="type == 'wechat'">
                                <el-image :src="weixin"></el-image>
                            </div>
                            <div class="image" v-if="type == 'alipay'">
                                <el-image :src="zfb"></el-image>
                            </div>
                            <div class="image" v-if="type == 'yunshanfu'">
                                <el-image :src="yunshanfu"></el-image>
                            </div>
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
 * @description 云闪付充值支付页面
 * - 生成云闪付支付二维码供用户扫码支付
 * - 支持微信、支付宝、云闪付等多种扫码方式
 * - 定时轮询检查支付状态，支付成功后自动跳转
 */
import { creatPay, orderPayInfo, checkPayStatus } from "~/api/order/pay";
import QRCode from "qrcode";
import weixin from "assets/images/wechat/weixin.png";
import zfb from "assets/images/wechat/zfb.png";
import yunshanfu from "assets/images/wechat/yunshanfu.png";
import { checkUserPayStatus, updateRechargeOrderCreate, updateRechargeOrderPay } from "~/api/user/account";
const route = useRoute();
const id = ref(route.query.id);
const type = ref(route.query.type);
const loading = ref(true);
const payResult = ref({});
const qrCodeImage = ref<string>("");
const order = ref({
    orderSn: "",
    amount: 0
});
const { t } = useI18n();
const loadOrderPayInfo = async () => {
    try {
        const result = await updateRechargeOrderPay({
            orderId: id.value
        });
        order.value = result.order;
    } catch (error: any) {
        message.error(error.message);
        navigateTo("/member/account/detail");
    } finally {
    }
};
loadOrderPayInfo();
const _creatPay = async () => {
    loading.value = true;
    try {
        const result = await updateRechargeOrderCreate({
            id: id.value,
            type: "yunpay_" + type.value
        });
        generateQRCodeImage(result?.payInfo.codeUrl);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const count = ref(0);
let intervalId: NodeJS.Timeout | null = null; // 存储定时器 ID，便于后续清除
onMounted(() => {
    _creatPay();
    //每隔三秒检查支付状态，超过20次则认为支付失败
    intervalId = setInterval(() => {
        _checkPayStatus();
        count.value++;
        if (count.value === 50) {
            clearInterval(intervalId!); // 清除定时器，停止执行
            message.error(t("支付已超时，您可以刷新页面重新支付"));
        }
    }, 2000);
});
const _checkPayStatus = () => {
    checkUserPayStatus({
        id: id.value
    }).then((res: any) => {
        if (res.payStatus == 1) {
            clearInterval(intervalId!);
            ElMessageBox.confirm(t("订单支付成功"), t("提示"), {
                confirmButtonText: t("查看订单"),
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
    const data = url; // 替换为你要转换的链接
    let opts = {
        margin: 1, //二维码留白边距
        width: 500, //宽
        height: 500 //高
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
                    align-items: center;
                    gap: 10px;

                    .image {
                        width: 270px;
                        height: 270px;
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
