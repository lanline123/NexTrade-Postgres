/**
 * 分类中心 V1 预览（左一级分类栏 + 右分组分类网格；演示分类为占位，真实分类由渲染端接口下发，颜色字段驱动 CSS 变量口径的内联样式）
 */
import { useMemo } from "react";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { cateCenterV1Defaults } from "./index";
import "./Preview.css";

const DEMO_SIDE = ["推荐", "食品饮料", "家居日用", "服饰鞋包"];
const DEMO_GROUPS = [
    { title: "热门分类", names: ["零食小吃", "饮料冲调", "粮油调味"] },
    { title: "精选分类", names: ["个护清洁", "纸品湿巾", "家居收纳"] }
];

export default function CateCenterV1Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, cateCenterV1Defaults);
        return merged;
    }, [data]);
    const theme = module.themeColor || "#65c4aa";
    const side = DEMO_SIDE.filter((name, i) => i !== 0 || module.showRecommend !== false);
    return (
        <div className="module-ad-con dec-cate-v1">
            <div className="dec-cate-v1__search">
                <div className="dec-cate-v1__search-box">🔍 {module.searchPlaceholder}</div>
            </div>
            <div className="dec-cate-v1__body">
                <div className="dec-cate-v1__side" style={{ background: module.sideBgColor || "#f7f7f7" }}>
                    {side.map((name, i) => (
                        <div
                            key={name}
                            className={`dec-cate-v1__side-item${i === 0 ? " is-active" : ""}`}
                            style={
                                i === 0
                                    ? { color: module.sideActiveTextColor || theme, background: module.sideActiveBgColor || "#ffffff" }
                                    : { color: module.sideTextColor || "#333333" }
                            }
                        >
                            {i === 0 && module.showRecommend !== false ? module.recommendText || "推荐" : name}
                        </div>
                    ))}
                </div>
                <div className="dec-cate-v1__main">
                    {DEMO_GROUPS.map((group) => (
                        <div className="dec-cate-v1__group" key={group.title}>
                            <div className="dec-cate-v1__group-title" style={{ color: module.groupTitleColor || "#333333" }}>{group.title}</div>
                            <div className="dec-cate-v1__grid">
                                {group.names.map((name) => (
                                    <div className="dec-cate-v1__cell" key={name}>
                                        <div className="dec-cate-v1__cell-img" />
                                        {module.showCategoryName !== false ? <div className="dec-cate-v1__cell-name" style={{ color: module.categoryNameColor || "#333333" }}>{name}</div> : null}
                                    </div>
                                ))}
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}
