import { lazy, Suspense } from "react";
import { View } from "@tarojs/components";

import type { ConfigPageBranch } from "./resolve";

/**
 * 整页配置型页型（登录 5 / 搜索 6 / 商品详情 8）的预览分发（2026-11-19）。
 *
 * 由 decoratePreview 在 resolveConfigPageBranch 命中时渲染（替代通用分发器
 * DecorateModules）：整页只渲染对应真实页面分支，根节点是预览根的唯一子
 * 元素——预览页的布局测量据此把整页映射为配置模块的那一个槽位（见
 * decoratePreview/index.tsx 的 reportLayout 分支口径）。
 *
 * 三个分支均懒加载：分支组件会拉起登录/搜索/商品页的子组件树，pt1/2/3 等
 * 常规页型走通用分发时不付这份体积（与 MODULE_REGISTRY 的 lazy 口径一致）。
 */
const LoginConfigPreview = lazy(() => import(/* webpackChunkName: "preview-login" */ "./LoginConfigPreview"));
const SearchConfigPreview = lazy(() => import(/* webpackChunkName: "preview-search" */ "./SearchConfigPreview"));
const ProductConfigPreview = lazy(() => import(/* webpackChunkName: "preview-product" */ "./ProductConfigPreview"));

export default function ConfigPagePreview({ branch, previewProductId }: { branch: ConfigPageBranch; previewProductId?: number }) {
    return (
        <View className="decorate-preview__config-page">
            <Suspense fallback={null}>
                {branch.pageType === 5 ? (
                    <LoginConfigPreview module={branch.module} />
                ) : branch.pageType === 6 ? (
                    <SearchConfigPreview module={branch.module} />
                ) : branch.pageType === 8 ? (
                    <ProductConfigPreview module={branch.module} type={branch.type} previewProductId={previewProductId} />
                ) : null}
            </Suspense>
        </View>
    );
}
