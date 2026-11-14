/**
 * 分类中心V1 预览（移植自 view/admin editor/modules/mobileCateCenter/cateCenterV1/Preview.vue）
 *
 * 整页 mock：高 700px（海外 820px）、白底、flex 纵向（搜索栏 + 左菜单/右内容）。
 * 海外版走 ViewOverseas 分块布局（isOverseas() 切换，与原版一致）；
 * 原版模板中左侧菜单无点击处理，默认选中「家居」(id=10) 展示其子分类，此处静态复刻。
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { isOverseas } from "../../../contract/version";
import { cateCenterV1Defaults } from "./index";
import "./Preview.css";

const BASE_URL = "https://oss.tigshop.com/";

interface CategoryItem {
    id: string;
    categoryName: string;
    catShortName?: string;
    categoryPic?: string;
    children?: CategoryItem[];
}

const HOME_CHILDREN: CategoryItem[] = [
    {
        id: "10-1",
        categoryName: "家纺床品",
        children: [
            { id: "21-home", categoryName: "床上套件", categoryPic: BASE_URL + "img/gallery/demo/16800815487NuMIvmCERTjSOwgXQ!!pic.jpeg" },
            { id: "23-home", categoryName: "被芯", categoryPic: BASE_URL + "img/gallery/demo/16800815488spO1YEwiLHQGaxDOe!!pic.jpeg" },
            { id: "22-home", categoryName: "电热毯", categoryPic: BASE_URL + "img/gallery/demo/1680081548dyg4S6CsdAZuIFW8eg!!pic.jpeg" },
            { id: "25-home", categoryName: "枕头", categoryPic: BASE_URL + "img/gallery/demo/16800817406z1c2Mfya4mXBPeWK5!!pic.jpeg" },
            { id: "26-home", categoryName: "毛毯", categoryPic: BASE_URL + "img/gallery/demo/16800817642vqqI5WOyFZx7tzWNt!!pic.jpeg" },
            { id: "24-home", categoryName: "床垫床褥", categoryPic: BASE_URL + "img/gallery/demo/16800817010xgA0W1WnXpiD5PMM0!!pic.jpeg" }
        ]
    },
    {
        id: "10-2",
        categoryName: "杯壶餐具",
        children: [
            { id: "37-cup", categoryName: "保温杯壶", categoryPic: BASE_URL + "img/gallery/demo/1680141635lnLO9uOYtOjc1FmHO0!!pic.png" },
            { id: "38-cup", categoryName: "冷水壶", categoryPic: BASE_URL + "img/gallery/demo/1680141705SgdjGmlBdXbkX0PqZ6!!pic.png" },
            { id: "39-cup", categoryName: "玻璃杯", categoryPic: BASE_URL + "img/gallery/demo/16801417468CllLXOUOaVKrdmObR!!pic.png" },
            { id: "40-cup", categoryName: "碗碟盘", categoryPic: BASE_URL + "img/gallery/demo/1680141775phNAAEQVURV773coZb!!pic.png" },
            { id: "41-cup", categoryName: "餐具套装", categoryPic: BASE_URL + "img/gallery/demo/1680141801c6z5Jif2dD43MZgqG7!!pic.png" },
            { id: "92-cup", categoryName: "保鲜/饭盒", categoryPic: BASE_URL + "img/gallery/demo/1680144052QlKYoadOHNt3TiASwk!!pic.png" }
        ]
    },
    {
        id: "10-3",
        categoryName: "烹饪厨房",
        children: [
            { id: "98-cook", categoryName: "锅具", categoryPic: BASE_URL + "img/gallery/demo/1680144199CHjBeEZ7vOiFAOsbuX!!pic.png" },
            { id: "99-cook", categoryName: "厨房置物架", categoryPic: BASE_URL + "img/gallery/demo/1680144198CJWq3VM74hNGFI0Rq9!!pic.png" },
            { id: "100-cook", categoryName: "刀具/剪刀", categoryPic: BASE_URL + "img/gallery/demo/1680144198DHJTmBSFVBEGXfCF16!!pic.png" }
        ]
    }
];

/** 国内版一级分类（原版 Preview.vue cateList，含「图书」共 15 项） */
const CATE_LIST: CategoryItem[] = [
    { id: "1", categoryName: "女装" },
    { id: "2", categoryName: "男装" },
    { id: "3", categoryName: "内衣" },
    { id: "4", categoryName: "鞋包" },
    { id: "5", categoryName: "母婴" },
    { id: "6", categoryName: "运动" },
    { id: "7", categoryName: "美妆" },
    { id: "8", categoryName: "配饰" },
    { id: "9", categoryName: "家电" },
    { id: "10", categoryName: "家居", children: HOME_CHILDREN },
    { id: "11", categoryName: "家装" },
    { id: "12", categoryName: "食品" },
    { id: "13", categoryName: "国际" },
    { id: "14", categoryName: "医药" },
    { id: "15", categoryName: "图书" }
];

/** 海外版一级分类（原版 ViewOverseas.vue cateList，无「图书」，共 14 项） */
const CATE_LIST_OVERSEAS: CategoryItem[] = CATE_LIST.filter((item) => item.id !== "15");

/** 装修颜色变量：只下发配置过的字段，未配置的走 CSS fallback（与原版 COLOR_VARS 一致） */
const COLOR_VARS: Array<[string, string]> = [
    ["sideBgColor", "--dec-cate-side-bg"],
    ["sideTextColor", "--dec-cate-side-text"],
    ["sideActiveTextColor", "--dec-cate-side-active-text"],
    ["sideActiveBgColor", "--dec-cate-side-active-bg"],
    ["groupTitleColor", "--dec-cate-group-title"],
    ["categoryNameColor", "--dec-cate-name"]
];

function SearchIcon({ size = 14 }: { size?: number }) {
    return (
        <svg className="cate-search-icon" viewBox="0 0 1024 1024" width={size} height={size} fill="currentColor" aria-hidden="true">
            <path d="M448 768A320 320 0 1 0 448 128a320 320 0 0 0 0 640z m297.6 43.3a448 448 0 1 1 66.7-66.7l194 194a48 48 0 0 1-67.9 67.9l-192.8-195.2z" />
        </svg>
    );
}

function BackIcon() {
    return (
        <svg viewBox="0 0 1024 1024" width="20" height="20" aria-hidden="true">
            <path
                d="M685.248 104.704a32 32 0 0 1 0 45.248L318.176 512l367.072 362.048a32 32 0 1 1-45.248 45.248l-392-384.64a32 32 0 0 1 0-45.248l392-384.64a32 32 0 0 1 45.248 0z"
                fill="#333"
            />
        </svg>
    );
}

export default function CateCenterV1Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, cateCenterV1Defaults);
        return merged;
    }, [data]);

    const frameFormat = formatFrame(module.frame ?? {});
    const decorateVars: Record<string, string> = {};
    COLOR_VARS.forEach(([field, varName]) => {
        const value = module[field];
        if (value) decorateVars[varName] = String(value);
    });
    const rootStyle = {
        ...cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom),
        ...decorateVars
    } as React.CSSProperties;

    const recommendText = String(module.recommendText ?? "").trim();
    const showRecommendEntry = module.showRecommend !== false && recommendText.length > 0;
    const showCategoryName = module.showCategoryName !== false;
    // 原版默认选中「家居」(catId="10")，右侧展示其子分类分组
    const activeId = "10";
    const childCat = CATE_LIST.find((item) => item.id === activeId)?.children ?? [];

    const renderAside = (cateList: CategoryItem[]) => (
        <div className="aside">
            {showRecommendEntry ? (
                <div className="item acea-row row-center-wrapper">
                    <span>{recommendText}</span>
                </div>
            ) : null}
            {cateList.map((item) => (
                <div key={item.id} className={`item acea-row row-center-wrapper${activeId === item.id ? " on" : ""}`}>
                    <span>{item.catShortName || item.categoryName}</span>
                </div>
            ))}
        </div>
    );

    const renderContent = () => (
        <div className="conter">
            {childCat.map((cat) =>
                cat.children && cat.children.length > 0 ? (
                    <div className="listw" key={cat.id}>
                        <div className="category-title">{cat.categoryName}</div>
                        <div className="list acea-row">
                            {cat.children.map((child) => (
                                <a className="item acea-row row-column row-middle" href="javascript:;" key={child.id}>
                                    {child.categoryPic ? (
                                        <div className="picture">
                                            <img src={child.categoryPic} alt="" />
                                        </div>
                                    ) : null}
                                    {showCategoryName ? <div className="name line1">{child.categoryName}</div> : null}
                                </a>
                            ))}
                        </div>
                    </div>
                ) : null
            )}
        </div>
    );

    // 海外走分块布局 ViewOverseas（已接颜色变量）；国内走平铺预览（与原版分支一致）
    if (isOverseas()) {
        return (
            <div className="module-ad-con dec-cate-center-v1-overseas" style={rootStyle}>
                <div className="search-bar-box-overseas">
                    <div className="back-btn">
                        <BackIcon />
                    </div>
                    <div className="search-input-wrap">
                        <span className="search-placeholder">{module.searchPlaceholder ?? "搜索"}</span>
                        <SearchIcon size={15} />
                    </div>
                </div>
                <div className="productSort">
                    {renderAside(CATE_LIST_OVERSEAS)}
                    {renderContent()}
                </div>
            </div>
        );
    }

    return (
        <div className="module-ad-con dec-cate-center-v1" style={rootStyle}>
            <div className="search-bar-box">
                <div className="search-input-wrap">
                    <SearchIcon />
                    <span className="search-placeholder">{module.searchPlaceholder ?? "点击搜索商品信息"}</span>
                </div>
            </div>
            <div className="productSort">
                {renderAside(CATE_LIST)}
                {renderContent()}
            </div>
        </div>
    );
}
