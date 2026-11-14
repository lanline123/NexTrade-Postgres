import { useMemo } from "react";
import { Image, Text, View } from "@tarojs/components";

import { formatFrame } from "@/components/modules";
import { redirect } from "@/utils";
import { imageFormat, urlFormat } from "@/utils/format";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
    shopId?: number;
}

/**
 * 店铺导航装修模块（type=storeNav，本线新增渲染入口）。
 * 注意：admin 侧该 type 没有源契约——精确版导入被注释，现注册的是
 * generic 占位 defaults {isShow, title:'店铺导航', showCount:4} + picList
 * 字段，线2 已将其 visible:false（存量可打开、新建不可见）。
 * 本入口按该字段集 + 图文导航通用结构渲染：picList 有数据时按
 * imageNav 同款（picUrl/picTitle/picLink）渲染；无 picList 时回落到
 * 店铺页既有导航语义（精选/全部商品/分类/店内搜索，跳 shop 页，
 * 与 shopInfo 模块内 .store-nav-con 的硬编码入口一致）。
 */
export default function StoreNav({ module, shopId = -1 }: Props) {
    const d = useMemo(() => {
        const data = module?.data && typeof module.data === "object" ? module.data : {};
        return { ...module, ...data };
    }, [module]);

    const frameFormat = useMemo(() => formatFrame(d.frame ?? {}), [d]);
    const showCount = Number(d.showCount ?? 4) || 4;

    const fallbackItems = useMemo(() => {
        const sid = shopId > 0 ? shopId : d.shopId || "";
        const q = sid ? `?shopId=${sid}` : "";
        return [
            { picTitle: t("精选"), picLink: `/pages/shop/index${q}` },
            { picTitle: t("全部商品"), picLink: `/pages/shop/list${q}` },
            { picTitle: t("分类"), picLink: `/pages/shop/category${q}` },
            { picTitle: t("店内搜索"), picLink: `/pages/search/result${sid ? `?shopId=${sid}` : ""}` }
        ];
    }, [shopId, d.shopId]);

    const picList: any[] = Array.isArray(d.picList) && d.picList.length ? d.picList : fallbackItems;
    const items = picList.slice(0, showCount);
    if (!items.length) return null;

    return (
        <View
            className="module-ad-con module-store-nav"
            style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
        >
            <View className="module-ad-content" style={frameFormat.innerPadding + frameFormat.backgroundColor + frameFormat.boxRadius}>
                {d.title ? <View className="store-nav__title">{t(d.title)}</View> : null}
                <View className="store-nav__row">
                    {items.map((item, i) => (
                        <View
                            key={i}
                            className="store-nav__item"
                            onClick={() => {
                                const link = typeof item.picLink === "string" ? item.picLink : item.picLink?.link || item.picLink?.url;
                                if (link) redirect({ url: urlFormat(link, item.picTitle) });
                            }}
                        >
                            {item.picUrl ? <Image className="store-nav__icon" src={imageFormat(item.picUrl?.picUrl || item.picUrl)} mode="aspectFill" /> : null}
                            <Text className="store-nav__text">{t(item.picTitle || "")}</Text>
                        </View>
                    ))}
                </View>
            </View>
        </View>
    );
}
