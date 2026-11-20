import { Input, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import TigImage from "@/components/tig/tig-image/TigImage";
import ShopLabelChips from "@/components/shop-label-chips/ShopLabelChips";
import { shopCollection } from "@/api/shop/shop";
import type { ShopDetailItem } from "@/types/shop/shop";
import { imageFormat } from "@/utils/format";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    /**
     * 店铺详情：仅店铺页分发器（ShopModules）与已接通 shopId 的通用分发器会传；
     * 缺失时必须渲染结构空态、绝不抛错（2026-11-19 修复：通用分发器不传时
     * 无守卫直读曾让含 shopInfo 的页面在编辑器真实渲染里整页空白）。
     */
    shopInfo?: ShopDetailItem;
    /** 旧 emit("refreshShopDetail", shopId)：收藏/取消收藏后让页面重拉店铺详情 */
    onRefreshShopDetail?: (shopId: number) => void;
}

/**
 * 店铺信息装修模块（自旧 pages/customPage/modules/shopInfo/index.vue 483 行移植）
 * - 三种风格（module.style）的店铺头图 + 搜索/导航 + 店铺名/商品数/收藏按钮
 * - 收藏态取 shopInfo.collectShop；收藏/取消收藏后回调页面刷新详情
 */
export default function ShopInfoModule({ module, shopInfo, onRefreshShopDetail }: Props) {
    // 守卫口径：shopInfo 完全缺失时用空快照渲染结构空态（店名/计数为空、未收藏）
    const shop = (shopInfo ?? {}) as Partial<ShopDetailItem>;
    const shopId = shop.shopId ?? 0;
    // 旧 computed：背景图列表为 OSS 内置 10 张，backgroundDefault=0 时取自定义图
    const imgList = () => {
        const arr: string[] = [];
        for (let i = 1; i <= 10; i++) {
            arr.push(`https://lyecs2.oss-cn-zhangjiakou.aliyuncs.com/static/mini/bg/${i}.jpg`);
        }
        return arr;
    };

    const backgroundImg = (() => {
        let url: string;
        if (module?.backgroundDefault == 0 && module?.customPic?.picUrl) {
            url = imageFormat(module.customPic.picUrl);
        } else {
            url = imgList()[module?.backgroundDefault - 1];
        }
        return url;
    })();

    const collectText = shop.collectShop ? t("已收藏") : t("收藏");

    const collect = async () => {
        if (!shopId) return;
        try {
            await shopCollection({ shopId });
            Taro.showToast({ title: t("收藏成功") });
            onRefreshShopDetail?.(shopId);
        } catch (error) {
            console.error(error);
        }
    };

    const cancelCollect = async () => {
        if (!shopId) return;
        Taro.showModal({
            title: t("提示"),
            content: t("确认要取消收藏店铺吗？"),
            confirmText: t("确定"),
            cancelText: t("取消"),
            success: async (res) => {
                if (res.confirm) {
                    try {
                        await shopCollection({ shopId });
                        Taro.showToast({ title: t("操作成功") });
                        onRefreshShopDetail?.(shopId);
                    } catch (error) {
                        console.error(error);
                    }
                }
            }
        });
    };

    const handleShopCollection = () => {
        if (!shopId) return;
        if (shop.collectShop) {
            void cancelCollect();
        } else {
            void collect();
        }
    };

    const toSearchTransferPage = () => {
        if (!shopId) return;
        Taro.navigateTo({ url: `/pages/search/index?shopId=${shopId}` });
    };

    const toSearch = () => {
        if (!shopId) return;
        Taro.navigateTo({ url: `/pages/search/result?&shopId=${shopId}` });
    };

    const toCategory = () => {
        if (!shopId) return;
        Taro.navigateTo({ url: `/pages/shop/category?shopId=${shopId}` });
    };

    /** 点击店铺名称进入店铺信息页（无快照时仍可打开，只是没有资质区块） */
    const toShopInfo = () => {
        if (!shopId) return;
        Taro.navigateTo({ url: `/pages/shop/info?shopId=${shopId}` });
    };

    return (
        <View className={`module-ad-con module-store_info ad-store_style__${module?.style}`}>
            {module && (
                <View className="module-ad-content">
                    <View className="cap-store-banner">
                        <View className="cap-store-banner__cover" style={`background-image: url(${backgroundImg});`}>
                            <View className="cap-store-banner__cover-mask" />
                        </View>
                        <View className="cap-store-nav-warp">
                            <View className="cap-store-nav-con">
                                <View className="store-search-con">
                                    <View id="search" className="flex align-center" onClick={toSearchTransferPage}>
                                        <View className="iconfont-h5 icon-sousuo" />
                                        <Input type="text" className="search-input" placeholder={t("搜商品")} />
                                    </View>
                                </View>
                                <View className="store-nav-con">
                                    <View className="store-nav-item active">
                                        <View className="store-nav-item-a">{t("精选")}</View>
                                    </View>
                                    <View className="store-nav-item">
                                        <View className="store-nav-item-a" onClick={toSearch}>
                                            {t("商品")}
                                        </View>
                                    </View>
                                    <View className="store-nav-item">
                                        <View className="store-nav-item-a" onClick={toCategory}>
                                            {t("分类")}
                                        </View>
                                    </View>
                                </View>
                            </View>
                        </View>

                        <View className="cap-store-banner__inner">
                            <View className="cap-store-banner__content">
                                <View className="cap-store-banner__logo">
                                    <TigImage src={shop.shopLogo ?? ""} mode="widthFix" />
                                </View>
                                <View className="cap-store-banner__right-content">
                                    <View className="cap-store-banner__right-content-title--middle" onClick={toShopInfo}>
                                        {shop.shopTitle ?? ""}
                                    </View>
                                    <ShopLabelChips labels={shop.labels} />
                                    <View className="cap-store-banner__sum-content">
                                        <View className="cap-store-banner__sum-content-total">
                                            {t("全部商品")} {shop.productCount ?? 0}
                                        </View>

                                        <View className="cap-store-banner__sum-content-total">
                                            {t("新品")} {shop.newProductCount ?? 0}
                                        </View>
                                    </View>
                                </View>
                                <View className="store-collect-button-con" onClick={handleShopCollection}>
                                    <View className="store-collect-button">{collectText}</View>
                                </View>
                            </View>
                        </View>
                    </View>
                </View>
            )}
        </View>
    );
}
