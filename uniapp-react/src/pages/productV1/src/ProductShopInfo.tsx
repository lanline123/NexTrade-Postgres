import { Text, View } from "@tarojs/components";

import type { StoreDetailData } from "@/types/store/store";
import ShopLabelChips from "@/components/shop-label-chips/ShopLabelChips";
import { redirect } from "@/utils";
import { imageFormat } from "@/utils/format";
import { useRegionStore } from "@/store/region";
import { useConfigStore } from "@/store/config";
import { t } from "@/utils/i18n";

import "./ProductShopInfo.scss";

interface Props {
    storeInfo: StoreDetailData;
    productId?: number;
    shopId?: number;
    formType: boolean;
}

/**
 * 商品门店信息组件（自旧 pages/productV1/src/productShopInfo.vue 移植）
 * - 显示门店名称
 * - 支持点击跳转门店详情 / 切换门店
 *
 * 复用决策：B2 pages/product/src 下无等价组件（merchantShopInfo 为店铺会员卡，
 * props 契约不同），故按旧源码独立落盘到 productV1/src。
 */
export default function ProductShopInfo({ storeInfo, productId, shopId, formType }: Props) {
    const configStore = useConfigStore();

    const showStoreInfo = {
        shopTitle: storeInfo?.shopTitle || t("商城自营"),
        shopLogo: storeInfo?.shopLogo || configStore.baseInfo.shopLogo
    };

    const handleToShop = () => {
        // 2026-11-22 体检修复：原守卫 !shopId 把平台自营 shopId=0 也拦掉（本组件恰为自营渲染
        // 「商城自营」兜底标题，整卡点击却无反应）；与 MerchantShopInfo 同口径：仅拦缺失/负数。
        if (!Number.isInteger(storeInfo?.shopId) || (storeInfo?.shopId as number) < 0) return;

        redirect({
            // 落点 pages/shop/index（本仓无 pages/store/*，入参名为 shopId；子线 A 收敛）
            url: `/pages/shop/index?shopId=${storeInfo.shopId}`
        });
    };

    // 待拍板 P-2（REPORT-A）：下方 /pages/storeList/index 未注册，本仓无「按商品切换门店」
    // 等价页（pages/shop/list 是店铺街、口径不同），暂保留原落点待裁决，勿臆指其他页面
    const handleStoreList = () => {
        if (!productId) return;

        // region store 已随 B6-6b-1 全量移植（currentRegion / currentLocationInfo 直接可用）；
        // 此处按旧 URL 契约兜底空值。
        const regionStore = useRegionStore.getState();

        // formType 为 true 时（通过带 shopId 的链接直接进入商详），用 redirectTo 替换当前商详页，
        // 让门店列表选店后重新打开商详；不直接操作 history.back，避免 PC iframe 套壳下退出整个 H5 应用导致跳转失效
        redirect({
            url: `/pages/storeList/index?productId=${productId}&regionId=${regionStore.currentRegion?.regionId ?? ""}&latitude=${
                regionStore.currentLocationInfo?.latitude ?? ""
            }&longitude=${regionStore.currentLocationInfo?.longitude ?? ""}&shopId=${shopId ?? ""}`,
            mode: formType ? "redirectTo" : "navigateTo"
        });
    };

    return (
        <View className="shop-info-box" onClick={handleToShop}>
            <View className="shop-info-title">
                <View className="shop-info-title-left">
                    <View className="shop-logo" style={{ backgroundImage: `url(${imageFormat(showStoreInfo.shopLogo)})` }} />
                </View>
                <View className="shop-info-title-right line1">{showStoreInfo.shopTitle}</View>
                <ShopLabelChips labels={storeInfo?.labels} />
            </View>

            <View
                className="shop-info-btn"
                onClick={(e) => {
                    e.stopPropagation();
                    handleStoreList();
                }}
            >
                <View>{formType ? t("进入门店") : t("切换门店")}</View>
                <View className="right-arrows">
                    <Text className="iconfont-h5 icon-gengduo1" style={{ fontSize: "22rpx", color: "#bfbfbf" }} />
                </View>
            </View>
        </View>
    );
}