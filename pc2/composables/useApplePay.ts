import { creatPay, createDepositPayment, checkPayStatus, applePayValidateMerchant, applePayConfirm } from "~/api/order/pay";

/**
 * Apple Pay 内联支付封装（一期原生单流程）
 * - 不建支付中间页，直接在收银台点击手势内拉起 ApplePaySession
 * - 流程：begin() → onvalidatemerchant(validateMerchant) → onpaymentauthorized(confirm) → checkStatus 兜底
 * - 收单 provider 前端无感知（后端不下发），一期不引任何 SDK
 */

/** Apple Pay 拉起入参 */
export interface ApplePayLaunchOptions {
    /** 订单 id */
    id: string | number;
    /** 是否预售定金支付（走 createDepositPayment） */
    isDeposit?: boolean;
    /** 支付金额（订单待支付金额，作为面板 total.amount） */
    amount: string | number;
    /** 面板展示名（商户名/订单号），缺省 "Apple Pay" */
    label?: string;
    /** 国家/地区码，缺省 CN（后端下发或订单携带时优先） */
    countryCode?: string;
    /** 货币码，缺省 CNY */
    currencyCode?: string;
    /** 支持的卡组织，缺省常用三项 */
    supportedNetworks?: string[];
    /** 商户能力，缺省 supports3DS */
    merchantCapabilities?: string[];
    /** 支付成功回调（由页面处理提示与跳转） */
    onSuccess?: () => void;
    /** 支付失败/异常回调 */
    onFail?: (message?: string) => void;
}

/**
 * 能力检测：仅 Safari + 支持 Apple Pay 的苹果设备返回 true
 * 非苹果环境 / 微信内 / 安卓 / Chrome 均返回 false
 */
export const canUseApplePay = (): boolean => {
    try {
        const AP = typeof window !== "undefined" ? (window as any).ApplePaySession : null;
        return !!AP && typeof AP.canMakePayments === "function" && AP.canMakePayments();
    } catch {
        return false;
    }
};

export function useApplePay() {
    const { t } = useI18n();
    const paying = ref(false);

    /** 确认成功后兜底轮询后端支付状态（最多 5 次，每 2s） */
    const pollPayStatus = (id: string | number) =>
        new Promise<boolean>((resolve) => {
            let count = 0;
            const timer = setInterval(async () => {
                count++;
                try {
                    const res: any = await checkPayStatus({ id });
                    if (res == 1) {
                        clearInterval(timer);
                        resolve(true);
                        return;
                    }
                } catch {
                    // 忽略单次轮询异常，继续下一次
                }
                if (count >= 5) {
                    clearInterval(timer);
                    resolve(false);
                }
            }, 2000);
        });

    /**
     * 拉起 Apple Pay 支付
     * 注意：ApplePaySession 必须在用户点击手势内同步创建并 begin()，不能先 await 再创建
     */
    const launchApplePay = (options: ApplePayLaunchOptions) => {
        if (paying.value) return;
        if (!canUseApplePay()) {
            options.onFail?.(t("当前环境不支持 Apple Pay"));
            return;
        }

        const AP = (window as any).ApplePaySession;
        const request = {
            countryCode: options.countryCode || "CN",
            currencyCode: options.currencyCode || "CNY",
            supportedNetworks: options.supportedNetworks?.length ? options.supportedNetworks : ["visa", "masterCard", "chinaUnionPay"],
            merchantCapabilities: options.merchantCapabilities?.length ? options.merchantCapabilities : ["supports3DS"],
            total: {
                label: options.label || "Apple Pay",
                amount: String(options.amount ?? "0")
            }
        };

        let session: any;
        try {
            session = new AP(3, request);
        } catch {
            options.onFail?.(t("Apple Pay 拉起失败"));
            return;
        }
        paying.value = true;

        // 商户验证：先建支付单，再向后端换取 merchantSession
        session.onvalidatemerchant = async (event: any) => {
            try {
                if (options.isDeposit) {
                    await createDepositPayment({ id: options.id, type: "applepay" });
                } else {
                    await creatPay({ id: options.id, type: "applepay" });
                }
                const res: any = await applePayValidateMerchant({ id: options.id, validationURL: event.validationURL });
                // 后端可能直接返回 merchantSession，也可能包一层
                session.completeMerchantValidation(res?.merchantSession ?? res);
            } catch (err: any) {
                session.abort();
                paying.value = false;
                options.onFail?.(err?.message || t("Apple Pay 商户验证失败"));
            }
        };

        // 支付授权：把 Apple token 交后端确认
        session.onpaymentauthorized = async (event: any) => {
            try {
                await applePayConfirm({ id: options.id, token: JSON.stringify(event.payment?.token ?? event.payment) });
                session.completePayment(AP.STATUS_SUCCESS);
                const paid = await pollPayStatus(options.id);
                paying.value = false;
                if (paid) {
                    options.onSuccess?.();
                } else {
                    // 已授权且确认成功，但后端未在兜底轮询内回执到账，交由页面提示"处理中"而非直接成功
                    options.onFail?.(t("支付处理中，请稍后在订单中查看支付结果"));
                }
            } catch (err: any) {
                session.completePayment(AP.STATUS_FAILURE);
                paying.value = false;
                options.onFail?.(err?.message || t("Apple Pay 支付失败"));
            }
        };

        // 用户取消：静默，不报错，允许重试
        session.oncancel = () => {
            paying.value = false;
        };

        try {
            session.begin();
        } catch {
            paying.value = false;
            options.onFail?.(t("Apple Pay 拉起失败"));
        }
    };

    return { paying, launchApplePay };
}
