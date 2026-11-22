import { useEffect, useRef, useState } from "react";
import { RichText, Text, View } from "@tarojs/components";
import Taro, { useDidShow, useRouter, useUnload } from "@tarojs/taro";

import CancelOrderReasonPopup from "@/components/order/CancelOrderReasonPopup";
import type { CancelOrderReasonPopupRef } from "@/components/order/CancelOrderReasonPopup";
import DeliveryLinks from "@/components/deliveryLinks";
import DeliveryFiles from "@/components/deliveryFiles";
import FormatPrice from "@/components/format-price/FormatPrice";
import Service from "@/components/service";
import TigButton2 from "@/components/tig/tig-button2/TigButton2";
import TigCountdown from "@/components/tig/tig-countdown/TigCountdown";
import TigImage from "@/components/tig/tig-image/TigImage";
import TigLayout from "@/components/tig/tig-layout/TigLayout";
import TigNavbar from "@/components/tig/tig-navbar/TigNavbar";
import PresaleCard from "./src/PresaleCard";
import { getOrder, confirmReceipt, getShippingInfo, cancelPaidRefund } from "@/api/user/order";
import { regenerateGiftShareToken, cancelGiftOrder } from "@/api/order/gift";
import { getKitchenInfo, urgeOrder, type KitchenInfoVO } from "@/api/order/kitchen";
import { buyerDeliveryProgress } from "@/api/rider/rider";
import type { OrderInfoItemElement, OrderInfoResponseItem, ShippingInfoTrace } from "@/types/user/order";
import { useUserStore } from "@/store/user";
import { useConfigStore } from "@/store/config";
import { useScrollTop } from "@/hooks";
import { copy, getSkuText, hasSkuText, toStoreOrShopHome, isOverseas } from "@/utils";
import { canShowOrderPayment } from "@/utils/orderPayment";
import { pickSkuPic } from "@/utils/format";
import { canGiftCardItemApplyAftersales, isGiftCardAftersalesBlocked } from "@/utils/giftCard";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";

import "./info.scss";

/**
 * 订单详情页（自旧 pages/user/order/info.vue 移植）
 * - 展示订单状态流程、收货信息、物流信息
 * - 展示商品列表，支持单品售后和整单售后
 * - 虚拟商品订单展示券码信息（支持查看/隐藏）
 * - 根据订单状态显示不同操作按钮（取消/付款/确认收货/售后）
 */
export default function OrderInfo() {
    usePageGuards();

    const router = useRouter();
    const configIntegralName = useConfigStore((s) => s.integralName);
    const serviceConfig = useUserStore((s) => s.serviceConfig);
    const { scrollTop } = useScrollTop();

    /** 海外皮肤：用于增值税等专属展示 */
    const isOverseasSkin = isOverseas();

    const [orderInfo, setOrderInfo] = useState<OrderInfoResponseItem>({} as OrderInfoResponseItem);
    const [itemsGroup, setItemsGroup] = useState<any[][]>([]);
    const [shippingInfo, setShippingInfo] = useState<ShippingInfoTrace[]>([]);
    // 同城配送/上门服务派送进度（自营骑手 MVP，我方新增；无派送单时为 null 不渲染）
    const [deliveryProgress, setDeliveryProgress] = useState<any>(null);
    const [emptyText] = useState<string>("");
    const [showMore, setShowMore] = useState(false);
    // 后厨面板（我方新增）：已付款待发货时拉取后厨状态与催菜频控现状
    const [kitchenInfo, setKitchenInfo] = useState<KitchenInfoVO | null>(null);
    // 金额明细默认展开
    const [showUp, setShowUp] = useState(true);
    const [cardViewStates, setCardViewStates] = useState<Record<string | number, boolean>>({});

    const idRef = useRef<null | number>(null);
    const isFirstLoadRef = useRef(true);
    const cancelOrderReasonPopupRef = useRef<CancelOrderReasonPopupRef>(null);

    /** 金额行统一字号/字色（对齐跨境扁平卡）；优惠类金额用红色 */
    const amountStyle = { fontSize: "26rpx", color: "#1a1a1a", fontWeight: "bold" };
    const discountStyle = { fontSize: "26rpx", color: "#cc0c1c", fontWeight: "bold" };
    const totalStyle = { fontSize: "30rpx", color: "#1a1a1a", fontWeight: "bold" };

    const oStatusInfo: Record<number, string> = {
        0: t("您的订单已提交成功，请尽快完成付款哦！"),
        1: t("感谢您在本店的购物，您的订单已提交成功，我们会尽快为您发货！"),
        2: t("您的订单已经发货，请注意查收，祝您购物愉快！"),
        3: t('您的订单已被取消，您可以通过"再次购买"，重新购买订单中的商品。'),
        4: t('您的订单被认定为无效订单，订单已被关闭。您可以通过"再次购买"，重新购买订单中的商品。给您造成的不便请多多谅解！'),
        5: t("您的订单已经确认收货，感谢您在本店购物，祝您生活愉快！")
    };

    const isOfflineOrder = Number(orderInfo?.payTypeId) === 3;
    const latestOfflineClaim = (value: any) => {
        if (!Array.isArray(value)) return value || null;
        return value.slice().sort((a: any, b: any) => Number(b?.claimId || 0) - Number(a?.claimId || 0))[0] || null;
    };
    const resolveClaimStatus = (data: any) => {
        const status = data?.offlinePaymentClaimStatus;
        if (status !== null && status !== undefined) return Number(status);
        const claim: any = data?.offlinePaymentClaim;
        const claimStatus = latestOfflineClaim(claim)?.status;
        return claimStatus === null || claimStatus === undefined ? null : Number(claimStatus);
    };
    const offlineClaimStatus = resolveClaimStatus(orderInfo);
    const offlineClaimRejectReason = (() => {
        const reason = orderInfo?.offlinePaymentClaimRejectReason;
        if (reason) return reason;
        const claim: any = (orderInfo as any)?.offlinePaymentClaim;
        return latestOfflineClaim(claim)?.rejectReason || "";
    })();
    const offlineClaimAuditRemark = String(latestOfflineClaim((orderInfo as any)?.offlinePaymentClaim)?.auditRemark || "").trim();
    /** 认款被驳回且订单未取消时才允许修改申请 */
    const isOfflineClaimRejected = isOfflineOrder && Number(offlineClaimStatus) === 2 && Number(orderInfo?.orderStatus) !== 3;
    /** 已取消订单不再提供认款详情和修改申请入口 */
    const canViewOfflineClaim =
        Number(orderInfo?.orderStatus) !== 3 && !!orderInfo?.availableActions?.offlinePaymentClaimDetail && !isOfflineClaimRejected;
    const canEditOfflineClaim =
        Number(orderInfo?.orderStatus) !== 3 && (isOfflineClaimRejected || !!orderInfo?.availableActions?.editOfflinePaymentClaim);
    const canShowToPay = canShowOrderPayment({ ...orderInfo, offlinePaymentClaimStatus: offlineClaimStatus });
    const getClaimStatusName = (status: number) => ({ 0: t("待审核"), 1: t("审核通过"), 2: t("审核驳回") }[status] || "");
    const handleOfflineClaim = (orderId: number, mode: "edit" | "detail") =>
        Taro.navigateTo({ url: `/pages/order/offlinePayment?orderId=${orderId}&mode=${mode}` });

    const hasECardInfo = (() => {
        // 按商品是否返回券码判断，积分兑换等非卡密订单类型购买的卡密商品也需展示
        if (!Array.isArray(orderInfo?.items)) return false;
        return orderInfo.items.some((item) => Array.isArray(item.eCard) && item.eCard.length > 0);
    })();

    // 预售信息（存在时用于替换顶部步骤条、以及尾款支付入口）
    const productPresale = (orderInfo as any)?.presaleInfo || null;
    /** 是否展示预售流程卡（有则替换状态文案区） */
    const showPresaleCard = (() => {
        const mode = Number(productPresale?.saleMode ?? 0);
        return mode > 0 && !!productPresale;
    })();
    /** 定金预售待付尾款阶段（saleMode=2 且 presaleStatus=1） */
    const isFinalPaymentPhase = Number(productPresale?.saleMode) === 2 && Number(productPresale?.presaleStatus) === 1;
    /** 尾款未到支付时间：尾款阶段但后端未放行支付 */
    const isFinalPaymentBlocked = isFinalPaymentPhase && productPresale?.canPayFinalPayment === false;
    /** 底栏支付按钮是否走「尾款 ¥x 去支付」的宽按钮形态 */
    const showPresaleFinalPayBtn = isFinalPaymentPhase && !!productPresale && !isFinalPaymentBlocked;

    const payEndTime = String((orderInfo as any)?.payEndTime || "");
    /** 是否待支付：以后端权限及当前阶段的认款状态为准。 */
    const isPendingPay = !!orderInfo?.availableActions?.toPay && canShowToPay;
    const payEndTimeMs = (() => {
        if (!payEndTime) return 0;
        const ms = Date.parse(payEndTime.replace(/-/g, "/"));
        return Number.isFinite(ms) ? ms : 0;
    })();
    const showPayCountdown = isPendingPay && payEndTimeMs > Date.now();

    const countdownTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
    const stopCountdownWatch = () => {
        if (countdownTimerRef.current) {
            clearInterval(countdownTimerRef.current);
            countdownTimerRef.current = null;
        }
    };
    const startCountdownWatch = (data: OrderInfoResponseItem) => {
        stopCountdownWatch();
        const raw = String((data as any)?.payEndTime || "");
        const endMs = raw ? Date.parse(raw.replace(/-/g, "/")) : 0;
        const pending = !!data?.availableActions?.toPay && canShowOrderPayment({ ...data, offlinePaymentClaimStatus: resolveClaimStatus(data) });
        if (!(pending && endMs > Date.now())) return;
        countdownTimerRef.current = setInterval(() => {
            if (endMs && endMs - Date.now() <= 0) {
                stopCountdownWatch();
                void __getOrder();
            }
        }, 1000);
    };
    useUnload(stopCountdownWatch);

    const isMeituanDelivery = (() => {
        let delivery: any = null;
        return !!delivery && Number(delivery.carrierType) === 2;
    })();
    const NAVBAR_SCROLL_THRESHOLD = 50;
    const navbarBackground = (() => {
        if (!isMeituanDelivery) return "#fff";
        const opacity = Math.min(Math.max(scrollTop / NAVBAR_SCROLL_THRESHOLD, 0), 1);
        return `rgba(255, 255, 255, ${opacity})`;
    })();
    const isGiftCardExchangeOrder = orderInfo?.orderType === 12;
    /** 礼品卡订单：实体卡按实体商品放开售后，仅含电子卡时禁止售后 */
    const giftCardAftersalesBlocked = isGiftCardAftersalesBlocked(orderInfo);
    const canItemApplyAftersales = (item: OrderInfoItemElement) => canGiftCardItemApplyAftersales(orderInfo, item);
    // 自提订单确认收货（orderStatus>=3 即已完成/已取消/已关闭）后禁止售后
    /** 抽奖商品订单，不支持售后 */
    const isLotteryOrder = orderInfo?.orderType === 14;
    const hasItemAftersales = orderInfo?.items?.some((item) => Boolean(item.aftersalesItem)) ?? false;
    const pickupAftersalesDisabled = !!orderInfo?.isPickup && orderInfo?.orderStatus >= 3;
    // 收货信息齐全时才在状态区展示地址块（虚拟/卡密类订单无地址）
    const showAddressBox = !!(orderInfo?.userAddress && orderInfo?.consignee && orderInfo?.mobile);

    /** 顶部状态标题：线下支付已提交凭证待审核时展示「待审核」 */
    const statusTitle = isOfflineOrder && offlineClaimStatus === 0 ? t("待审核") : orderInfo?.orderStatusName || "";

    /** 顶部状态副文案（对齐跨境：tip / 自动收货 / oStatusInfo） */
    const statusSubText = (() => {
        if (orderInfo?.paidCancelRefundStatusName) return `${t("买家取消退款")}：${orderInfo.paidCancelRefundStatusName}`;
        if (isOfflineOrder && offlineClaimStatus === 0) return t("您已提交汇款凭证，等待财务人员核对流水，我们尽快完成审核。");
        if (isOfflineClaimRejected) {
            const reason = String(offlineClaimRejectReason || "").trim();
            return reason ? `${t("认款审核未通过")}：${reason}` : t("认款审核未通过");
        }
        if (showPayCountdown) return "";
        if (isPendingPay) return t("请尽快完成支付，以免订单被自动取消");
        const tip = String((orderInfo as any)?.orderStatusTip || "").trim();
        if (tip) return tip;
        const status = Number(orderInfo?.orderStatus ?? -1);
        const autoDays = Number(orderInfo?.autoDeliveryDays ?? 0);
        if ((status === 1 || status === 2) && autoDays > 0) {
            return t("订单将在{0}天后自动收货", { 0: autoDays });
        }
        if (status in oStatusInfo) return oStatusInfo[status];
        return "";
    })();

    /** 订单信息卡片中的状态说明；与顶部副文案相同时不再重复展示 */
    const orderCardStatusText = (() => {
        const text = orderInfo?.autoDeliveryDays
            ? t("订单将在{0}天后自动收货", { 0: orderInfo.autoDeliveryDays })
            : oStatusInfo[orderInfo?.orderStatus as number] || "";
        return text && text !== statusSubText ? text : "";
    })();

    /** 订单商品总件数（对齐跨境卡头「订单商品(N)」） */
    const itemsTotalQuantity = (() => {
        const items = orderInfo?.items;
        if (!Array.isArray(items)) return 0;
        return items.reduce((sum, p) => sum + Number(p.quantity || 0), 0);
    })();
    /** 商品卡头店名：订单 shop > 单店商品标题 > 多店留空（分组标题展示）> 官方自营店 */
    const productCardShopTitle = (() => {
        if (orderInfo?.shop?.shopTitle) return orderInfo.shop.shopTitle;
        const items = (orderInfo?.items || []) as any[];
        const shopIds = [...new Set(items.map((i) => i.shopId))];
        if (shopIds.length > 1) return "";
        if (shopIds.length === 1) {
            const first = items[0];
            if (first?.shopId === 0) return first?.shopTitle || t("官方自营店");
            if (first?.shopTitle) return first.shopTitle;
        }
        return t("官方自营店");
    })();
    /** 卡头可跳转的店铺 id；多店不跳 */
    const productCardShopId = (() => {
        if (orderInfo?.shop?.shopId != null) return orderInfo.shop.shopId;
        const items = (orderInfo?.items || []) as any[];
        const shopIds = [...new Set(items.map((i) => i.shopId))];
        if (shopIds.length === 1) return shopIds[0] ?? null;
        return null;
    })();

    const shippingFeeAmount = Number((orderInfo as any)?.shippingFee ?? 0);

    /** 跨境税费金额（仅海外站后端返回 taxRateAmount） */
    const taxRateAmountValue = (orderInfo as any)?.taxRateAmount ?? 0;
    /** 是否展示税费行：数据驱动，taxRate 或 taxRateAmount > 0 才显示 */
    const showTaxAmount = (() => {
        const taxRate = Number((orderInfo as any)?.taxRate ?? 0);
        const taxRateAmount = Number((orderInfo as any)?.taxRateAmount ?? 0);
        return taxRate > 0 || taxRateAmount > 0;
    })();

    // 优先按商品类型判断：虚拟商品=2，卡密商品=3，付费内容=4，电子礼品卡=8（这些都无需配送，不展示运费）
    const noDeliveryProductTypes = new Set([2, 3, 4, 8]);
    /**
     * - 自提：不显示
     * - 预约（orderType=9/10）：上门服务费；0元显示“免费上门”
     * - 同城配送（有配送员信息 / 配送方式含“同城”）：配送费；0元显示“免配送费”
     * - 其它：快递；0元显示“包邮”
     */
    const isPickupOrder = (() => {
        const shippingTypeName = String(orderInfo?.shippingTypeName || "");
        // 优先用后端返回的配送方式名称判断
        if (shippingTypeName.includes(t("自提"))) return true;
        // 兜底：存在自提信息且不是预约上门时，视为自提订单
        return !!orderInfo?.isPickup && !orderInfo?.isReservationHome && !!orderInfo?.pickupDetail;
    })();
    const isReserveOrder = orderInfo?.orderType === 9 || orderInfo?.orderType === 10;
    const isLocalDeliveryOrder = String(orderInfo?.shippingTypeName || "").includes("同城");
    // 虚拟内容：按商品判断（商品是虚拟商品 productType === 2 且有 virtualSample 就显示），不再依赖订单类型
    const virtualSampleItem = (() => {
        const items = ((orderInfo as any)?.items || []) as any[];
        return items.find((item) => Number(item?.productType) === 2 && !!item?.virtualSample) || null;
    })();
    // 交付附件（文件型交付，我方新增）：后端按交付门逐项下发 deliveryFiles，未达门为空数组
    const deliveryFileItems = (() => {
        const items = ((orderInfo as any)?.items || []) as any[];
        return items.filter((item) => Array.isArray(item?.deliveryFiles) && item.deliveryFiles.length > 0);
    })();
    const isNoDeliveryOrder = (() => {
        const items = ((orderInfo as any)?.items || []) as any[];
        const hasNoDeliveryProductType = items.some((item) => noDeliveryProductTypes.has(Number(item?.productType)));
        if (hasNoDeliveryProductType) return true;

        // 兜底：配送方式名称为“无需配送”
        const shippingTypeName = String((orderInfo as any)?.shippingTypeName || "");
        return shippingTypeName.includes(t("无需配送"));
    })();

    // 运费行展示规则：
    // - 到店自提：不展示
    // - 同城配送/预约上门：仍展示（即使为 0）
    // - 其它：若“无需配送”则不展示，否则展示（即使为 0）
    const showShippingFeeRow = (() => {
        // 自提：不显示
        if (isPickupOrder) return false;
        // 预约/同城：始终显示（0元显示对应文案）
        if (isReserveOrder || isLocalDeliveryOrder) return true;
        // 其它订单：若无需配送则不显示
        if (isNoDeliveryOrder) return false;
        // 默认（快递等）：显示（0元显示“包邮”）
        return true;
    })();

    const shippingFeeTitleKey = isReserveOrder ? t("上门服务费") : isLocalDeliveryOrder ? t("配送费") : t("快递");
    const shippingFeeFreeTextKey = isReserveOrder ? t("免费上门") : isLocalDeliveryOrder ? t("免配送费") : t("包邮");

    /**
     * 判断是否显示客服入口
     * - 优先使用店铺配置的客服入口（kefuInlet包含2表示开启）
     * - 无店铺配置时使用全局客服配置
     */
    const showService = (() => {
        if (orderInfo?.shop && Object.keys(orderInfo.shop).length > 0) {
            // kefuInlet=2 表示在订单详情页显示客服入口
            if (orderInfo.shop.kefuInlet && orderInfo.shop.kefuInlet.length > 0 && orderInfo.shop.kefuInlet.includes(2)) {
                return true;
            }
            return false;
        } else {
            return serviceConfig.show;
        }
    })();

    /**
     * 切换卡券显示/隐藏状态
     * 用于虚拟商品订单的券码信息保护
     */
    const toggleCardView = (cardKey: string | number) => {
        setCardViewStates((prev) => ({ ...prev, [cardKey]: !prev[cardKey] }));
    };

    const getMaskedCardNumber = (cardNumber: string, cardKey: string | number) => {
        if (cardViewStates[cardKey]) {
            return cardNumber;
        }
        if (!cardNumber || cardNumber.length <= 5) {
            return cardNumber;
        }
        const firstChar = cardNumber[0];
        const lastFour = cardNumber.slice(-4);
        const maskedLength = cardNumber.length - 5;
        return firstChar + "*".repeat(maskedLength) + lastFour;
    };

    const getMaskedCardPassword = (cardPwd: string, cardKey: string | number) => {
        if (cardViewStates[cardKey]) {
            return cardPwd;
        }
        if (!cardPwd || cardPwd.length <= 5) {
            return cardPwd;
        }
        const firstChar = cardPwd[0];
        const lastFour = cardPwd.slice(-4);
        const maskedLength = cardPwd.length - 5;
        return firstChar + "*".repeat(maskedLength) + lastFour;
    };

    /**
     * 获取订单详情
     * 将商品按店铺分组展示，支持多店铺订单场景
     */
    async function __getOrder() {
        try {
            const result = await getOrder({ id: idRef.current });
            setOrderInfo(result);
            startCountdownWatch(result);

            // 按店铺ID分组商品，用于多店铺订单展示
            const itemsByShop = new Map<number, typeof result.items>();

            result.items.forEach((item) => {
                if (!itemsByShop.has(item.shopId)) {
                    itemsByShop.set(item.shopId, []);
                }
                itemsByShop.get(item.shopId)!.push(item);
            });

            setItemsGroup(Array.from(itemsByShop.values()));
        } catch (error) {
            console.error(error);
        }
    }

    async function __getDeliveryProgress() {
        try {
            const result = await buyerDeliveryProgress(Number(idRef.current));
            setDeliveryProgress(result || null);
        } catch {
            setDeliveryProgress(null);
        }
    }

    async function __getShippingInfo() {
        try {
            const result = await getShippingInfo({ id: idRef.current });
            if (result) {
                if (Array.isArray(result)) {
                    setShippingInfo(result);
                } else {
                    setShippingInfo((result as any).traces ?? ([] as ShippingInfoTrace[]));
                }
            }
        } catch (error) {
            console.error(error);
        } finally {
            Taro.hideLoading();
        }
    }

    // 旧 onLoad
    useEffect(() => {
        const options = (router.params || {}) as Record<string, any>;
        if (options && options.id) {
            idRef.current = options.id;
            try {
                Taro.showLoading({
                    title: t("加载中")
                });
                void Promise.all([__getOrder(), __getShippingInfo(), __getDeliveryProgress()]);
            } catch (error) {
                console.error(error);
            }
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // 旧 onShow
    useDidShow(() => {
        if (isFirstLoadRef.current) {
            isFirstLoadRef.current = false;
            return;
        }
        if (idRef.current) {
            void Promise.all([__getOrder(), __getShippingInfo(), __getDeliveryProgress()]);
        }
    });

    // 后厨面板数据（我方新增）：仅已付款待发货（orderStatus=1）时拉取
    useEffect(() => {
        const oid = Number(orderInfo?.orderId || 0);
        if (!oid || Number(orderInfo?.orderStatus) !== 1) {
            setKitchenInfo(null);
            return;
        }
        let cancelled = false;
        getKitchenInfo(oid)
            .then((data) => {
                if (!cancelled) setKitchenInfo(data);
            })
            .catch(() => {
                if (!cancelled) setKitchenInfo(null);
            });
        return () => {
            cancelled = true;
        };
    }, [orderInfo?.orderId, orderInfo?.orderStatus]);

    const handleCancelOrder = (id: number) => {
        cancelOrderReasonPopupRef.current?.open(id);
    };

    // 取消并退款（我方新增）：已付未发货自助取消，二次确认后提交，
    // 退款进度在顶部状态区展示。
    const handleCancelPaidRefund = (id: number) => {
        Taro.showModal({
            title: t("取消并退款"),
            content: t("确认取消该订单吗？订单尚未发货，取消后款项将原路退回。"),
            success: async (res) => {
                if (!res.confirm) return;
                try {
                    await cancelPaidRefund({ orderId: id });
                    Taro.showToast({ title: t("已提交取消退款"), icon: "success" });
                    void __getOrder();
                } catch (e: any) {
                    Taro.showToast({ title: e?.message || t("提交失败"), icon: "none" });
                }
            }
        });
    };

    /** 催菜（我方新增）：频控在后端（10 分钟冷却 + 每日上限），成功后本地置冷却态 */
    const handleUrge = async (id: number) => {
        try {
            const data = await urgeOrder(id);
            Taro.showToast({ title: t("已催菜，商家会尽快处理"), icon: "none" });
            setKitchenInfo((prev) =>
                prev
                    ? { ...prev, urgeCount: data.urgeCount, canUrge: false, cooldownRemainSeconds: data.cooldownSeconds }
                    : prev
            );
        } catch (e: any) {
            Taro.showToast({ title: e?.message || t("催菜失败"), icon: "none" });
            // 频控类失败后刷新一次面板状态，保持按钮态与后端一致
            void getKitchenInfo(id)
                .then((data) => setKitchenInfo(data))
                .catch(() => undefined);
        }
    };

    /** 送礼物分享（我方新增）：重新生成链接（旧链接作废）并复制到剪贴板 */
    const handleShareGift = async (id: number) => {
        try {
            const data = await regenerateGiftShareToken(id);
            const base = typeof window !== "undefined" ? `${window.location.origin}/mobile/` : "";
            const link = `${base}#/pages/order/giftClaim?token=${data.token}`;
            Taro.setClipboardData({
                data: `${t("送你一份礼物，点击领取")}：${link}`,
                success: () => {
                    Taro.showToast({ title: t("领取链接已复制，发给好友吧"), icon: "none" });
                }
            });
        } catch (error: any) {
            Taro.showToast({ title: error.message || t("生成链接失败"), icon: "none" });
        }
    };

    /** 送礼物取消（我方新增）：待领取时走退款关闭，未付款时直接取消 */
    const handleCancelGift = (id: number) => {
        Taro.showModal({
            title: t("取消送礼物"),
            content: t("取消后礼物将关闭，已付款的将原路退款，确认取消吗？"),
            cancelText: t("再想想"),
            confirmText: t("确认取消"),
            success: async (res) => {
                if (res.confirm) {
                    try {
                        await cancelGiftOrder(id);
                        Taro.showToast({ title: t("已取消"), icon: "success" });
                        void __getOrder();
                    } catch (error: any) {
                        Taro.showToast({ title: error.message || t("取消失败"), icon: "none" });
                    }
                }
            }
        });
    };

    const handlePay = (id: number) => {
        Taro.navigateTo({
            url: Number(orderInfo.payTypeId) === 3 ? `/pages/order/pay?orderId=${id}` : `/pages/order/cashier?orderId=${id}&from=orderInfo`
        });
    };

    const handleToShop = (id: number) => {
        toStoreOrShopHome(id, { navType: "navigateTo" });
    };

    const handleConfirmReceipt = (id: number) => {
        Taro.showModal({
            title: t("提示"),
            content: t("确认收货吗？"),
            cancelText: t("取消"),
            confirmText: t("确认"),
            success: async (res) => {
                if (res.confirm) {
                    try {
                        await confirmReceipt({ id });
                        Taro.showToast({ title: t("确认收货成功"), icon: "success" });
                        void __getOrder();
                    } catch (error: any) {
                        Taro.showToast({
                            title: error.message,
                            icon: "none"
                        });
                    }
                }
            }
        });
    };

    /**
     * 申请售后
     * @param itemId 商品项ID，传入则为单品售后，不传则为整单售后
     */
    const handleAfterSale = (itemId: null | number) => {
        if (itemId) {
            Taro.navigateTo({
                url: `/pages/user/afterSale/edit?itemId=${itemId}&orderId=${orderInfo.orderId}`
            });
        } else {
            Taro.navigateTo({
                url: `/pages/user/afterSale/edit?orderId=${orderInfo.orderId}`
            });
        }
    };

    const handleAfterSaleDetail = (id: number) => {
        Taro.navigateTo({
            url: `/pages/user/afterSale/info?id=${id}`
        });
    };

    const handleLink = (id: number, shopId?: number) => {
        Taro.navigateTo({
            url: `/pages/product/index?id=${id}${shopId != null ? `&shopId=${shopId}` : ""}`
        });
    };

    const handleCopy = (data: any) => {
        copy(data, () => {
            Taro.showToast({
                title: t("复制成功"),
                icon: "none"
            });
        });
    };

    const handleToGrouponDetail = (grouponPromotionRecordId: number) => {
        Taro.navigateTo({
            url: `/pages/product/groupBuying?grouponPromotionRecordId=${grouponPromotionRecordId}`
        });
    };

    // 旧版保留但模板未接线的拨号方法（客服入口走 Service 组件）
    const handleCallPhone = (phone: string) => {
        if (!phone) return;
        Taro.makePhoneCall({
            phoneNumber: phone,
            fail: () => {
                Taro.showToast({
                    title: t("拨打电话失败"),
                    icon: "none"
                });
            }
        });
    };
    void handleCallPhone;

    const showBottomBar =
        (orderInfo?.availableActions?.cancelOrder ||
            orderInfo?.availableActions?.cancelPaidRefund ||
            // 后厨催菜（我方新增）：已付款待出单时底栏需可见以承载催菜入口
            (kitchenInfo?.inKitchen && kitchenInfo.kitchenStatus !== 2) ||
            (orderInfo?.availableActions?.toPay && canShowToPay) ||
            isOfflineClaimRejected ||
            canViewOfflineClaim ||
            canEditOfflineClaim ||
            orderInfo?.availableActions?.confirmReceipt ||
            (orderInfo?.availableActions?.toAftersales &&
                !orderInfo?.afterSalesIng &&
                !hasItemAftersales &&
                !giftCardAftersalesBlocked &&
                !isGiftCardExchangeOrder &&
                !pickupAftersalesDisabled &&
                !isLotteryOrder)) &&
        orderInfo?.orderSource !== "pos" &&
        !isLotteryOrder;

    const hasOrder = Object.keys(orderInfo).length > 0;

    return (
        <TigLayout>
            <TigNavbar
                title={isMeituanDelivery ? t("我的订单") : statusTitle || t("我的订单")}
                background={navbarBackground}
                borderBottom={false}
                rightSlot={
                    !isMeituanDelivery && showService ? (
                        <View className="icon-box">
                            <Service
                                phone={orderInfo.shop && orderInfo.shop.kefuPhone ? orderInfo.shop.kefuPhone : ""}
                                orderId={orderInfo.orderId}
                                shopId={orderInfo.shopId}
                            >
                                <Text className="iconfont-h5 icon-a-21" />
                            </Service>
                        </View>
                    ) : undefined
                }
            />
            <View className={`order-info${isMeituanDelivery ? " order-info--meituan" : ""}`}>
                {hasOrder ? (
                    <>
                        {!isMeituanDelivery ? (
                            <View className="order-info-steps">
                                {showPresaleCard ? (
                                    <PresaleCard presaleInfo={productPresale} total={orderInfo} />
                                ) : (
                                    <View className="order-info-header">
                                        <View className="status-title">{statusTitle}</View>
                                        {showPayCountdown || statusSubText ? (
                                            <View className="status-sub">
                                                {showPayCountdown ? (
                                                    <>
                                                        {t("还剩")}
                                                        <View className="status-countdown">
                                                            <TigCountdown
                                                                endTime={payEndTime}
                                                                fontSize={13}
                                                                color="#cc0c1c"
                                                                splitorColor="#cc0c1c"
                                                            />
                                                        </View>
                                                        {t("订单自动取消")}
                                                    </>
                                                ) : (
                                                    statusSubText
                                                )}
                                            </View>
                                        ) : null}
                                    </View>
                                )}

                                {showAddressBox && !isMeituanDelivery ? (
                                    <View className="address-box" style={{ marginTop: showPresaleCard ? "0" : "40rpx" }}>
                                        <View className="icon">
                                            <Text className="iconfont-h5 icon-dizhi" />
                                        </View>
                                        <View className="info">
                                            <View className="title">
                                                <View className="name">{orderInfo.consignee}</View>
                                                <View className="mobile">{orderInfo.mobile}</View>
                                            </View>
                                            <View className="address-txt">{orderInfo.userAddress}</View>
                                        </View>
                                    </View>
                                ) : null}
                                {orderInfo.cancelRemark?.trim() ? (
                                    <View className="cancel-reason card-space">
                                        {t("取消原因")}：{orderInfo.cancelRemark}
                                    </View>
                                ) : null}
                                {/* 拼团自提面板（我方新增，帖 407906）：自提点 + 核销码（订单号）+ 成团门提示 */}
                                {(orderInfo as any)?.orderType === 2 && isPickupOrder ? (
                                    <View className="card-space" style={{ background: "#fff", borderRadius: "18rpx", padding: "24rpx 28rpx", marginTop: "20rpx" }}>
                                        <View style={{ display: "flex", flexDirection: "row", justifyContent: "space-between", alignItems: "center" }}>
                                            <Text style={{ fontSize: "30rpx", fontWeight: 600, color: "#1a1a1a" }}>{t("拼团自提")}</Text>
                                            <Text style={{ fontSize: "26rpx", color: "#65c4aa" }}>{(orderInfo as any)?.joinStatusDesc || ""}</Text>
                                        </View>
                                        {(orderInfo as any)?.pickupDetail ? (
                                            <View style={{ fontSize: "26rpx", color: "#666", marginTop: "10rpx" }}>
                                                {t("自提点")}：{(orderInfo as any).pickupDetail.pickupName || (orderInfo as any).pickupDetail.shopTitle || ""}
                                                {(orderInfo as any).pickupDetail.address ? ` · ${(orderInfo as any).pickupDetail.address}` : ""}
                                            </View>
                                        ) : null}
                                        {(orderInfo as any)?.joinStatus === 1 ? (
                                            <View style={{ fontSize: "24rpx", color: "#999", marginTop: "8rpx" }}>
                                                {t("拼团成功后方可到店自提，请等待成团")}
                                            </View>
                                        ) : null}
                                        {(orderInfo as any)?.joinStatus === 2 ? (
                                            <View style={{ fontSize: "24rpx", color: "#999", marginTop: "8rpx" }}>
                                                {t("已成团，请凭订单号（核销码）到自提点取货")}：{orderInfo.orderSn}
                                            </View>
                                        ) : null}
                                        {(orderInfo as any)?.joinStatus === 3 ? (
                                            <View style={{ fontSize: "24rpx", color: "#999", marginTop: "8rpx" }}>
                                                {t("拼团失败，款项将自动原路退回，无需到店")}
                                            </View>
                                        ) : null}
                                    </View>
                                ) : null}
                                {/* 送礼物面板（我方新增）：状态/赠言/分享/取消 */}
                                {(orderInfo as any)?.isGiftOrder && (orderInfo as any)?.giftInfo ? (
                                    <View className="card-space" style={{ background: "#fff", borderRadius: "18rpx", padding: "24rpx 28rpx", marginTop: "20rpx" }}>
                                        <View style={{ display: "flex", flexDirection: "row", justifyContent: "space-between", alignItems: "center" }}>
                                            <Text style={{ fontSize: "30rpx", fontWeight: 600, color: "#1a1a1a" }}>{t("送礼物")}</Text>
                                            <Text style={{ fontSize: "26rpx", color: "#65c4aa" }}>{(orderInfo as any).giftInfo.giftStatusName}</Text>
                                        </View>
                                        {(orderInfo as any).giftInfo.giftMessage ? (
                                            <View style={{ fontSize: "26rpx", color: "#666", marginTop: "10rpx" }}>
                                                {t("赠言")}：「{(orderInfo as any).giftInfo.giftMessage}」
                                            </View>
                                        ) : null}
                                        {(orderInfo as any).giftInfo.giftStatus === 1 && (orderInfo as any).giftInfo.expireTime ? (
                                            <View style={{ fontSize: "24rpx", color: "#999", marginTop: "8rpx" }}>
                                                {t("好友需在")} {(orderInfo as any).giftInfo.expireTime} {t("前领取，超时自动退款")}
                                            </View>
                                        ) : null}
                                        {(orderInfo as any).giftInfo.giftStatus === 2 ? (
                                            <View style={{ fontSize: "24rpx", color: "#999", marginTop: "8rpx" }}>
                                                {t("已被")} {(orderInfo as any).giftInfo.claimUserNameMasked || ""} {t("领取")}
                                                {(orderInfo as any).giftInfo.claimTime ? ` · ${(orderInfo as any).giftInfo.claimTime}` : ""}
                                            </View>
                                        ) : null}
                                        {(orderInfo as any).giftInfo.giftStatus === 3 && (orderInfo as any).giftInfo.closeReason ? (
                                            <View style={{ fontSize: "24rpx", color: "#999", marginTop: "8rpx" }}>
                                                {(orderInfo as any).giftInfo.closeReason}
                                            </View>
                                        ) : null}
                                        {((orderInfo as any).giftInfo.giftStatus === 0 || (orderInfo as any).giftInfo.giftStatus === 1) && (
                                            <View style={{ display: "flex", flexDirection: "row", marginTop: "18rpx" }}>
                                                <View
                                                    style={{ background: "#65c4aa", color: "#fff", borderRadius: "10rpx", padding: "12rpx 26rpx", fontSize: "26rpx" }}
                                                    onClick={() => void handleShareGift(orderInfo.orderId)}
                                                >
                                                    {t("分享给好友")}
                                                </View>
                                                <View
                                                    style={{ border: "2rpx solid #ddd", color: "#666", borderRadius: "10rpx", padding: "12rpx 26rpx", fontSize: "26rpx", marginLeft: "16rpx" }}
                                                    onClick={() => handleCancelGift(orderInfo.orderId)}
                                                >
                                                    {t("取消送礼物")}
                                                </View>
                                            </View>
                                        )}
                                    </View>
                                ) : null}
                                {/* 后厨面板（我方新增）：后厨进度 + 已催次数 */}
                                {kitchenInfo?.inKitchen ? (
                                    <View className="card-space" style={{ background: "#fff", borderRadius: "18rpx", padding: "24rpx 28rpx", marginTop: "20rpx" }}>
                                        <View style={{ display: "flex", flexDirection: "row", justifyContent: "space-between", alignItems: "center" }}>
                                            <Text style={{ fontSize: "30rpx", fontWeight: 600, color: "#1a1a1a" }}>{t("后厨进度")}</Text>
                                            <Text style={{ fontSize: "26rpx", color: "#65c4aa" }}>{kitchenInfo.kitchenStatusName}</Text>
                                        </View>
                                        {kitchenInfo.kitchenStatus === 2 ? (
                                            <View style={{ fontSize: "24rpx", color: "#999", marginTop: "8rpx" }}>
                                                {t("餐品已出单，正在为您安排发货/取餐")}
                                            </View>
                                        ) : kitchenInfo.urgeCount > 0 ? (
                                            <View style={{ fontSize: "24rpx", color: "#999", marginTop: "8rpx" }}>
                                                {t("已催菜")} {kitchenInfo.urgeCount} {t("次，商家正在赶工")}
                                            </View>
                                        ) : (
                                            <View style={{ fontSize: "24rpx", color: "#999", marginTop: "8rpx" }}>
                                                {t("久等了？可在下方点「催菜」提醒商家")}
                                            </View>
                                        )}
                                    </View>
                                ) : null}
                            </View>
                        ) : null}

                        <View className="order-info-content">
                            {isOfflineOrder && Number(offlineClaimStatus) === 1 && offlineClaimAuditRemark ? (
                                <View className="offline-claim-card card-space">
                                    <View className="offline-claim-title">{getClaimStatusName(1)}</View>
                                    <View className="offline-claim-tip">
                                        {t("审核备注")}：{offlineClaimAuditRemark}
                                    </View>
                                </View>
                            ) : null}

                            <View className="product-card card-space">
                                <View
                                    className="product-card-title"
                                    onClick={() => {
                                        if (productCardShopId != null) handleToShop(productCardShopId);
                                    }}
                                >
                                    <View className="shop-info">
                                        <Text className="text">{productCardShopTitle}</Text>
                                    </View>
                                    <View className="product-card-title-right">
                                        {t("订单商品")}({itemsTotalQuantity})
                                    </View>
                                </View>
                                {itemsGroup.map((items, index) => (
                                    <View className="product-card-group" key={index}>
                                        {itemsGroup.length > 1 && (items[0].shopId === 0 || items[0]?.shopTitle) ? (
                                            <View className="product-card-group-title">
                                                <View
                                                    className="shop-info"
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                        handleToShop(items[0].shopId);
                                                    }}
                                                >
                                                    <Text className="text">
                                                        {items[0].shopId === 0 ? items[0]?.shopTitle || t("自营") : items[0]?.shopTitle}
                                                    </Text>
                                                    <Text className="iconfont icon-xiangyou" />
                                                </View>
                                            </View>
                                        ) : null}

                                        {items.map((item: OrderInfoItemElement) => (
                                            <View className="item-box" key={item.productId}>
                                                <View className="product-card-item">
                                                    <View
                                                        className="product-card-item-left"
                                                        onClick={() => handleLink(item.productId, orderInfo.shopId)}
                                                    >
                                                        <View className="left-img">
                                                            <TigImage src={pickSkuPic(item)} />
                                                            {item.isGift === 1 ? (
                                                                <View className="gift-text">{t("赠品")}</View>
                                                            ) : null}
                                                        </View>
                                                    </View>
                                                    <View
                                                        className="product-card-item-right"
                                                        onClick={() => handleLink(item.productId, orderInfo.shopId)}
                                                    >
                                                        <View className="product-info-box">
                                                            <View className="right-title line2">
                                                                {showPresaleCard ? (
                                                                    <Text className="presale-tag">{t("预售")}</Text>
                                                                ) : null}
                                                                <Text>{item.productName}</Text>
                                                            </View>
                                                            <View className="product-price">
                                                                <FormatPrice
                                                                    showText={false}
                                                                    fontStyle={amountStyle}
                                                                    currencyStyle={amountStyle}
                                                                    decimalsStyle={amountStyle}
                                                                    withExact priceData={item.price}
                                                                />
                                                            </View>
                                                        </View>
                                                        {hasSkuText(item.skuData) ? (
                                                            <View className="sku-card">
                                                                <View className="sku-item line1">{getSkuText(item.skuData)}</View>
                                                            </View>
                                                        ) : null}
                                                        {item.extraSkuData && item.extraSkuData.length ? (
                                                            <View className="extraskudata-box">
                                                                <View className="extraskudata">
                                                                    {item.extraSkuData.map((attr: any, attrIndex: number) => (
                                                                        <View className="extraskudata-item" key={attrIndex}>
                                                                            <View className="title">{attr.attrName}</View>
                                                                            <View className="line-box">
                                                                                <View className="line" />
                                                                            </View>
                                                                            <View className="content">
                                                                                <Text className="content-text">{attr.attrValue}</Text>
                                                                                <View className="price">
                                                                                    <FormatPrice showText={false} withExact priceData={attr.attrPrice} />
                                                                                </View>
                                                                                <View className="num">x{item.quantity}</View>
                                                                            </View>
                                                                        </View>
                                                                    ))}
                                                                </View>
                                                            </View>
                                                        ) : null}
                                                        <View className="right-price">
                                                            <View className="right-price-quantity">
                                                                x<Text>{item.quantity}</Text>
                                                            </View>
                                                        </View>
                                                    </View>
                                                    {!isMeituanDelivery &&
                                                    orderInfo.payStatus === 2 &&
                                                    item.isGift === 0 &&
                                                    !pickupAftersalesDisabled &&
                                                    !isLotteryOrder ? (
                                                        <View className="item-btn">
                                                            {orderInfo.availableActions?.toAftersales &&
                                                            canItemApplyAftersales(item) &&
                                                            !isGiftCardExchangeOrder &&
                                                            orderInfo.orderSource !== "pos" &&
                                                            !isLotteryOrder &&
                                                            !(
                                                                (orderInfo as any).cycleInfo &&
                                                                Object.keys((orderInfo as any).cycleInfo)?.length > 0
                                                            ) &&
                                                            item.quantity > (item.aftersalesNumber || 0) ? (
                                                                <TigButton2 className="btn" onClick={() => handleAfterSale(item.itemId)}>
                                                                    {t("申请售后")}
                                                                </TigButton2>
                                                            ) : item.aftersalesItem ? (
                                                                <TigButton2
                                                                    className="btn"
                                                                    onClick={() => handleAfterSaleDetail(item.aftersalesItem.aftersaleId)}
                                                                >
                                                                    {t("售后详情")}
                                                                </TigButton2>
                                                            ) : null}
                                                        </View>
                                                    ) : null}
                                                </View>
                                            </View>
                                        ))}
                                    </View>
                                ))}
                            </View>

                            {hasECardInfo ? (
                                <View className="product-card1 card-space">
                                    {orderInfo.items.map((item) => {
                                        const cardKeyBase = item.itemId || item.productId;
                                        return item.eCard?.length > 0 ? (
                                            <View className="item-box" key={item.productId}>
                                                <View className="virtualsample-box">
                                                    <View className="virtualsample-header">
                                                        <View className="virtualsample-title">{t("券码信息")}</View>
                                                        <View className="virtualsample-count">
                                                            ({t("共{0}张", { 0: item.eCard?.length || 0 })})
                                                        </View>
                                                    </View>
                                                    {item.eCard.map((subItem, cardIndex) => {
                                                        const cardKey = `${cardKeyBase}-${cardIndex}`;
                                                        return (
                                                            <View className="virtualsample-card" key={cardKey}>
                                                                <View className="virtualsample-item">
                                                                    <View className="virtualsample-item-con">
                                                                        <View className="label">{t("卡号")}：</View>
                                                                        <View className="value">
                                                                            {getMaskedCardNumber(subItem.cardNumber, cardKey)}
                                                                        </View>
                                                                    </View>
                                                                    <View
                                                                        className="virtualsample-view-btn"
                                                                        onClick={() => toggleCardView(cardKey)}
                                                                    >
                                                                        <Text
                                                                            className={`iconfont-h5 ${
                                                                                cardViewStates[cardKey] ? "icon-xianshi" : "icon-buxianshi"
                                                                            }`}
                                                                        />
                                                                        <Text className="view-text">{t("查看券码")}</Text>
                                                                    </View>
                                                                    <View
                                                                        className="virtualsample-view-btn"
                                                                        onClick={() => {
                                                                            copy(subItem.cardNumber, () => {
                                                                                Taro.showToast({ title: t("卡号已复制"), icon: "none" });
                                                                            });
                                                                        }}
                                                                    >
                                                                        <Text className="view-text">{t("复制")}</Text>
                                                                    </View>
                                                                </View>
                                                                <View className="virtualsample-item">
                                                                    <View className="virtualsample-item-con">
                                                                        <View className="label">{t("卡密")}：</View>
                                                                        <View className="value">
                                                                            {getMaskedCardPassword(subItem.cardPwd, cardKey)}
                                                                        </View>
                                                                    </View>
                                                                    <View
                                                                        className="virtualsample-view-btn"
                                                                        onClick={() => {
                                                                            copy(subItem.cardPwd, () => {
                                                                                Taro.showToast({ title: t("卡密已复制"), icon: "none" });
                                                                            });
                                                                        }}
                                                                    >
                                                                        <Text className="view-text">{t("复制")}</Text>
                                                                    </View>
                                                                </View>
                                                            </View>
                                                        );
                                                    })}
                                                </View>
                                            </View>
                                        ) : null;
                                    })}
                                </View>
                            ) : null}

                            {virtualSampleItem ? (
                                <View className="virtualsample card-space">
                                    <View className="virtualsample-label">{t("虚拟内容")}：</View>
                                    <View className="virtualsample-value">
                                        <RichText className="desc-text-item" nodes={virtualSampleItem.virtualSample} />
                                        {/* 交付链接行（打开/复制），不依赖 RichText 锚点 */}
                                        <DeliveryLinks html={virtualSampleItem.virtualSample} />
                                    </View>
                                </View>
                            ) : null}

                            {deliveryFileItems.map((item) => (
                                <View className="virtualsample card-space" key={item.itemId || item.productId}>
                                    <View className="virtualsample-label">{t("交付附件")}：</View>
                                    <View className="virtualsample-value">
                                        {/* 点击申领限时令牌后下载（接口层强校验） */}
                                        <DeliveryFiles files={item.deliveryFiles} orderId={orderInfo.orderId} />
                                    </View>
                                </View>
                            ))}

                            <View className="money-card card-space">
                                <View className="money-title-box" onClick={() => setShowUp((prev) => !prev)}>
                                    <View className="money-title">
                                        {t("订单总价")}
                                        {isOverseasSkin ? <Text>{t("(含增值税)")}</Text> : null}
                                    </View>
                                    <View className="money-price">
                                        <View style={{ marginRight: "10rpx" }}>
                                            <FormatPrice
                                                showText={false}
                                                isBottom={false}
                                                fontStyle={totalStyle}
                                                currencyStyle={totalStyle}
                                                decimalsStyle={totalStyle}
                                                withExact priceData={orderInfo.totalAmount}
                                            />
                                        </View>
                                        {showUp ? (
                                            <Text className="iconfont-h5 icon-up" />
                                        ) : (
                                            <Text className="iconfont-h5 icon-arrowdown" />
                                        )}
                                    </View>
                                </View>
                                {showUp ? (
                                    <>
                                        <View className="money-card-item">
                                            <Text className="order-item-label bold">{t("商品总价")}</Text>
                                            <FormatPrice
                                                showText={false}
                                                isBottom={false}
                                                fontStyle={amountStyle}
                                                currencyStyle={amountStyle}
                                                decimalsStyle={amountStyle}
                                                withExact priceData={orderInfo.productAmount}
                                            />
                                        </View>
                                        {orderInfo.unpaidAmount && orderInfo.unpaidAmount > 0 ? (
                                            <View className="money-card-item">
                                                <Text className="order-item-label bold">{t("待支付金额")}</Text>
                                                <FormatPrice
                                                    showText={false}
                                                    isBottom={false}
                                                    fontStyle={amountStyle}
                                                    currencyStyle={amountStyle}
                                                    decimalsStyle={amountStyle}
                                                    withExact priceData={orderInfo.unpaidAmount}
                                                />
                                            </View>
                                        ) : null}
                                        {/* 税费行由 taxRate 与 taxRateAmount 数据控制 */}
                                        {showTaxAmount ? (
                                            <View className="money-card-item">
                                                <Text className="order-item-label">{t("税费")}</Text>
                                                <FormatPrice
                                                    showText={false}
                                                    isBottom={false}
                                                    fontStyle={amountStyle}
                                                    currencyStyle={amountStyle}
                                                    decimalsStyle={amountStyle}
                                                    withExact priceData={taxRateAmountValue}
                                                />
                                            </View>
                                        ) : null}
                                        {orderInfo.serviceFee && Number(orderInfo.serviceFee) > 0 ? (
                                            <View className="money-card-item">
                                                <Text className="order-item-label">{t("附加费用")}</Text>
                                                <FormatPrice
                                                    showText={false}
                                                    isBottom={false}
                                                    fontStyle={amountStyle}
                                                    currencyStyle={amountStyle}
                                                    decimalsStyle={amountStyle}
                                                    withExact priceData={orderInfo.serviceFee}
                                                />
                                            </View>
                                        ) : null}
                                        {/* 运费：除到店自提外，其它类型订单均需展示；运费=0 显示对应文案 */}
                                        {showShippingFeeRow ? (
                                            <View className="money-card-item">
                                                <Text className="order-item-label">{t(shippingFeeTitleKey)}</Text>
                                                {shippingFeeAmount > 0 ? (
                                                    <FormatPrice
                                                        showText={false}
                                                        isBottom={false}
                                                        fontStyle={amountStyle}
                                                        currencyStyle={amountStyle}
                                                        decimalsStyle={amountStyle}
                                                        withExact priceData={shippingFeeAmount}
                                                    />
                                                ) : (
                                                    <Text className="free-text">{t(shippingFeeFreeTextKey)}</Text>
                                                )}
                                            </View>
                                        ) : null}
                                        {(orderInfo as any).cycleCardInfo && (orderInfo as any).cycleCardInfo?.unitPrice ? (
                                            <View className="money-card-item">
                                                <Text className="order-item-label">{t("次卡抵扣")}</Text>
                                                <View className="order-item-price">
                                                    <Text>-</Text>
                                                    <FormatPrice
                                                        showText={false}
                                                        isBottom={false}
                                                        fontStyle={discountStyle}
                                                        currencyStyle={discountStyle}
                                                        decimalsStyle={discountStyle}
                                                        priceData={(orderInfo as any).cycleCardInfo?.unitPrice}
                                                    />
                                                </View>
                                            </View>
                                        ) : null}
                                        {orderInfo.balance && orderInfo.balance > 0 ? (
                                            <View className="money-card-item">
                                                <Text className="order-item-label">{t("使用余额")}</Text>
                                                <FormatPrice
                                                    base
                                                    showText={false}
                                                    isBottom={false}
                                                    fontStyle={discountStyle}
                                                    currencyStyle={discountStyle}
                                                    decimalsStyle={discountStyle}
                                                    withExact priceData={orderInfo.balance}
                                                />
                                            </View>
                                        ) : null}
                                        {orderInfo.pointsAmount && orderInfo.pointsAmount > 0 ? (
                                            <View className="money-card-item">
                                                <Text className="order-item-label">
                                                    {`${t("使用")}${configIntegralName}`}
                                                </Text>
                                                <FormatPrice
                                                    showText={false}
                                                    isBottom={false}
                                                    fontStyle={discountStyle}
                                                    currencyStyle={discountStyle}
                                                    decimalsStyle={discountStyle}
                                                    withExact priceData={orderInfo.pointsAmount}
                                                />
                                            </View>
                                        ) : null}
                                        {orderInfo.couponAmount && orderInfo.couponAmount > 0 ? (
                                            <View className="money-card-item">
                                                <Text className="order-item-label">{t("优惠券")}</Text>
                                                <FormatPrice
                                                    showText={false}
                                                    isBottom={false}
                                                    fontStyle={discountStyle}
                                                    currencyStyle={discountStyle}
                                                    decimalsStyle={discountStyle}
                                                    withExact priceData={orderInfo.couponAmount}
                                                />
                                            </View>
                                        ) : null}
                                        {orderInfo.discountAmount && orderInfo.discountAmount > 0 ? (
                                            <View className="money-card-item">
                                                <Text className="order-item-label">{t("其他优惠")}</Text>
                                                <FormatPrice
                                                    showText={false}
                                                    isBottom={false}
                                                    fontStyle={discountStyle}
                                                    currencyStyle={discountStyle}
                                                    decimalsStyle={discountStyle}
                                                    withExact priceData={orderInfo.discountAmount}
                                                />
                                            </View>
                                        ) : null}
                                        {orderInfo.payStatus == 2 ? (
                                            <View className="money-card-item">
                                                <Text className="order-item-label bold">{t("实付款")}</Text>
                                                <FormatPrice
                                                    showText={false}
                                                    isBottom={false}
                                                    fontStyle={amountStyle}
                                                    currencyStyle={amountStyle}
                                                    decimalsStyle={amountStyle}
                                                    withExact priceData={orderInfo.totalAmount}
                                                />
                                            </View>
                                        ) : null}
                                    </>
                                ) : null}
                            </View>

                            {!isMeituanDelivery ? (
                                <View className="pay-card card-space">
                                    {orderInfo.shippingTypeName ? (
                                        <View className="pay-card-item">
                                            <Text className="order-item-label">{t("配送方式")}</Text>
                                            <Text>{t(orderInfo.shippingTypeName)}</Text>
                                        </View>
                                    ) : null}
                                    <View className="pay-card-item">
                                        <Text className="order-item-label">{t("支付方式")}：</Text>
                                        {isGiftCardExchangeOrder ? (
                                            <Text>{t("礼品卡兑换")}</Text>
                                        ) : (
                                            <Text>
                                                {orderInfo.payTypeId === 1
                                                    ? t("在线支付")
                                                    : orderInfo.payTypeId === 2
                                                      ? t("货到付款")
                                                      : t("线下支付")}
                                            </Text>
                                        )}
                                    </View>
                                </View>
                            ) : null}

                            {!isMeituanDelivery ? (
                                <View className="order-card card-space">
                                    <View className="order-card-item justify-between">
                                        <Text className="order-item-label">{t("订单编号")}</Text>
                                        <Text>{orderInfo.orderSn}</Text>
                                    </View>
                                    <View className="order-card-item justify-between">
                                        <Text className="order-item-label">{t("下单时间")}</Text>
                                        <Text>{orderInfo.addTime}</Text>
                                    </View>
                                    {orderCardStatusText ? <View className="order-card-item">{orderCardStatusText}</View> : null}
                                </View>
                            ) : null}

                            {deliveryProgress ? (
                                <View className="expressage-card card-space">
                                    <View className="shippingInfo-title">
                                        {deliveryProgress.type === 2 ? t("服务进度") : t("配送进度")}
                                        {deliveryProgress.staffName ? (
                                            <Text style={{ marginLeft: "16rpx", fontSize: "26rpx", color: "#666" }}>
                                                {deliveryProgress.type === 2 ? t("服务人员") : t("骑手")}：{deliveryProgress.staffName}
                                                {deliveryProgress.staffPhone ? ` · ${deliveryProgress.staffPhone}` : ""}
                                            </Text>
                                        ) : null}
                                    </View>
                                    <View className="shipping-steps">
                                        {(deliveryProgress.steps || []).map((step: any, index: number) => (
                                            <View className="shipping-step" key={index}>
                                                <View className={`step-dot${step.done ? " active" : ""}`} />
                                                <View className="step-content">
                                                    <View className="step-title">{step.label}</View>
                                                    {step.time ? (
                                                        <View className="step-desc">
                                                            {(() => {
                                                                const d = new Date(Number(step.time) * 1000);
                                                                const p = (n: number) => String(n).padStart(2, "0");
                                                                return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
                                                            })()}
                                                        </View>
                                                    ) : null}
                                                </View>
                                            </View>
                                        ))}
                                    </View>
                                    {deliveryProgress.status === 50 && deliveryProgress.cancelReason ? (
                                        <View style={{ fontSize: "26rpx", color: "#999", marginTop: "8rpx" }}>
                                            {t("取消原因")}：{deliveryProgress.cancelReason}
                                        </View>
                                    ) : null}
                                </View>
                            ) : null}

                            {!isMeituanDelivery &&
                            orderInfo.orderId &&
                            (orderInfo.orderStatus === 2 || orderInfo.orderStatus === 5) &&
                            (orderInfo as any).isCycleOrder !== 1 &&
                            orderInfo.shippingTypeId !== 3 ? (
                                <View className="expressage-card card-space">
                                    {shippingInfo.length === 0 ? (
                                        <View className="shippingInfo-title">{t("物流信息")}</View>
                                    ) : null}

                                    <View className="expressage-box">
                                        <View className="expressage-info">
                                            <View className="">{orderInfo.logisticsName}</View>
                                            <View className="">{orderInfo.trackingNo}</View>
                                        </View>
                                        {orderInfo.logisticsName !== "无需配送" ? (
                                            <View className="copy-btn" onClick={() => handleCopy(orderInfo.trackingNo)}>
                                                {t("复制")}
                                            </View>
                                        ) : null}
                                    </View>
                                    {shippingInfo.length > 0 ? (
                                        <View className="shippingInfo-box">
                                            <View className={`shippingInfo-box-content${!showMore ? " dim" : ""}${showMore ? " more-height" : ""}`}>
                                                {/* 旧 up-steps（uview-plus，column/dot）→ 自绘时间轴 */}
                                                <View className="shipping-steps">
                                                    {shippingInfo.map((trace, index) => (
                                                        <View className="shipping-step" key={index}>
                                                            <View className={`step-dot${index === 0 ? " active" : ""}`} />
                                                            <View className="step-content">
                                                                <View className="step-title">{trace.acceptStation}</View>
                                                                <View className="step-desc">{trace.acceptTime}</View>
                                                            </View>
                                                        </View>
                                                    ))}
                                                </View>
                                            </View>
                                            <View className="shippingInfo-btn" onClick={() => setShowMore((prev) => !prev)}>
                                                {showMore ? t("收起") : t("展开")}
                                                {showMore ? (
                                                    <Text className="iconfont-h5 icon-up" />
                                                ) : (
                                                    <Text className="iconfont-h5 icon-arrowdown" />
                                                )}
                                            </View>
                                        </View>
                                    ) : (
                                        <View className="emptyText">{emptyText}</View>
                                    )}
                                </View>
                            ) : null}
                        </View>

                        {/* orderType 14 为抽奖商品订单：禁展示取消/付款/确认收货/售后底栏 */}
                        {showBottomBar ? (
                            <View className="fixed-placeholder">
                                <View className="content action-bar-eGNvMjI4Mno">
                                    <View className="order-info-btn">
                                        <View className={`order-info-btn-box${showPresaleFinalPayBtn ? " presale-btn-box" : ""}`}>
                                            {orderInfo.availableActions?.cancelOrder ? (
                                                <TigButton2 className="btn" onClick={() => handleCancelOrder(orderInfo.orderId)}>
                                                    {t("取消订单")}
                                                </TigButton2>
                                            ) : null}
                                            {orderInfo.availableActions?.cancelPaidRefund ? (
                                                <TigButton2 className="btn" onClick={() => handleCancelPaidRefund(orderInfo.orderId)}>
                                                    {t("取消并退款")}
                                                </TigButton2>
                                            ) : null}
                                            {kitchenInfo?.inKitchen && kitchenInfo.kitchenStatus !== 2 ? (
                                                kitchenInfo.canUrge ? (
                                                    <TigButton2 className="btn" onClick={() => void handleUrge(orderInfo.orderId)}>
                                                        {t("催菜")}
                                                    </TigButton2>
                                                ) : (
                                                    <TigButton2 className="btn" disabled>
                                                        {t("已催菜")}
                                                    </TigButton2>
                                                )
                                            ) : null}
                                            {orderInfo.availableActions?.toPay && canShowToPay ? (
                                                <TigButton2
                                                    theme
                                                    style={{ height: "100%" }}
                                                    className={showPresaleFinalPayBtn ? "presale-final-btn" : ""}
                                                    disabled={isFinalPaymentBlocked}
                                                    onClick={() => handlePay(orderInfo.orderId)}
                                                >
                                                    {showPresaleFinalPayBtn ? (
                                                        <View className="presale-final-btn-content">
                                                            <View className="presale-final-btn-amount">
                                                                <Text className="presale-final-btn-text">{t("尾款")}</Text>
                                                                <FormatPrice
                                                                    isBottom={false}
                                                                    withExact priceData={productPresale?.finalPaymentAmount}
                                                                    fontStyle={{ fontSize: "26rpx", fontWeight: "bold" }}
                                                                />
                                                            </View>
                                                            <Text className="presale-final-btn-text">{t("去支付")}</Text>
                                                        </View>
                                                    ) : isFinalPaymentBlocked ? (
                                                        t("尾款未到支付时间")
                                                    ) : (
                                                        t("去支付")
                                                    )}
                                                </TigButton2>
                                            ) : null}
                                            {canViewOfflineClaim ? (
                                                <TigButton2 className="btn" onClick={() => handleOfflineClaim(orderInfo.orderId, "detail")}>
                                                    {t("认款详情")}
                                                </TigButton2>
                                            ) : null}
                                            {canEditOfflineClaim ? (
                                                <TigButton2 className="btn" onClick={() => handleOfflineClaim(orderInfo.orderId, "edit")}>
                                                    {t("修改申请")}
                                                </TigButton2>
                                            ) : null}
                                            {orderInfo.availableActions?.confirmReceipt ? (
                                                <TigButton2 className="btn" onClick={() => handleConfirmReceipt(orderInfo.orderId)}>
                                                    {t("确认已收货")}
                                                </TigButton2>
                                            ) : null}
                                            {orderInfo.availableActions?.toAftersales &&
                                            !orderInfo.afterSalesIng &&
                                            !hasItemAftersales &&
                                            !giftCardAftersalesBlocked &&
                                            !isGiftCardExchangeOrder &&
                                            orderInfo.orderSource !== "pos" &&
                                            !pickupAftersalesDisabled &&
                                            !isLotteryOrder ? (
                                                <TigButton2 className="btn" onClick={() => handleAfterSale(null)}>
                                                    {t("整单售后")}
                                                </TigButton2>
                                            ) : null}
                                            {orderInfo.grouponPromotionRecordId ? (
                                                <TigButton2
                                                    className="btn"
                                                    plain
                                                    onClick={() => handleToGrouponDetail(orderInfo.grouponPromotionRecordId)}
                                                >
                                                    {t("拼团详情")}
                                                </TigButton2>
                                            ) : null}
                                        </View>
                                    </View>
                                </View>
                                <View className="placeholder" style={{ paddingTop: "calc(100rpx + var(--safe-bottom))" }} />
                            </View>
                        ) : null}
                    </>
                ) : null}
            </View>
            {/* 放在 tig-layout 内才能继承主题色变量 */}
            <CancelOrderReasonPopup ref={cancelOrderReasonPopupRef} onSuccess={__getOrder} />
        </TigLayout>
    );
}