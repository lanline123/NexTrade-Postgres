/**
 * 店铺信息 预览（DOM 类名逐字对齐 uniapp-react 渲染端 shopInfo/index.tsx）
 *
 * 店铺数据在渲染端由页面传入 shopInfo prop，编辑器预览用演示店铺兜底；
 * 演示字段与渲染端使用一致：shopLogo、shopTitle、productCount、newProductCount、collectShop。
 * 背景图规则与渲染端一致：backgroundDefault=0 且 customPic 有图时用自定义图，
 * 否则取 OSS 内置 10 张背景图（序号 backgroundDefault-1）。
 */
import { useMemo } from "react";
import { imageFormat } from "../../../shared/format";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { shopInfoDefaults } from "./index";
import "./Preview.css";

/** 演示店铺（仅预览兜底） */
const DEMO_SHOP = {
    shopLogo: "https://oss.tigshop.com/img/gallery/202411/17319190474y0XWFxO8GJZF0dg64.jpg",
    shopTitle: "演示店铺",
    productCount: 128,
    newProductCount: 12,
    collectShop: false
};

function builtinBackground(index: number): string {
    const list: string[] = [];
    for (let i = 1; i <= 10; i++) {
        list.push(`https://lyecs2.oss-cn-zhangjiakou.aliyuncs.com/static/mini/bg/${i}.jpg`);
    }
    return list[index] || list[0];
}

export default function ShopInfoPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, shopInfoDefaults);
        return merged;
    }, [data]);

    const backgroundImg =
        Number(module.backgroundDefault) === 0 && module.customPic?.picUrl
            ? imageFormat(module.customPic.picUrl)
            : builtinBackground(Number(module.backgroundDefault || 1) - 1);
    const shopInfo = DEMO_SHOP;

    return (
        <div className={`module-ad-con module-store_info ad-store_style__${module.style}`}>
            <div className="module-ad-content">
                <div className="cap-store-banner">
                    <div className="cap-store-banner__cover" style={{ backgroundImage: `url(${backgroundImg})` }}>
                        <div className="cap-store-banner__cover-mask" />
                    </div>
                    <div className="cap-store-nav-warp">
                        <div className="cap-store-nav-con">
                            <div className="store-search-con">
                                <div className="flex align-center">
                                    <span className="iconfont-h5 icon-sousuo" />
                                    <input type="text" className="search-input" placeholder="搜商品" readOnly />
                                </div>
                            </div>
                            <div className="store-nav-con">
                                <div className="store-nav-item active">
                                    <div className="store-nav-item-a">精选</div>
                                </div>
                                <div className="store-nav-item">
                                    <div className="store-nav-item-a">商品</div>
                                </div>
                                <div className="store-nav-item">
                                    <div className="store-nav-item-a">分类</div>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div className="cap-store-banner__inner">
                        <div className="cap-store-banner__content">
                            <div className="cap-store-banner__logo">
                                <img src={imageFormat(shopInfo.shopLogo)} alt="" />
                            </div>
                            <div className="cap-store-banner__right-content">
                                <div className="cap-store-banner__right-content-title--middle">{shopInfo.shopTitle}</div>
                                <div className="cap-store-banner__sum-content">
                                    <div className="cap-store-banner__sum-content-total">全部商品 {shopInfo.productCount}</div>
                                    <div className="cap-store-banner__sum-content-total">新品 {shopInfo.newProductCount}</div>
                                </div>
                            </div>
                            <div className="store-collect-button-con">
                                <div className="store-collect-button">{shopInfo.collectShop ? "已收藏" : "收藏"}</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
