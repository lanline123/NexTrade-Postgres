/**
* 分类中心V2 预览（移植自 view/admin editor/modules/mobileCateCenter/cateCenterV2/Preview.vue）
*
* 整页 mock：搜索栏 + 左一级菜单 + 顶部二级胶囊页签 + 商品网格（styleType 2）/图文列表（styleType 3）。
* 左菜单与二级页签可点击切换（与原版 @click 行为一致）；商品为原版内置演示数据。
* 海外版仅搜索栏形态不同（返回钮 + 描边框），与原版 isOverseas() 分支一致。
*/
import { useMemo, useState} from "react";
import { cssText} from "../../../shared/format";
import { mergeDefaultModule, formatFrame} from "../../../contract/sharedDefaults";
import { isOverseas} from "../../../contract/version";
import { cateCenterV2Defaults} from "./index";
import "./Preview.css";

interface MockCategory {
id: string;
name: string;
children: Array<{ id: string; name: string}>;
}
interface MockProduct {
id: string;
title: string;
price: string;
cover: string;
}

const ROOT_CATEGORIES: MockCategory[] = [
{ id: "c1", name: "女装", children: [{ id: "c1-1", name: "内衬"}, { id: "c1-2", name: "上装"}, { id: "c1-3", name: "裤装"}, { id: "c1-4", name: "裙装"}]},
{ id: "c2", name: "男装", children: [{ id: "c2-1", name: "上装"}, { id: "c2-2", name: "裤装"}]},
{ id: "c3", name: "内衣", children: [{ id: "c3-1", name: "睡衣"}, { id: "c3-2", name: "家居服"}]},
{ id: "c4", name: "鞋包", children: [{ id: "c4-1", name: "女单鞋"}, { id: "c4-2", name: "女休闲鞋"}, { id: "c4-3", name: "女皮鞋"}]},
{ id: "c5", name: "母婴", children: [{ id: "c5-1", name: "童装"}, { id: "c5-2", name: "玩具"}]},
{ id: "c6", name: "运动", children: [{ id: "c6-1", name: "跑鞋"}, { id: "c6-2", name: "卫衣"}]},
{ id: "c7", name: "美妆", children: [{ id: "c7-1", name: "彩妆"}, { id: "c7-2", name: "护肤"}]},
{ id: "c8", name: "配饰", children: [{ id: "c8-1", name: "首饰"}, { id: "c8-2", name: "包包"}]},
{ id: "c9", name: "家电", children: [{ id: "c9-1", name: "厨房家电"}]},
{ id: "c10", name: "家居", children: [{ id: "c10-1", name: "家纺"}]},
{ id: "c11", name: "家装", children: [{ id: "c11-1", name: "绿植"}]},
{ id: "c12", name: "食品", children: [{ id: "c12-1", name: "零食"}]},
{ id: "c13", name: "国际", children: [{ id: "c13-1", name: "海淘"}]},
{ id: "c14", name: "医药", children: [{ id: "c14-1", name: "常备"}]}
];

const TEXT_FILE_PRODUCTS = [
{ productName: "夏季复古方领刺绣衬衣西裤两件", price: "248.00", picUrl: "https://oss.tigshop.com/img/item/demo/1680762243Myvkn91XZndQaLrMTf!!pic400x400.jpeg"},
{ productName: "法式U领露锁骨短袖T恤女2023年夏季奶系温柔泡泡袖上衣", price: "78.00", picUrl: "https://oss.tigshop.com/img/item/demo/1680589546tPrOZyKK683sfEOCq7!!pic400x400.jpeg"},
{ productName: "陕西正宗奶油亚特猕猴桃新鲜水果奇异果孕妇水果现货绿心猕猴桃 带箱2斤中果70-90g（超值装）", price: "9.90", picUrl: "https://oss.tigshop.com/img/gallery/202509/1756782675WSx4GEiEA1a2TPj5V6.jpeg"},
{ productName: "鲜生美国进口车厘子 黑珍珠大樱桃新鲜当季时令节日水果生鲜礼盒 1000g(净重) 超大果 JJ 28-30mm", price: "168.00", picUrl: "https://oss.tigshop.com/img/gallery/202509/175678163620Qgvoe1xOz7mmmfcn.jpeg"},
{ productName: "23年春季新款气质西服领九分袖搭收腰显瘦纱裙套装", price: "339.00", picUrl: "https://oss.tigshop.com/img/item/demo/16807622759lrB7tSYTaDMl21ver!!pic400x400.jpeg"},
{ productName: "法式西装领两件套纯色西装套裙女春秋复古设计感小众连衣裙两件套", price: "287.00", picUrl: "https://oss.tigshop.com/img/item/demo/1680762215SVhVhwTAK3TCRQdOQC!!pic400x400.jpeg"},
{ productName: "羽绒服秋冬时尚休闲保暖舒适小清新外套上衣女", price: "399.00", picUrl: "https://oss.tigshop.com/img/item/demo/1680762133yqpJm3JLTLInfq4Gb1!!pic400x400.jpeg"},
{ productName: "直筒90绒轻薄女士内胆圆领短款休闲外套", price: "254.00", picUrl: "https://oss.tigshop.com/img/item/demo/168076207428pJgHL7h3bVwGN5fN!!pic400x400.jpeg"},
{ productName: "简洁可脱帽印花内里潮酷中长款保暖女式羽绒服", price: "799.00", picUrl: "https://oss.tigshop.com/img/item/demo/1680762049xrpLXPKPv7LRtNFhO8!!pic400x400.jpeg"},
{ productName: "毛呢外套女春季翻领长袖英伦时尚百搭中长款上衣", price: "374.00", picUrl: "https://oss.tigshop.com/img/item/demo/1680761949huXDrN7BfQHZ6R0G6w!!pic400x400.jpeg"}
];

// 高保真鞋子数据，当点击鞋包类目时展示（与原版 SHOES_PRODUCTS 一致）
const SHOES_PRODUCTS = [
{ productName: "春季新款高帮女鞋时尚女潮板鞋百搭小白鞋女", price: "179.00", picUrl: "https://oss.tigshop.com/img/item/demo/1680589546tPrOZyKK683sfEOCq7!!pic400x400.jpeg"},
{ productName: "韩版厚底高帮鞋潮流爆款熊猫鞋子女2023春季百搭休闲运动...", price: "207.00", picUrl: "https://oss.tigshop.com/img/item/demo/1680762243Myvkn91XZndQaLrMTf!!pic400x400.jpeg"},
{ productName: "清爽透气舒适女鞋双鞋带休闲板鞋耐磨耐穿...", price: "160.00", picUrl: "https://oss.tigshop.com/img/item/demo/16807622759lrB7tSYTaDMl21ver!!pic400x400.jpeg"},
{ productName: "春季新款舒适旅游鞋小白鞋男士板鞋时尚休闲鞋潮流百搭运动...", price: "239.00", picUrl: "https://oss.tigshop.com/img/item/demo/1680762215SVhVhwTAK3TCRQdOQC!!pic400x400.jpeg"},
{ productName: "2023春季新款女鞋子贝壳头小白鞋运动休闲鞋女", price: "149.00", picUrl: "https://oss.tigshop.com/img/item/demo/1680762133yqpJm3JLTLInfq4Gb1!!pic400x400.jpeg"},
{ productName: "春季小白鞋女板鞋舒适运动鞋休闲鞋女", price: "149.00", picUrl: "https://oss.tigshop.com/img/item/demo/168076207428pJgHL7h3bVwGN5fN!!pic400x400.jpeg"}
];

const ALL_SUB_SUFFIX = "-all";
const buildAllSubId = (rootId: string) => `${rootId}${ALL_SUB_SUFFIX}`;

function priceParts(value: string) {
const [integer = "0", decimal = "00"] = String(value).split(".");
return { integer, decimal: `.${decimal.padEnd(2, "0").slice(0, 2)}`};
}

/** 装修颜色变量（与原版 COLOR_VARS 一致） */
const COLOR_VARS: Array<[string, string]> = [
["themeColor", "--dec-cate-theme"],
["sideBgColor", "--dec-cate-side-bg"],
["sideTextColor", "--dec-cate-side-text"],
["sideActiveTextColor", "--dec-cate-side-active-text"],
["sideActiveBgColor", "--dec-cate-side-active-bg"],
["sideIndicatorColor", "--dec-cate-side-indicator"],
["subTabTextColor", "--dec-cate-subtab-text"],
["subTabBgColor", "--dec-cate-subtab-bg"],
["subTabActiveTextColor", "--dec-cate-subtab-active-text"],
["subTabActiveBgColor", "--dec-cate-subtab-active-bg"],
["goodsListBgColor", "--dec-cate-goods-bg"],
["productTitleColor", "--dec-cate-title"],
["priceColor", "--dec-cate-price"],
["buyBtnColor", "--dec-cate-buy-btn"]
];

function SearchIcon({ size = 14}: { size?: number}) {
return (
<svg className="cate-search-icon" viewBox="0 0 1024 1024" width={size} height={size} fill="currentColor" aria-hidden="true">
<path d="M448 768A320 320 0 1 0 448 128a320 320 0 0 0 0 640z m297.6 43.3a448 448 0 1 1 66.7-66.7l194 194a48 48 0 0 1-67.9 67.9l-192.8-195.2z" />
</svg>
);
}

function HomeIcon() {
return (
<svg viewBox="0 0 1024 1024" width="16" height="16" fill="#333" aria-hidden="true">
<path d="M946.5 505L560.1 118.8l-25.9-25.9a31.5 31.5 0 0 0-44.4 0L77.5 505a63.9 63.9 0 0 0-18.8 46c0 35.2 28.7 63.9 63.9 63.9h42.5V940h691.8V614.9h43.4c17 0 33.3-6.7 45.3-18.8a63.6 63.6 0 0 0 18.7-45.3c0-17-6.7-33.1-18.8-45.8zM568 868H456V664h112v204zm217.9-325.7V868H632V640c0-22.1-17.9-40-40-40H432c-22.1 0-40 17.9-40 40v228H238.1V542.3H169a9 9 0 0 1-4.9-15.4L512 179.2l347.9 347.7a9 9 0 0 1-4.9 15.4h-69.1z" />
</svg>
);
}

const CART_PATH = "M401.3 829.9c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m400.4 0c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m211.9-622.1c-11-12.7-27.2-19.9-44.1-19.9H238.9l-17.4-96.8C217.4 55.4 185.5 29 149.5 29H63.4C37.8 29 17 49.8 17 75.4s20.8 46.4 46.4 46.4h86.1c3.2 0 5.9 2.3 6.5 5.4l76.7 425.7c8.5 47.6 49.8 82.1 98.2 82.1h438.3c46.4 0 86.9-33 96.2-78.5l54.7-270.4c3.4-18.1-1.3-36.6-13.5-50.3z";
const PLUS_PATH = "M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64z m213.3 480H544v181.3c0 17.6-14.4 32-32 32s-32-14.4-32-32V544H298.7c-17.6 0-32-14.4-32-32s14.4-32 32-32H480V298.7c0-17.6 14.4-32 32-32s32 14.4 32 32V480h181.3c17.6 0 32 14.4 32 32s-14.4 32-32 32z";

/** 购买按钮（与原版 lyecs-buyBtn 模板一致：1/2 购物车图标、3/4 加号图标、5/6 购买、7/8 马上抢） */
function BuyButton({ style}: { style: number}) {
if (style === 0) return null;
const render = () => {
if (style === 1 || style === 2)
return (
<span className="iconfont-pc iconh5-style">
<svg viewBox="0 0 1024 1024" width="21" height="21" fill="currentColor" aria-hidden="true"><path d={CART_PATH} /></svg>
</span>
);
if (style === 3 || style === 4)
return (
<span className="iconfont-pc iconh5-style">
<svg viewBox="0 0 1024 1024" width="21" height="21" fill="currentColor" aria-hidden="true"><path d={PLUS_PATH} /></svg>
</span>
);
if (style === 5) return <span className="outlineBtnStyle">购买</span>;
if (style === 6) return <span className="activeBtnStyle">购买</span>;
if (style === 7) return <span className="outlineBtnStyle">马上抢</span>;
if (style === 8) return <span className="activeBtnStyle">马上抢</span>;
return null;
};
return (
<div className="item-buy">
<a href="javascript:;" className="buy-btn lyecs-buyBtn">{render()}</a>
</div>
);
}

function Price({ value}: { value: string}) {
const parts = priceParts(value);
return (
<div className="price">
<span className="price-currency">¥</span>
<span className="price-integer">{parts.integer}</span>
<span className="price-decimal">{parts.decimal}</span>
</div>
);
}

export default function CateCenterV2Preview({ data}: { data: Record<string, any>}) {
const module = useMemo(() => {
const merged = JSON.parse(JSON.stringify(data?? {}));
mergeDefaultModule(merged, cateCenterV2Defaults);
return merged;
}, [data]);

const frameFormat = formatFrame(module.frame?? {});
const decorateVars: Record<string, string> = {};
COLOR_VARS.forEach(([field, varName]) => {
const value = module[field];
if (value) decorateVars[varName] = String(value);
});
const rootStyle = {
...cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom),
...decorateVars
} as React.CSSProperties;

const moduleConfig = {
styleType: module.styleType?? 2,
subTabStyle: module.subTabStyle?? 1,
textAlign: module.textAlign?? 1,
textWeight: module.textWeight?? 1,
goodsNameRow: module.goodsNameRow?? 2,
showName: module.showName!== 0,
buyBtnStyle: module.buyBtnStyle?? 1
};
const moduleClasses = [
"module-ad-con",
"dec-cate-center-v2",
`dec-cate-center-v2--subtab-style-${moduleConfig.subTabStyle}`,
`dec-cate-center-v2--text-align-${moduleConfig.textAlign}`,
`dec-cate-center-v2--text-weight-${moduleConfig.textWeight}`,
`dec-cate-center-v2--name-row-${moduleConfig.goodsNameRow}`,
`dec-cate-center-v2--buy-btn-${moduleConfig.buyBtnStyle}`
].join(" ");

const [activeRootId, setActiveRootId] = useState<string>(ROOT_CATEGORIES[0]?.id?? "");
const [activeSubId, setActiveSubId] = useState<string>(buildAllSubId(ROOT_CATEGORIES[0]?.id?? ""));

const subCategories = useMemo(() => {
const root = ROOT_CATEGORIES.find((x) => x.id === activeRootId);
if (!root) return [];
return [{ id: buildAllSubId(root.id), name: "全部商品"},...(root.children?? [])];
}, [activeRootId]);

const productList: MockProduct[] = useMemo(() => {
const pool = activeRootId === "c4"? SHOES_PRODUCTS: TEXT_FILE_PRODUCTS;
return pool.map((item, index) => ({
id: `${activeRootId}_${activeSubId}_${index}`,
title: item.productName,
price: item.price,
cover: item.picUrl
}));
}, [activeRootId, activeSubId]);

const handleRootClick = (id: string) => {
setActiveRootId(id);
setActiveSubId(buildAllSubId(id));
};
const overseas = isOverseas();

return (
<div className={moduleClasses} style={rootStyle}>
{overseas? (
<div className="search-bar-box-overseas">
<div className="back-btn">
<svg viewBox="0 0 1024 1024" width="16" height="16" aria-hidden="true">
<path d="M685.248 104.704a32 32 0 0 1 0 45.248L318.176 512l367.072 362.048a32 32 0 1 1-45.248 45.248l-392-384.64a32 32 0 0 1 0-45.248l392-384.64a32 32 0 0 1 45.248 0z" fill="#333" />
</svg>
</div>
<div className="search-input-wrap">
<span className="search-placeholder">搜索商品</span>
<SearchIcon />
</div>
</div>
): (
<div className="search-bar-box">
<div className="home-btn">
<HomeIcon />
</div>
<div className="search-input-wrap">
<SearchIcon />
<span className="search-placeholder">搜索商品</span>
</div>
</div>
)}

<div className="dec-cate-center-v2__body">
<div className="dec-cate-center-v2__aside">
{ROOT_CATEGORIES.map((cat) => (
<div
key={cat.id}
className={`dec-cate-center-v2__aside-item${activeRootId === cat.id? " is-active": ""}`}
onClick={() => handleRootClick(cat.id)}
>
<span className="dec-cate-center-v2__aside-text">{cat.name}</span>
</div>
))}
</div>

<div className="dec-cate-center-v2__main">
{subCategories.length > 0? (
<div className="dec-cate-center-v2__subtabs-wrap">
<div className="dec-cate-center-v2__subtabs">
{subCategories.map((sub) => (
<div
key={sub.id}
className={`dec-cate-center-v2__subtab${activeSubId === sub.id? " is-active": ""}`}
onClick={() => setActiveSubId(sub.id)}
>
{sub.name}
</div>
))}
</div>
<div className="dec-cate-center-v2__subtabs-arrow">
<svg className="dec-cate-center-v2__subtabs-arrow-icon" viewBox="0 0 1024 1024" width="12" height="12" aria-hidden="true">
<path d="M512 710.4L181.3 379.7c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L512 619.7l285.3-285.3c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3L512 710.4z" fill="#999" />
</svg>
</div>
</div>
): null}

<div className="dec-cate-center-v2__content">
{moduleConfig.styleType!== 3? (
<div className="dec-cate-center-v2__grid">
{productList.map((p) => (
<div className="dec-cate-center-v2__grid-item" key={p.id}>
<div className="pic"><img src={p.cover} alt="" /></div>
{moduleConfig.showName? <div className="title">{p.title}</div>: null}
<div className="action">
<Price value={p.price} />
<BuyButton style={moduleConfig.buyBtnStyle} />
</div>
</div>
))}
</div>
): (
<div className="dec-cate-center-v2__list">
{productList.map((p) => (
<div className="dec-cate-center-v2__list-item" key={p.id}>
<div className="pic"><img src={p.cover} alt="" /></div>
<div className="info">
{moduleConfig.showName? <div className="title">{p.title}</div>: null}
<div className="meta">
<div className="price-box">
<Price value={p.price} />
</div>
<BuyButton style={moduleConfig.buyBtnStyle} />
</div>
</div>
</div>
))}
</div>
)}
</div>
</div>
</div>
</div>
);
}
