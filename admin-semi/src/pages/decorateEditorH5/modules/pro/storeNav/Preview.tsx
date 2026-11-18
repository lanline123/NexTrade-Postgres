/**
* 门店导航 预览
*
* 数据层口径（与原版 Preview.vue 的写死演示数据：预览不得以假数据掩盖空配置）：
* - 自动获取（storeSource=1）：按 SelectStorePop 同口径拉 GET /merchant/shop/list
* （shopType=2、status=1、includeSelfOperated=1、page=1、size=storeNum 钳制 1~20），
* 用真实记录渲染门店卡；拉取失败/无记录显示「暂无门店数据」空态。
* - 指定门店（storeSource=2）：渲染 data.storeList（契约仅 { shopId, shopTitle, shopLogo}），
* 为空显示「未选择门店」空态，绝不兜底演示门店。
* - 评分取列表记录 shopStar、地址取 shopDetailedAddress，有真实值才渲染（评分受 showRating 开关控制）；
* 距离 / 起送价 / 配送费在门店列表 VO（ShopVO）中无对应字段（距离需定位、起送/配送属门店配送配置，
* 均不在该接口口径内），恒不渲染——showMinOrder/showDeliveryFee/showDistance 开关保留在设置中但预览无数据可挂。
* 指定门店条目不带评分/地址字段，故该模式下评分行/地址行同样不渲染。
* - 样式二商品行：按店铺真实拉取 GET /product/product/list（shopId + productStatus=1 + checkStatus=1，
* size=4，对齐原版演示行的 4 项版式），无商品的门店不渲染商品行；名称/价格受
* showProductName/productNameRow/showProductPrice 控制。
* 样式结构（storeStyle 1/2 布局、间距/颜色/圆角/开关布局）与 frame/moduleStyle 容器处理对齐原版 Preview.vue；
* 评分标签图 store-tag.png 为原版 admin 本地资源，本工程无此资源文件，预览中略去该图，仅保留评分文案。
*/
import "./Preview.css";
import { useEffect, useMemo, useState} from "react";
import { apiGet } from "../../../../../api";
import { mergeDefaultModule, cloneDeep, formatCommonStyle} from "../../../contract/sharedDefaults";
import { cssText, imageFormat} from "../../../shared/format";
import { storeNavDefaults} from "./index";

interface Props {
data: Record<string, any>;
}

/** 样式二每家门店展示的商品数（版式口径，对齐原版演示行的 4 项） */
const PRODUCT_ROW_SIZE = 4;

/** 预览门店（真实数据归一化结果） */
interface PreviewStore {
shopId: number;
shopTitle: string;
shopLogo?: string;
/** 门店评分（列表记录 shopStar），无有效评分时为空 */
rating?: number;
/** 门店详细地址（列表记录 shopDetailedAddress），无地址时为空 */
address?: string;
}

interface PreviewProduct {
productId?: number;
productName?: string;
productPrice?: number | string;
picThumb?: string;
picUrl?: string;
}

/** 评分字段归一化：非数字、<=0（未评分）一律视为无评分，不渲染 */
const toRating = (value: unknown): number | undefined => {
const num = Number(value);
return Number.isFinite(num) && num > 0? num: undefined;
};

export default function StoreNavPreview({ data}: Props) {
const m = useMemo(() => { const merged = JSON.parse(JSON.stringify(data?? {})); mergeDefaultModule(merged, cloneDeep(storeNavDefaults)); return merged;}, [data]);

const isAssign = Number(m.storeSource) === 2;
const isTwo = Number(m.storeStyle) === 2;
const autoNum = Math.min(Math.max(Math.round(Number(m.storeNum) || 1), 1), 20);

/** 指定门店：直接用装修数据 storeList（契约仅三字段） */
const assignedStores = useMemo<PreviewStore[]>(
() =>
isAssign
? (Array.isArray(m.storeList)? m.storeList: []).map((item: Record<string, any>) => ({
shopId: Number(item.shopId) || 0,
shopTitle: item.shopTitle || "",
shopLogo: item.shopLogo || ""
}))
: [],
[isAssign, m.storeList]
);

/** 自动获取：真实拉取门店列表；null 表示加载中 */
const [autoStores, setAutoStores] = useState<PreviewStore[] | null>(null);
useEffect(() => {
if (isAssign) {
setAutoStores(null);
return;
}
let alive = true;
setAutoStores(null);
apiGet<any>("/merchant/shop/list", {
shopType: 2,
status: 1,
includeSelfOperated: 1,
page: 1,
size: autoNum
})
.then((res: any) => {
if (!alive) return;
const records: any[] = res?.records || res?.list || [];
setAutoStores(
records.map((row: any) => ({
shopId: Number(row.shopId) || 0,
shopTitle: row.shopTitle || row.shopName || "",
shopLogo: row.shopLogo || "",
rating: toRating(row.shopStar),
address: row.shopDetailedAddress || ""
}))
);
})
.catch(() => {
if (alive) setAutoStores([]);
});
return () => {
alive = false;
};
}, [isAssign, autoNum]);

const stores: PreviewStore[] = isAssign? assignedStores: autoStores?? [];
const loading =!isAssign && autoStores === null;

/** 样式二商品行：按门店逐家拉真实商品（shopId 维度），无商品不渲染该行 */
const storeIdsKey = stores.map((store) => store.shopId).filter((id) => id > 0).join(",");
const [productsMap, setProductsMap] = useState<Record<number, PreviewProduct[]>>({});
useEffect(() => {
if (!isTwo ||!storeIdsKey) {
setProductsMap({});
return;
}
let alive = true;
const ids = storeIdsKey.split(",").map((id) => Number(id));
Promise.all(
ids.map((shopId) =>
apiGet<any>("/product/product/list", {
shopId,
productStatus: 1,
checkStatus: 1,
page: 1,
size: PRODUCT_ROW_SIZE
})
.then((res: any) => ({ shopId, products: (res?.records || res?.list || []) as PreviewProduct[]}))
.catch(() => ({ shopId, products: [] as PreviewProduct[]}))
)
).then((entries) => {
if (!alive) return;
const next: Record<number, PreviewProduct[]> = {};
entries.forEach((entry) => {
next[entry.shopId] = entry.products;
});
setProductsMap(next);
});
return () => {
alive = false;
};
}, [isTwo, storeIdsKey]);

const common = formatCommonStyle(m.moduleStyle?? {}, m.contentStyle?? {}, m.subTitle?? {});
const outerStyle = cssText(common.moduleStyle.boxPaddingTopBottom + common.moduleStyle.boxPadding + common.moduleStyle.backgroundColor);
const contentStyle = cssText(
common.moduleContentStyle.backgroundColor +
common.moduleContentStyle.backgroundImage +
common.moduleContentStyle.backgroundSize +
common.moduleContentStyle.boxRadius2 +
common.moduleContentStyle.boxShadow +
common.moduleContentStyle.paddingTopBottom +
common.moduleContentStyle.paddingLeftRight
);
const titleStyle = cssText(`font-size: ${m.subTitle?.titleFontSize}px; color: ${m.subTitle?.titleColor}; font-weight: ${m.subTitle?.titleFontBold};`);
const sts = m.storeTitleStyle?? storeNavDefaults.storeTitleStyle;
const storeTitleStyle: React.CSSProperties = {
fontSize: `${sts.titleFontSize?? 14}px`,
color: sts.titleColor || "#1a1a1a",
fontWeight: sts.titleFontBold?? 700
};
const boxBase = (index: number): React.CSSProperties => ({
backgroundColor: m.storeBackgroundColor || "#ffffff",
padding: `${m.storePaddingY?? 0}px ${m.storePaddingX?? 0}px`,
borderRadius: `${m.storeRadius?? 0}px`,
overflow: "hidden",
...(index > 0? { marginTop: `${m.storeGap?? 15}px`}: {})
});
const storeTitle = (store: PreviewStore) => store.shopTitle || (store.shopId? `门店${store.shopId}`: "");
const emptyText = loading? "加载中…": isAssign? "未选择门店": "暂无门店数据";

return (
<div className="stn-root" style={outerStyle}>
<div style={contentStyle}>
{!isTwo? (
<div className="stn-store-nav-box">
<div className="stn-store-header" style={{ padding: `${m.titlePaddingY?? 0}px ${m.titlePaddingX?? 0}px`}}>
<div className="stn-header-title" style={titleStyle}>
附近门店
</div>
<div className="stn-header-more">
更多<span aria-hidden="true">›</span>
</div>
</div>
<div>
{stores.length === 0? (
<div className="stn-empty">{emptyText}</div>
): (
stores.map((store, index) => (
<div className="stn-stores-item" key={store.shopId || index} style={boxBase(index)}>
<div className="stn-item-left">
{store.shopLogo? <img src={imageFormat(store.shopLogo)} alt="" />: null}
</div>
<div className="stn-item-right">
<div style={storeTitleStyle}>{storeTitle(store)}</div>
{Number(m.showRating) === 1 && store.rating? <div className="stn-store-desc">{store.rating.toFixed(2)}</div>: null}
{store.address? (
<div className="stn-store-address-info">
<div className="stn-address stn-line1">{store.address}</div>
</div>
): null}
</div>
</div>
))
)}
</div>
</div>
): (
<div className="stn-store-card-box">
{stores.length === 0? (
<div className="stn-empty">{emptyText}</div>
): (
stores.map((store, index) => {
const products = productsMap[store.shopId]?? [];
return (
<div className="stn-store-card" key={store.shopId || index} style={boxBase(index)}>
<div className="stn-card-header">
<div className="stn-shop-logo">
{store.shopLogo? <img src={imageFormat(store.shopLogo)} alt="" />: null}
</div>
<div className="stn-shop-main">
<div className="stn-title-row">
<div className="stn-shop-title stn-line1" style={storeTitleStyle}>
{storeTitle(store)}
</div>
{Number(m.showFollow) === 1? (
<div className="stn-collect-btn">
<span className="stn-collect-text">关注</span>
</div>
): null}
</div>
{Number(m.showRating) === 1 && store.rating? (
<div className="stn-meta-row">
<span className="stn-shop-star">{store.rating.toFixed(2)}分</span>
</div>
): null}
</div>
</div>
{products.length > 0? (
<div className="stn-product-scroll">
<div className="stn-product-row" style={{ columnGap: `${m.productGap?? 8}px`}}>
{products.map((product, productIndex) => (
<div className="stn-product-item" key={product.productId?? productIndex}>
<div className="stn-product-image">
{product.picThumb || product.picUrl? <img src={imageFormat(product.picThumb || product.picUrl)} alt="" />: null}
</div>
{Number(m.showProductName) === 1 && product.productName? (
<div className={`stn-product-name ${Number(m.productNameRow) === 2? "stn-line2": "stn-line1"}`}>{product.productName}</div>
): null}
{Number(m.showProductPrice) === 1 && product.productPrice!== undefined && product.productPrice!== null && product.productPrice!== ""? (
<div className="stn-product-price">¥{product.productPrice}</div>
): null}
</div>
))}
</div>
</div>
): null}
</div>
);
})
)}
</div>
)}
</div>
</div>
);
}
