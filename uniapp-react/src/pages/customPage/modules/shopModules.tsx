import { Suspense } from "react";
import type { ComponentType } from "react";
import { View } from "@tarojs/components";

import { MODULE_REGISTRY } from "./registry";
import ModuleErrorBoundary from "@/pages/customPage/components/ModuleErrorBoundary";
import type { ShopDetailItem } from "@/types/shop/shop";

/** 当前编辑器可投放的店铺模块，以及历史店铺装修使用的模块。 */
const SHOP_MODULE_TYPES = [
    "article",
    "commonTitle",
    "imageAd2",
    "product2",
    "live",
    "groupon",
    "seckill2",
    "videoNumber",
    "bannerA2",
    "notice",
    "titleAd",
    "imageAd",
    "imageNav",
    "coupon",
    "whiteBlank",
    "whiteLine",
    "imageSquareAd",
    "imageHotarea",
    "shopInfo",
    "product",
    "productA1",
    "limitedSpecial",
    "productSlide",
    "categoryNav",
    "categoryNavA2",
    "banner",
    "slidingImage",
    "slideLargeImage",
    "comboSuggestions",
    "noticeA2",
    "specialOffer",
    "mixedProductDisplay",
    "categoryRec",
    "categoryRecA1",
    "noticeA3",
    "shopTips",
    "countdown",
    "groupSliding",
    "productGrouping",
    "imageTextComponent",
    "imageShowcase",
    "tagNav",
    "productCarousel",
    "categoryNavA1",
    "seckill",
    // storeNav/storeOrderCard：原版 uniapp 两个分发器都没有这两个 type（原版前端从未渲染，
    // 只有编辑器定义）；2026-11-18 补齐批次已按原版编辑器契约重建渲染器，
    // 店铺装修是这两个模块的放置主场，故补进白名单（超出原版的补齐项）。
    "storeNav",
    "storeOrderCard"
];

const SHOP_MODULE_TYPE_SET = new Set(SHOP_MODULE_TYPES);

/** 旧模板里带 :shop-id="props.shopInfo.shopId" 的模块 */
const SHOP_ID_MODULE_TYPES = new Set(["coupon", "product", "product2", "productA1", "limitedSpecial", "productSlide", "productCarousel", "shopInfo", "storeNav", "storeOrderCard"]);

interface Props {
    modules: any[];
    shopInfo: ShopDetailItem;
    /** 对应旧 @refresh-shop-detail（shopInfo 模块收藏后触发，页面重拉店铺详情） */
    onRefreshShopDetail?: (shopId?: number) => void;
    /** 模块渲染失败时显示占位提示块；默认静默空占位（线上店铺页口径） */
    moduleErrorPlaceholder?: boolean;
}

export default function ShopModules({ modules, shopInfo, onRefreshShopDetail, moduleErrorPlaceholder }: Props) {
    return (
        <View>
            {modules.map((mod: any, index: number) => {
                if (mod.isShow === false || !SHOP_MODULE_TYPE_SET.has(mod.type)) return null;
                const Registered = MODULE_REGISTRY[mod.type] as ComponentType<any> | undefined;
                if (!Registered) {
                    console.warn(`[ShopModules] 模块类型 ${mod.type} 尚未迁移`);
                    return null;
                }
                // 旧模板只有 seckill 分支判 isShow，其余类型无 isShow 过滤
                if (mod.type === "seckill" && !mod?.isShow) return null;

                const props: Record<string, any> = {
                    module: mod.module,
                    moduleIndex: mod.moduleIndex,
                    index
                };
                if (SHOP_ID_MODULE_TYPES.has(mod.type)) {
                    props.shopId = shopInfo?.shopId;
                }
                if (mod.type === "shopInfo") {
                    props.shopInfo = shopInfo;
                    props.onRefreshShopDetail = onRefreshShopDetail;
                }
                // 贴附标题条独立分发标记（与 DecorateModules 同口径，2026-11-21）：
                // 店铺页顶层分发的 commonTitle 同为独立模块形态；内嵌直引不经此处。
                if (mod.type === "commonTitle") {
                    props.standaloneTitle = true;
                    if (moduleErrorPlaceholder) props.editorPreview = true;
                }
                return (
                    <ModuleErrorBoundary key={mod.moduleIndex ?? index} moduleType={mod.type} moduleIndex={index} resetKey={mod} placeholder={moduleErrorPlaceholder}>
                        <Suspense fallback={null}>
                            <Registered {...props} />
                        </Suspense>
                    </ModuleErrorBoundary>
                );
            })}
        </View>
    );
}
