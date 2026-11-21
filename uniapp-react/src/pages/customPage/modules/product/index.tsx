import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { Image, Swiper, SwiperItem, Text, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import { designPxToDevicePx, designPxToRpx, formatFrame } from "@/components/modules";
import SwiperDots from "@/components/SwiperDots";
import InquiryPrice from "@/components/inquiry-price/InquiryPrice";
import TigImage from "@/components/tig/tig-image/TigImage";
import { getProductsList } from "@/api/common";
import { useConfigStore } from "@/store/config";
import { resolveH5BuyBtnStyle } from "@/utils/buyStyle";
import CommonTitle from "@/pages/customPage/modules/commonTitle/index";
import { useProductStyle } from "@/hooks/useProductStyle";
import { productImage, pickPic, urlFormat } from "@/utils/format";
import { redirect, delay, getElementRect } from "@/utils";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
    shopId?: number;
}

// 对应旧 nextTick：状态更新后的渲染时机
const nextTick = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

export default function Product({ module, shopId = -1 }: Props) {
    const { frame } = module;

    // 购买按钮「跟随全局」解析（读时、不回写）：buyBtnStyle=-1 时取全局样式
    // 降级到 0–8；其余值原样透传，渲染与此前逐字一致。全局 iconColor 非空时
    // 以 --general 局部变量作用于按钮宿主元素（不外溢到整模块的价格等元素）。
    const globalBuyStyle = useConfigStore((s) => s.buyStyle);
    const buyBtn = resolveH5BuyBtnStyle(module.buyBtnStyle, globalBuyStyle);
    const buyBtnVarStyle = buyBtn.iconColor ? ({ "--general": buyBtn.iconColor } as CSSProperties) : undefined;

    const frameFormat = useMemo(() => {
        return formatFrame(frame ?? {});
    }, [frame]);

    const allFormat = useMemo(() => {
        return {
            backgroundColor: `background-color: ${module.backgroundColor};`,
            boxPadding: `margin-left: ${designPxToRpx(module.boxPadding)};margin-right: ${designPxToRpx(module.boxPadding)};`,
            boxPaddingBottom: `margin-bottom: ${designPxToRpx(module.boxPaddingBottom)};`,
            boxPaddingTop: `margin-top: ${designPxToRpx(module.boxPaddingTop)};`,
            boxRadius: `border-radius: ${designPxToRpx(module.boxRadius)};`,
            innerPadding: `padding: ${designPxToRpx(module.innerPadding)};`,
            goodsPadding: `padding: ${designPxToRpx(module.goodsPadding)};`
        };
    }, [module]);

    const paramsValueMap: { [key: number]: string[] } = {
        1: ["productIds"],
        2: ["productCategoryId", "productNumber"],
        3: ["productTag", "productNumber"],
        // 4 全部商品（我方新增，论坛 B 类 237824）：仅 size，无任何过滤
        4: ["productNumber"]
    };
    const paramsKeyMap: { [key: string]: string } = {
        productIds: "ids",
        productCategoryId: "categoryId",
        productNumber: "size",
        productTag: "introType"
    };
    const [productList, setProductList] = useState<any[]>([]);
    const { styleMap } = useProductStyle(productList);

    // ========== 瀑布流（仅"一行两个"且开启瀑布流时生效，按列高分配实现真瀑布流） ==========
    const rootRef = useRef<any>(null);
    const isWaterfall = useMemo(() => Number(module.style) === 2 && Number(module.waterfall) === 1, [module.style, module.waterfall]);
    const [columnList, setColumnList] = useState<any[][]>([[], []]);
    const columnListRef = useRef<any[][]>([[], []]);
    const waterfallTaskIdRef = useRef(0);
    const distributeWaterfall = async (items: any[]) => {
        waterfallTaskIdRef.current++;
        const taskId = waterfallTaskIdRef.current;
        columnListRef.current = [[], []];
        setColumnList([[], []]);
        await nextTick();
        const columnHeights = [0, 0];
        const fallbackHeight = designPxToDevicePx(300);
        for (let i = 0; i < items.length; i++) {
            if (taskId !== waterfallTaskIdRef.current) return;
            let idx: number;
            if (columnHeights[0] === columnHeights[1]) {
                // 高度相等（含首次测高失败两列都为 0）时按已渲染数量交替，避免全堆一列
                idx = columnListRef.current[0].length > columnListRef.current[1].length ? 1 : 0;
            } else {
                idx = columnHeights[0] < columnHeights[1] ? 0 : 1;
            }
            columnListRef.current[idx].push(items[i]);
            setColumnList([columnListRef.current[0], columnListRef.current[1]]);
            await delay(80);
            await nextTick();
            if (taskId !== waterfallTaskIdRef.current) return;
            try {
                const rect: any = await getElementRect(`#wf-col-${idx}`, rootRef.current);
                columnHeights[idx] = rect?.height || columnHeights[idx] + fallbackHeight;
            } catch {
                columnHeights[idx] += fallbackHeight;
            }
        }
    };
    useEffect(() => {
        if (isWaterfall && productList && productList.length) {
            distributeWaterfall(productList);
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [productList, isWaterfall]);

    const _getproductList = async () => {
        let param: any = {};
        if (typeof module.products === "object") {
            if (module.products.productSelectType) {
                const type = module.products.productSelectType;
                const keys = paramsValueMap[type];
                if (type == 1) {
                    // 手选未选商品时 ids 为空串会被请求层剥掉、size=0 被后端 clamp 到 1，
                    // 退化成默认商品并误渲染（2026-11-19 页 152 诊断：发布时 product2 类型
                    // 改写为 product、已发布路径漏出的正是本组件）。与 product2 同口径守卫：
                    // 手选且未选 → 不取数、不渲染；非手选模式不受影响。【我方新增守卫】
                    const productIds = module.products.productIds ?? [];
                    if (!productIds.length) {
                        setProductList([]);
                        return;
                    }
                    param[paramsKeyMap[keys[0]]] = productIds.join(",");
                    param.size = productIds.length;
                } else {
                    param.page = 1;
                    keys.forEach((key) => {
                        param[paramsKeyMap[key]] = module.products[key];
                    });
                    // 全部商品：数量未配时不传 size，走后端默认分页（与共享 hook 同口径）
                    if (type == 4 && !(Number(param.size) > 0)) delete param.size;
                }
            }
        }
        if (shopId && shopId != -1) {
            param.shopId = shopId;
        }
        try {
            const result = await getProductsList(param);
            setProductList(result.records);
        } catch (error) {
            console.log(error);
        }
    };
    useEffect(() => {
        void _getproductList();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    const [swiperList, setSwiperList] = useState<any[]>([]);
    const [swiperBottom, setSwiperBottom] = useState(designPxToDevicePx(15));
    const updateSwiperBottom = () => {
        setSwiperBottom(designPxToDevicePx(15));
    };
    useEffect(() => {
        Taro.onWindowResize(updateSwiperBottom);
        return () => {
            Taro.offWindowResize(updateSwiperBottom);
        };
    }, []);
    const getSwiperList = () => {
        if (module.style == 7 && productList) {
            const list: any[] = [];
            const swiperListNum = Math.ceil(productList.length / 3);
            for (let index = 0; index < swiperListNum; index++) {
                list.push(productList.slice(index * 3, (index + 1) * 3));
            }
            setSwiperList(list);
        } else {
            setSwiperList([]);
        }
    };
    useEffect(() => {
        getSwiperList();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [module.style, productList]);

    const [current, setCurrent] = useState(0);
    const selectColor = useMemo(() => {
        return module.swiperPageColor || "#333";
    }, [module.swiperPageColor]);

    return (
        productList.length > 0 && (
            <View
                ref={rootRef as any}
                className={`module-ad-con decorate-product module-goods_ad ad-style__${module.style} ad-buy_btn_style__${buyBtn.style ?? module.buyBtnStyle} ad-goods_style__${module.goodsStyle} ad-goods_radio_style__${module.goodsRadioStyle} ad-text_align__${module.textAlign} ad-text_weight__${module.textWeight} ad-goods_name_row__${module.goodsNameRow} ad-goods_name_padding__${module.goodsNamePadding} ad-goods-title_style__${module.title?.titleStyle}${
                    isWaterfall ? " ad-waterfall__1" : ""
                }`}
                style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
            >
                <View
                    className={`module-ad-content ad-goods-waterfall-${module.waterfall}`}
                    style={
                        frameFormat.backgroundColor +
                        " " +
                        frameFormat.boxRadius +
                        allFormat.backgroundColor +
                        allFormat.boxPadding +
                        allFormat.boxPaddingBottom +
                        allFormat.boxPaddingTop +
                        allFormat.boxRadius +
                        allFormat.innerPadding
                    }
                >
                    {module.title.showTitle == 1 && <CommonTitle module={module.title} />}
                    {productList.length > 0 && (
                        <>
                            {module.style == 7 ? (
                                <View className="goods-ad-warp">
                                    <View className="goods-ad-con">
                                        <View
                                            className={`goods-ad-swiper ${module.picPageType == 3 ? "uni-swiper-dot-box" : ""}`}
                                            // 旧 tig-swiper 等价展开：height=480rpx、circular、autoplay 默认 true、interval 3000；
                                            // style 对象里的 rpx 在 H5 被丢弃（同 imageAd 纯白案），按设计 px 口径换算落 px
                                            style={{ height: designPxToDevicePx(240) + "px" }}
                                        >
                                            <Swiper
                                                circular
                                                indicatorDots={false}
                                                autoplay
                                                interval={3000}
                                                duration={500}
                                                style={{ height: "100%" }}
                                                onChange={(e: any) => setCurrent(e.detail.current)}
                                            >
                                                {swiperList.map((item, key) => (
                                                    <SwiperItem key={key}>
                                                        {item.map((subItem: any, index: number) => (
                                                            <View key={index} className="goods-ad-item">
                                                                <View className="item-content" style={allFormat.goodsPadding}>
                                                                    <View className="item-con">
                                                                        <View className="item-photo">
                                                                            <View
                                                                                className="item-image-a"
                                                                                onClick={() =>
                                                                                    redirect({
                                                                                        url: urlFormat({ path: "product", data: { id: subItem.productId } })
                                                                                    })
                                                                                }
                                                                            >
                                                                                <Image src={productImage(subItem)} mode="widthFix" />
                                                                            </View>
                                                                            {styleMap[subItem.productId]?.borderImage && (
                                                                                <Image
                                                                                    className="product-style-border"
                                                                                    src={styleMap[subItem.productId].borderImage}
                                                                                    mode="scaleToFill"
                                                                                />
                                                                            )}
                                                                        </View>
                                                                        <View className="item-info">
                                                                            {Number(module.showName) === 1 && (
                                                                                <View className="item-name">
                                                                                    <View
                                                                                        className="item-name-a"
                                                                                        onClick={() =>
                                                                                            redirect({
                                                                                                url: urlFormat({ path: "product", data: { id: subItem.productId } })
                                                                                            })
                                                                                        }
                                                                                    >
                                                                                        <Text>{subItem.productName ?? ""}</Text>
                                                                                    </View>
                                                                                    {Number(module.showBrief) === 1 && (
                                                                                        <View
                                                                                            className="item-brief"
                                                                                            onClick={() =>
                                                                                                redirect({
                                                                                                    url: urlFormat({ path: "product", data: { id: subItem.productId } })
                                                                                                })
                                                                                            }
                                                                                        >
                                                                                            {t(subItem.productBrief ?? "商品描述")}
                                                                                        </View>
                                                                                    )}
                                                                                </View>
                                                                            )}
                                                                            {Number(module?.showPrice) === 1 && (
                                                                                <View className="item-action">
                                                                                    <View className="item-price">
                                                                                        <InquiryPrice
                                                                                            product={subItem}
                                                                                            fontStyle={{ fontSize: "32rpx" }}
                                                                                            currencyStyle={{
                                                                                                fontSize: "22rpx"
                                                                                            }}
                                                                                            decimalsStyle={{
                                                                                                fontSize: "24rpx"
                                                                                            }}
                                                                                            priceData={
                                                                                                subItem.deliveryMode === 2 ? subItem.cyclePrice : subItem.productPrice
                                                                                            }
                                                                                        />
                                                                                    </View>
                                                                                    <View className="item-buy" style={buyBtnVarStyle} />
                                                                                </View>
                                                                            )}
                                                                        </View>
                                                                    </View>
                                                                </View>
                                                            </View>
                                                        ))}
                                                    </SwiperItem>
                                                ))}
                                            </Swiper>
                                            {/* 原 tig-swiper mode="default" 恒有圆点指示（picPageType==3 时另加胶囊底盒 + dot 色 #fff）；
                                                此前自绘横条仅 picPageType==3 渲染且选中色不对，现统一走 SwiperDots 三式 */}
                                            <SwiperDots
                                                count={swiperList.length}
                                                current={current}
                                                picPageType={module.picPageType}
                                                color={module.picPageType == 3 ? "#fff" : ""}
                                                selectColor={selectColor}
                                                bottom={swiperBottom}
                                            />
                                        </View>
                                    </View>
                                </View>
                            ) : isWaterfall ? (
                                <View className="goods-ad-warp">
                                    <View className="goods-ad-con">
                                        {columnList.map((col, colIndex) => (
                                            <View id={`wf-col-${colIndex}`} key={`wf-col-${colIndex}`} className="goods-ad-col">
                                                {col.map((item: any) => (
                                                    <View key={item.productId} className="goods-ad-item">
                                                        <View className="item-content" style={allFormat.goodsPadding}>
                                                            <View className="item-con">
                                                                <View className="item-photo">
                                                                    <View
                                                                        className="item-image-a"
                                                                        onClick={() =>
                                                                            redirect({ url: urlFormat({ path: "product", data: { id: item.productId } }) })
                                                                        }
                                                                    >
                                                                        <TigImage src={pickPic(item)} mode="widthFix" />
                                                                    </View>
                                                                    {styleMap[item.productId]?.borderImage && (
                                                                        <Image
                                                                            className="product-style-border"
                                                                            src={styleMap[item.productId].borderImage}
                                                                            mode="scaleToFill"
                                                                        />
                                                                    )}
                                                                </View>
                                                                <View className="item-info">
                                                                    {Number(module.showName) === 1 && (
                                                                        <View className="item-name">
                                                                            <View
                                                                                className="item-name-a"
                                                                                onClick={() =>
                                                                                    redirect({ url: urlFormat({ path: "product", data: { id: item.productId } }) })
                                                                                }
                                                                            >
                                                                                <Text> {t(item.productName ?? "商品名称")} </Text>
                                                                            </View>
                                                                        </View>
                                                                    )}
                                                                    {module.showPrice ? (
                                                                        <View className="item-action">
                                                                            <View className="item-price">
                                                                                <InquiryPrice
                                                                                    product={item}
                                                                                    fontStyle={{ fontSize: "32rpx" }}
                                                                                    currencyStyle={{ fontSize: "22rpx", position: "relative", top: "-3rpx" }}
                                                                                    priceData={item.deliveryMode === 2 ? item.cyclePrice : item.productPrice}
                                                                                />
                                                                            </View>
                                                                            <View
                                                                                className="item-buy"
                                                                                style={buyBtnVarStyle}
                                                                                onClick={() => redirect({ url: `/pages/product/index?id=${item.productId}` })}
                                                                            >
                                                                                <View className="buy-btn">
                                                                                    {buyBtn.style == 5 || buyBtn.style == 6 ? (
                                                                                        <View>{t("购买")}</View>
                                                                                    ) : buyBtn.style == 7 || buyBtn.style == 8 ? (
                                                                                        <View>{t("马上抢")}</View>
                                                                                    ) : (
                                                                                        <View className="iconfont-h5" />
                                                                                    )}
                                                                                </View>
                                                                            </View>
                                                                        </View>
                                                                    ) : (
                                                                        <View className="blank" style={{ height: "20rpx" }} />
                                                                    )}
                                                                </View>
                                                            </View>
                                                        </View>
                                                    </View>
                                                ))}
                                            </View>
                                        ))}
                                    </View>
                                </View>
                            ) : (
                                <View className="goods-ad-warp">
                                    <View className="goods-ad-con">
                                        {productList.map((item: any) => (
                                            <View key={item.productId} className="goods-ad-item">
                                                <View className="item-content" style={allFormat.goodsPadding}>
                                                    <View className="item-con">
                                                        <View className="item-photo">
                                                            <View
                                                                className="item-image-a"
                                                                onClick={() => redirect({ url: urlFormat({ path: "product", data: { id: item.productId } }) })}
                                                            >
                                                                <TigImage src={pickPic(item)} mode="widthFix" />
                                                            </View>
                                                            {styleMap[item.productId]?.borderImage && (
                                                                <Image
                                                                    className="product-style-border"
                                                                    src={styleMap[item.productId].borderImage}
                                                                    mode="scaleToFill"
                                                                />
                                                            )}
                                                        </View>
                                                        <View className="item-info">
                                                            {Number(module.showName) === 1 && (
                                                                <View className="item-name">
                                                                    <View
                                                                        className="item-name-a"
                                                                        onClick={() =>
                                                                            redirect({ url: urlFormat({ path: "product", data: { id: item.productId } }) })
                                                                        }
                                                                    >
                                                                        <Text> {t(item.productName ?? "商品名称")} </Text>
                                                                    </View>
                                                                </View>
                                                            )}
                                                            {module.showPrice ? (
                                                                <View className="item-action">
                                                                    <View className="item-price">
                                                                        <InquiryPrice
                                                                            product={item}
                                                                            fontStyle={{ fontSize: "32rpx" }}
                                                                            currencyStyle={{ fontSize: "22rpx", position: "relative", top: "-3rpx" }}
                                                                            priceData={item.deliveryMode === 2 ? item.cyclePrice : item.productPrice}
                                                                        />
                                                                    </View>
                                                                    <View
                                                                        className="item-buy"
                                                                                style={buyBtnVarStyle}
                                                                        onClick={() => redirect({ url: `/pages/product/index?id=${item.productId}` })}
                                                                    >
                                                                        <View className="buy-btn">
                                                                            {buyBtn.style == 5 || buyBtn.style == 6 ? (
                                                                                <View>{t("购买")}</View>
                                                                            ) : buyBtn.style == 7 || buyBtn.style == 8 ? (
                                                                                <View>{t("马上抢")}</View>
                                                                            ) : (
                                                                                <View className="iconfont-h5" />
                                                                            )}
                                                                        </View>
                                                                    </View>
                                                                </View>
                                                            ) : (
                                                                <View className="blank" style={{ height: "20rpx" }} />
                                                            )}
                                                        </View>
                                                    </View>
                                                </View>
                                            </View>
                                        ))}
                                    </View>
                                </View>
                            )}
                        </>
                    )}
                </View>
            </View>
        )
    );
}
