import { Suspense, useRef, useState, type ReactNode } from "react";
import { View, Image, Swiper, SwiperItem } from "@tarojs/components";

import { MODULE_REGISTRY, STICKY_REGISTRY, type DecorateModuleProps } from "@/pages/customPage/modules/registry";
import type { ShopDetailItem } from "@/types/shop/shop";
import { redirect, getElementRect } from "@/utils";
import { imageFormat } from "@/utils/format";

import ModuleErrorBoundary from "./ModuleErrorBoundary";
import EditorEmptyShell from "./EditorEmptyShell";
import "./DecorateModules.scss";

/** 与 shopModules.tsx 的 SHOP_ID_MODULE_TYPES 保持一致（原版店铺页分发口径：这些模块带 shopId） */
const SHOP_ID_MODULE_TYPES = new Set(["coupon", "product", "product2", "productA1", "limitedSpecial", "productSlide", "productCarousel", "shopInfo", "storeNav"]);

interface CatNavData {
    id: number;
    categoryId: number;
    showCatNav: number;
    catColor: string;
    childCatInfo: any[];
    brandInfo: any[];
    imgUrl: any[];
}

interface Props {
    modules: any[];
    pageModles: any;
    scrollTop: number;
    onLoadGoodsList?: (categoryId: number) => void;
    /**
     * 店铺上下文（可选）：由店铺相关页面/编辑器嵌入预览传入，镜像 ShopModules
     * 口径分发给 SHOP_ID 类模块（shopId）与 shopInfo 模块（shopInfo+刷新回调）。
     * 首页/自定义页不传时行为与此前完全一致。
     */
    shopInfo?: ShopDetailItem;
    shopId?: number;
    onRefreshShopDetail?: (shopId?: number) => void;
    /** 模块渲染失败时显示占位提示块（编辑器嵌入预览用）；默认静默空占位（线上口径） */
    moduleErrorPlaceholder?: boolean;
    /**
     * 模块无内容时显示编辑占位块（编辑器嵌入预览用）：空模块前台口径渲染为
     * 0 高，编辑器里不可见不可点；开启后由 EditorEmptyShell 量测内容高度，
     * 为空时在槽位内补可见占位。默认关闭（线上口径，DOM 不变）。
     */
    moduleEmptyPlaceholder?: boolean;
}

/**
 * 装修模块分发器（自旧 pages/customPage/modules/index.vue 移植）：
 * - navbar（页面标题装修）+ 按 moduleIndex 顺序渲染已注册模块
 * - catNav 选中分类后的二级展示（banner 轮播 / 子分类 / 品牌）
 */
export default function DecorateModules({ modules, pageModles, scrollTop, onLoadGoodsList, shopInfo, shopId, onRefreshShopDetail, moduleErrorPlaceholder, moduleEmptyPlaceholder }: Props) {
    /**
     * 编辑器空态壳：仅嵌入预览开启时包裹槽位内容（量测为空补占位高度）；
     * 关闭时原样返回节点，前台 DOM 逐字不变。壳恒为槽位内的单根节点，
     * decoratePreview 按容器 children 下标对齐槽位的契约不受影响。
     */
    const wrapSlot = (type: string, resetKey: unknown, node: ReactNode, key?: string | number): ReactNode =>
        moduleEmptyPlaceholder ? (
            <EditorEmptyShell key={key} moduleType={type} resetKey={resetKey}>
                {node}
            </EditorEmptyShell>
        ) : (
            node
        );
    const effectiveShopId = shopId ?? shopInfo?.shopId;
    /** 首个可见模块下标（会员页 MemberDecorate 同口径：isShow !== false 即可见，供 userInfo3 沉浸式判定） */
    const firstVisibleModuleIndex = (modules ?? []).findIndex((m: any) => m?.isShow !== false);
    const [showCatNav, setShowCatNav] = useState(0);
    const [show, setShow] = useState(false);
    const catIdRef = useRef(0);
    const [childCatInfoData, setChildCatInfoData] = useState<any[]>([]);
    const [brandInfoData, setBrandInfoData] = useState<any[]>([]);
    const [imgUrlData, setImgUrlData] = useState<any[]>([]);
    const [currentColor, setCurrentColor] = useState("");
    const [swiperHeight, setSwiperHeight] = useState(0);
    const instanceRef = useRef<any>(null);

    const onChangeCatNav = (data: CatNavData) => {
        const { categoryId, catColor, imgUrl, childCatInfo, brandInfo } = data;
        setChildCatInfoData(childCatInfo);
        setBrandInfoData(brandInfo);
        setImgUrlData(imgUrl);
        setCurrentColor(catColor);
        if (categoryId === catIdRef.current) return;
        catIdRef.current = categoryId; //  记录当前分类id
        // 对应旧 nextTick：状态批量更新后回调父级
        setTimeout(() => {
            if (catIdRef.current > 0) {
                setShowCatNav(1);
            } else {
                setShowCatNav(0);
            }
            setShow(true);
            onLoadGoodsList?.(categoryId);
        }, 0);
    };

    const computeImgHeight = () => {
        setTimeout(async () => {
            const res = await getElementRect(".swiper-image", instanceRef.current);
            setSwiperHeight(res?.height ?? 0);
        }, 0);
    };

    return (
        <View ref={instanceRef as any}>
            {pageModles && pageModles.title && pageModles.title !== "" && (
                <ModuleErrorBoundary moduleType="navbar" resetKey={pageModles} placeholder={moduleErrorPlaceholder}>
                    <ModuleById type="navbar" module={pageModles} />
                </ModuleErrorBoundary>
            )}

            {modules.map((mod: any, index: number) => {
                // navbar（页面导航）不是列表模块：原版分发器（旧 modules/index.vue）仅在
                // 列表之前以 pageModles 渲染页面级标题栏，模块循环内无 navbar 分支。
                // 列表中的 navbar 条目一律不渲染（标题栏配置走页面设置 pageModule）。
                if (mod.type === "navbar") return null;
                if (mod.type === "searchBar" && mod?.isShow) {
                    return wrapSlot(
                        mod.type,
                        mod,
                        <ModuleErrorBoundary key={mod.moduleIndex} moduleType={mod.type} moduleIndex={index} resetKey={mod} placeholder={moduleErrorPlaceholder}>
                            <ModuleById type="searchBar" module={mod.module} />
                        </ModuleErrorBoundary>,
                        mod.moduleIndex
                    );
                }
                if (mod.type === "searchBar2" && mod?.isShow) {
                    return wrapSlot(
                        mod.type,
                        mod,
                        <ModuleErrorBoundary key={mod.moduleIndex} moduleType={mod.type} moduleIndex={index} resetKey={mod} placeholder={moduleErrorPlaceholder}>
                            <ModuleById type="searchBar2" module={mod.module} />
                        </ModuleErrorBoundary>,
                        mod.moduleIndex
                    );
                }
                if (mod.type === "catNav" && mod?.isShow) {
                    return wrapSlot(
                        mod.type,
                        mod,
                        <ModuleErrorBoundary key={mod.moduleIndex} moduleType={mod.type} moduleIndex={index} resetKey={mod} placeholder={moduleErrorPlaceholder}>
                            <View>
                                <ModuleById
                                    type="catNav"
                                    module={mod.module}
                                    scrollTop={scrollTop}
                                    onChangeCatNav={onChangeCatNav}
                                    onIsCut={setShow}
                                />
                                <ModuleById
                                    type="catNavSticky"
                                    module={mod.module}
                                    scrollTop={scrollTop}
                                    onChangeCatNav={onChangeCatNav}
                                    onIsCut={setShow}
                                />
                            </View>
                        </ModuleErrorBoundary>,
                        mod.moduleIndex
                    );
                }

                const Registered = MODULE_REGISTRY[mod.type];
                if (Registered && mod?.isShow && showCatNav === 0) {
                    const props: DecorateModuleProps = { module: mod.module, moduleIndex: mod.moduleIndex, index };
                    if (mod.type === "bannerA2") {
                        props.needStatusBar = !(pageModles && pageModles.title !== "");
                    }
                    // 店铺上下文分发（镜像 ShopModules 的原版口径）：SHOP_ID 类模块带
                    // shopId，shopInfo 模块另带 shopInfo 与刷新回调；未传上下文时不带，
                    // 各模块按自身守卫处理（shopInfo 已全面守卫，缺失渲染空态）。
                    if (SHOP_ID_MODULE_TYPES.has(mod.type) && effectiveShopId) {
                        props.shopId = effectiveShopId;
                    }
                    if (mod.type === "shopInfo" && shopInfo) {
                        props.shopInfo = shopInfo;
                        props.onRefreshShopDetail = onRefreshShopDetail;
                    }
                    // 会员页沉浸式分发（镜像 MemberDecorate 口径）：userInfo3 仅在
                    // pageModule.immersive 为真且本模块是首个可见模块时沉浸，
                    // 其渐变背景才延伸到页面顶部；其余情况 immersive=false。
                    if (mod.type === "userInfo3" || mod.type === "user-info3") {
                        props.immersive = !!(pageModles && pageModles.immersive) && index === firstVisibleModuleIndex;
                    }
                    return (
                        <View key={mod.moduleIndex} className={show ? "show" : ""}>
                            {wrapSlot(
                                mod.type,
                                mod,
                                <ModuleErrorBoundary moduleType={mod.type} moduleIndex={index} resetKey={mod} placeholder={moduleErrorPlaceholder}>
                                    <Suspense fallback={null}>
                                        <Registered {...props} />
                                    </Suspense>
                                </ModuleErrorBoundary>
                            )}
                        </View>
                    );
                }
                if (mod?.isShow && !Registered && showCatNav === 0) {
                    console.warn(`[DecorateModules] 模块类型 ${mod.type} 尚未迁移（TODO B1-B6）`);
                }
                return null;
            })}

            {showCatNav === 1 && (
                <View className="decorate-category-panel">
                    {imgUrlData.length > 0 && (
                        <View className="bannerBox" style={`background: ${currentColor};`}>
                            <View className={`bannerWrap ${show ? "show" : ""}`}>
                                <Swiper
                                    current={0}
                                    indicatorDots={false}
                                    autoplay
                                    circular
                                    interval={3000}
                                    duration={500}
                                    indicatorColor="rgba(255,255,255,0.6)"
                                    indicatorActiveColor="#fff"
                                    style={{ height: swiperHeight + "px" }}
                                >
                                    {imgUrlData.map((item: any, index: number) => (
                                        <SwiperItem key={index}>
                                            <View className="swiper-slide">
                                                <Image
                                                    className="swiper-image"
                                                    mode="widthFix"
                                                    src={imageFormat(item.picUrl)}
                                                    onLoad={computeImgHeight}
                                                />
                                            </View>
                                        </SwiperItem>
                                    ))}
                                </Swiper>
                            </View>
                        </View>
                    )}

                    {childCatInfoData.length > 0 && (
                        <View className="categoryBox" style={`background: ${currentColor};`}>
                            <View className={`categoryWrap ${show ? "show" : ""}`}>
                                {childCatInfoData.map((item: any, index: number) =>
                                    index < 10 ? (
                                        <View
                                            key={index}
                                            className="item"
                                            onClick={() => redirect({ url: "/pages/search/result", param: { categoryId: item.categoryId } })}
                                        >
                                            <View className="item-img">
                                                <Image src={item.categoryPic} mode="widthFix" />
                                            </View>
                                            <View className="txt">{item.categoryName}</View>
                                        </View>
                                    ) : null
                                )}
                            </View>
                        </View>
                    )}

                    {brandInfoData.length > 0 && (
                        <View className="brandBox">
                            <View className={`brandWrap ${show ? "show" : ""}`}>
                                {brandInfoData.map((brand: any, index: number) =>
                                    index < 8 ? (
                                        <View key={index} className="item">
                                            <View
                                                className="itemWrap"
                                                onClick={() => redirect({ url: "/pages/search/result", param: { brandId: brand.brandId } })}
                                            >
                                                <Image src={brand.brandLogo} mode="widthFix" />
                                                <View className="txt">{brand.brandName}</View>
                                            </View>
                                        </View>
                                    ) : null
                                )}
                            </View>
                        </View>
                    )}
                </View>
            )}
        </View>
    );
}

/** 从注册表取模块；navbar/catNavSticky 等特殊 key 走各自注册表 */
function ModuleById({ type, ...props }: { type: string } & DecorateModuleProps) {
    const Cmp = type === "catNavSticky" ? STICKY_REGISTRY["catNav"] : MODULE_REGISTRY[type];
    if (!Cmp) {
        console.warn(`[DecorateModules] 模块类型 ${type} 尚未迁移（TODO B1-B6）`);
        return null;
    }
    return (
        <Suspense fallback={null}>
            <Cmp {...props} />
        </Suspense>
    );
}
