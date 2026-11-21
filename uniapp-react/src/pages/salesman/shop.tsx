import { useEffect, useState } from "react";
import { Text, View } from "@tarojs/components";
import Taro, { useRouter } from "@tarojs/taro";

import FormatPrice from "@/components/format-price/FormatPrice";
import FixedPlaceholder from "@/components/tig/tig-fixed-placeholder/FixedPlaceholder";
import TigImage from "@/components/tig/tig-image/TigImage";
import TigLayout from "@/components/tig/tig-layout/TigLayout";
import { getCategoryProduct } from "@/api/search/search";
import { getShellShopDetail, type ShellShopDetail } from "@/api/salesman/salesmanShop";
import useList from "@/hooks/useList";
import { useUserStore } from "@/store/user";
import { redirect } from "@/utils";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";

import Empty from "./src/Empty";

import "./shop.scss";

/**
 * 分销员外壳店「店中店」买家页（我方新增，论坛帖 257876041323848173）。
 *
 * 只是呈现 + 归因层：
 * - 店面皮 = 分销员自定义店名 / 店招图 / 简介（未自定义回退昵称）；
 * - 商品流 = 主商城商品（getCategoryProduct），点商品带 salesmanId 进
 *   商品页，由商品页既有逻辑写 salesmanProducts / bindSalesmanId；
 * - 落地归因与分销员名片页同口径：无条件写本地 bindSalesmanId，
 *   注册时由注册页读取并走既有绑定（salesman_customer 先绑为准、
 *   不覆盖）；冲突口径一律以既有绑定规则为准，本页不另造逻辑；
 * - 平台禁用（enabled=false）时只展示关闭提示，**不写归因**、不出
 *   商品流；禁用不动分销员身份 / 绑定 / 佣金。
 */
export default function ShellShop() {
    usePageGuards();

    const router = useRouter();
    const token = useUserStore((s) => s.token);

    const [salesmanId, setSalesmanId] = useState<number | null>(null);
    const [detail, setDetail] = useState<ShellShopDetail | null>(null);
    const [loadError, setLoadError] = useState(false);

    const { data: list, total, isLoading, getList } = useList<any>(getCategoryProduct as any, {
        params: { page: 1, size: 20 },
        path: { dataKey: "records", totalKey: "total", pageKey: "page", sizeKey: "size" },
        needReachBottom: true
    });

    const tabbarList = [
        { name: t("小店"), icon: "icon-dianpu", url: "" },
        { name: t("商城"), icon: "icon-shangcheng-", url: "/pages/index/index" },
        { name: t("我的"), icon: "icon-wode", url: "/pages/member/index" }
    ];

    const handleLink = (id: number) => {
        const url = salesmanId ? `/pages/product/index?id=${id}&salesmanId=${salesmanId}` : `/pages/product/index?id=${id}`;
        Taro.navigateTo({ url });
    };

    const handleTabbar = (url: string) => {
        if (!url) return;
        if (!token) {
            const backUrl = salesmanId ? `/pages/salesman/shop?salesmanId=${salesmanId}` : "/pages/salesman/shop";
            Taro.navigateTo({
                url: `/pages/login/index?url=${encodeURIComponent(backUrl)}`
            });
            return;
        }
        redirect({ url });
    };

    useEffect(() => {
        const optionId = router.params?.salesmanId;
        if (!optionId) {
            setLoadError(true);
            return;
        }
        const id = Number(optionId);
        setSalesmanId(id);
        void (async () => {
            try {
                const result = await getShellShopDetail(id);
                setDetail(result || null);
                if (result && result.enabled) {
                    // 与名片页同口径：落地即写归因标记（注册绑定先绑为准）
                    Taro.setStorageSync("bindSalesmanId", id);
                    void getList();
                }
            } catch (error) {
                console.error(error);
                setLoadError(true);
            }
        })();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    return (
        <View>
            <TigLayout title={detail?.shopName || t("分销员小店")} safeTopBgColor="#f5f5f5">
                {loadError ? (
                    <View className="shell-shop-state">
                        <Text>{t("小店不存在或链接已失效")}</Text>
                    </View>
                ) : !detail ? (
                    <View className="shell-shop-state">
                        <Text>{t("加载中…")}</Text>
                    </View>
                ) : !detail.enabled ? (
                    <View className="shell-shop-state">
                        <Text>{t("该小店已暂时关闭")}</Text>
                    </View>
                ) : (
                    <>
                        <View className="shell-shop-header">
                            {detail.banner ? (
                                <View className="shell-shop-banner">
                                    <TigImage src={detail.banner} />
                                </View>
                            ) : null}
                            <View className="shell-shop-profile">
                                <View className="shell-shop-avatar">
                                    <TigImage src={detail.avatar} />
                                </View>
                                <View className="shell-shop-meta">
                                    <View className="shell-shop-name">{detail.shopName}</View>
                                    {detail.intro ? <View className="shell-shop-intro">{detail.intro}</View> : null}
                                </View>
                            </View>
                        </View>

                        <View className="shell-shop-list">
                            <View className="shell-shop-list-title">{t("小店商品")}</View>
                            <View className="shell-shop-list-content">
                                {list.map((item: any) => (
                                    <View className="shell-shop-item" key={item.productId} onClick={() => handleLink(item.productId)}>
                                        <View className="shell-shop-item-img">
                                            <TigImage src={item.picUrl} />
                                        </View>
                                        <View className="shell-shop-item-text">
                                            <View className="shell-shop-item-name line2">{item.productName}</View>
                                            <View className="shell-shop-item-price">
                                                <FormatPrice displayPrice priceData={item.productPrice} />
                                                <Text className="iconfont-h5 icon-gouwuche3" />
                                            </View>
                                        </View>
                                    </View>
                                ))}
                            </View>
                            {!isLoading && total === 0 ? <Empty styles={{ width: "320rpx", height: "320rpx" }} /> : null}
                        </View>
                    </>
                )}

                <FixedPlaceholder backgroundColor="#fff">
                    <View className="bottom-btns">
                        {tabbarList.map((item, index) => (
                            <View
                                className={`bottom-btns-item${index === 0 ? " active" : ""}`}
                                key={index}
                                onClick={() => handleTabbar(item.url)}
                            >
                                <View className={`bottom-btns-item-icon iconfont-h5 ${item.icon}`} />
                                <View className="bottom-btns-item-text">{t(item.name)}</View>
                            </View>
                        ))}
                    </View>
                </FixedPlaceholder>
            </TigLayout>
        </View>
    );
}
