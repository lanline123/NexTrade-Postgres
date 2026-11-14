import { useMemo } from "react";
import { Image, Text, View } from "@tarojs/components";

import { formatFrame } from "@/components/modules";
import { useUserStore } from "@/store/user";
import { redirect } from "@/utils";
import { imageFormat } from "@/utils/format";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

const ORDER_ENTRIES = [
    { title: "待付款", link: "/pages/user/order/list?type=awaitPay", badgeKey: "awaitPay" },
    { title: "待发货", link: "/pages/user/order/list?type=awaitShipping", badgeKey: "awaitShipping" },
    { title: "待收货", link: "/pages/user/order/list?type=awaitReceived", badgeKey: "awaitReceived" },
    { title: "待评价", link: "/pages/user/order/list?type=awaitComment", badgeKey: "awaitComment" },
    { title: "售后", link: "/pages/user/afterSale/list", badgeKey: "awaitAftersalesCollect" }
] as const;

/**
 * 店铺订单卡装修模块（type=storeOrderCard，本线新增渲染入口）。
 * 与 storeNav 同：admin 侧无源契约（generic defaults
 * {isShow, title:'我的订单', showCount:3, itemTitle}，线2 已 visible:false）。
 * 渲染语义对齐会员装修的 orderEntry（同为订单状态入口 + 角标）：
 * 条目取 useUserStore.orderNum 角标（awaitPay/awaitShipping/
 * awaitReceived/awaitComment/awaitAftersalesCollect），showCount 截断；
 * 数据若带 picList 则以其图标/标题覆盖默认条目（链接仍走订单列表）。
 */
export default function StoreOrderCard({ module }: Props) {
    const d = useMemo(() => {
        const data = module?.data && typeof module.data === "object" ? module.data : {};
        return { ...module, ...data };
    }, [module]);

    const frameFormat = useMemo(() => formatFrame(d.frame ?? {}), [d]);
    const orderNum: any = useUserStore((s) => s.orderNum) || {};

    const showCount = Number(d.showCount ?? 5) || 5;
    const picList: any[] = Array.isArray(d.picList) ? d.picList : [];
    const entries = ORDER_ENTRIES.slice(0, showCount).map((entry, i) => ({
        ...entry,
        title: picList[i]?.picTitle || entry.title,
        picUrl: picList[i]?.picUrl?.picUrl || picList[i]?.picUrl || ""
    }));

    return (
        <View
            className="module-ad-con module-store-order-card"
            style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
        >
            <View className="module-ad-content" style={frameFormat.innerPadding + frameFormat.backgroundColor + frameFormat.boxRadius}>
                <View className="store-order-card__head">
                    <Text className="store-order-card__title">{t(d.title || "我的订单")}</Text>
                    <Text className="store-order-card__all" onClick={() => redirect({ url: "/pages/user/order/list" })}>
                        {t("全部订单")} &gt;
                    </Text>
                </View>
                <View className="store-order-card__row">
                    {entries.map((entry) => {
                        const badge = Number(orderNum[entry.badgeKey] || 0);
                        return (
                            <View key={entry.badgeKey} className="store-order-card__item" onClick={() => redirect({ url: entry.link })}>
                                <View className="store-order-card__icon-wrap">
                                    {entry.picUrl ? (
                                        <Image className="store-order-card__icon" src={imageFormat(entry.picUrl)} mode="aspectFill" />
                                    ) : (
                                        <View className="store-order-card__icon-placeholder" />
                                    )}
                                    {badge > 0 ? <Text className="store-order-card__badge">{badge > 99 ? "99+" : badge}</Text> : null}
                                </View>
                                <Text className="store-order-card__text">{t(entry.title)}</Text>
                            </View>
                        );
                    })}
                </View>
            </View>
        </View>
    );
}
