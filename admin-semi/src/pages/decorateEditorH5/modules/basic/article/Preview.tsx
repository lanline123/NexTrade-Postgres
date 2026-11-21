/**
 * 文章列表 预览（真实拉取版）
 *
 * 与原版的差异（有意偏离）：原版 view/admin article/Preview.vue 本身只是
 * 「文章列表」文字占位、不拉数据；本次按派单要求补真实预览，渲染形态与
 * 字段口径以 uniapp-react 前端渲染器（pages/customPage/modules/article）
 * 为准：三种 style、缩略图/标题/时间/阅读/点赞按开关显示、颜色取
 * moduleColor。数据契约（defaults）与原版逐字段一致，不因补预览而变。
 *
 * 取数走管理端 GET /content/article/list（与文章管理页 Content.tsx 同一
 * 接口，返回 { records, total }）：分类为级联路径数组时取末级 ID 传
 * articleCategoryId（后端服务层自展开子分类，与 useDecorateProductList
 * 的分类口径一致）；isShow=1 只取前台可见文章；size=articleNum。
 * 字段映射（ArticleRow）：articleId/articleTitle/articleThumb/addTime
 * （秒级时间戳，格式化方式同 Content.tsx）；阅读数按前端多名兜底
 * pageView ?? viewNum ?? viewCount 再兜底本接口的 clickCount（后端列表
 * VO 实际下发字段即 clickCount，原 Java ArticleVO 同）。
 * 点赞数口径（2026-11-21 articlelist-meta 变更）：原版与我方均无点赞
 * 数据源（Article 实体/VO 无点赞字段、无点赞表），此前「接口不下发故
 * 不显示」使 showLike 开关在任何数据下都永不生效；改为记录带
 * likeNum ?? likeCount ?? likes 时显示其值，否则开关开启时显示真实
 * 零值 0（与 uniapp 渲染器同口径）。statisticColor 前端渲染器亦未
 * 消费，此处同样不使用。
 * 三种 style 按原版命名差异化（同日）：1 大图展示=单列大图卡、
 * 2 两列展示(纵向)=两列纵向卡、3 两列展示(横向)=横向行式卡，
 * 此前 style1 实为小图行式、style2/3 渲染完全相同（见 Preview.css）。
 */
import { useEffect, useMemo, useState } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { apiGet } from "../../../../../api";
import { articleDefaults } from "./index";

import { useI18n } from '../../../../../i18n';
import "./Preview.css";

interface ArticleItem {
    articleId?: number;
    articleTitle?: string;
    articleThumb?: string;
    articleCategoryId?: number;
    addTime?: number | string;
    pageView?: number;
    viewNum?: number;
    viewCount?: number;
    clickCount?: number;
    likeNum?: number;
    likeCount?: number;
    likes?: number;
    [key: string]: any;
}

const viewOf = (r: ArticleItem) => r.pageView ?? r.viewNum ?? r.viewCount ?? r.clickCount;
const likeOf = (r: ArticleItem) => r.likeNum ?? r.likeCount ?? r.likes;

/** addTime：字符串原样（商城侧口径）；数字按秒级时间戳格式化（同 Content.tsx formatTime） */
const timeTextOf = (value: number | string | undefined): string => {
    if (value === undefined || value === null || value === "") return "";
    if (typeof value === "string") return value;
    if (!value) return "";
    return new Date(value * 1000).toLocaleString("zh-CN", { hour12: false });
};

export default function ArticlePreview({ data }: { data: Record<string, any> }) {
    const { t } = useI18n();
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, articleDefaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});

    const style = Number(module.style ?? 1) || 1;
    const articleNum = Number(module.articleNum) || 10;
    // 分类 ID 路径数组（兼容存量标量）：取末级 ID 做接口筛选参数
    const leafCategoryId = useMemo(() => {
        const raw = module.articleCategoryId;
        const arr = Array.isArray(raw) ? raw : raw ? [raw] : [];
        const ids = arr.map((x: any) => Number(x)).filter((x: number) => x > 0);
        return ids.length ? ids[ids.length - 1] : 0;
    }, [module.articleCategoryId]);

    const [list, setList] = useState<ArticleItem[]>([]);

    useEffect(() => {
        let alive = true;
        (async () => {
            try {
                const params: Record<string, any> = { page: 1, size: articleNum, isShow: 1 };
                if (leafCategoryId) params.articleCategoryId = leafCategoryId;
                const result = await apiGet<{ records?: ArticleItem[]; list?: ArticleItem[] }>("/content/article/list", params);
                if (!alive) return;
                setList((result?.records || result?.list || []).slice(0, articleNum));
            } catch {
                if (alive) setList([]);
            }
        })();
        return () => {
            alive = false;
        };
    }, [leafCategoryId, articleNum]);

    const colors = module.moduleColor || {};
    const showTime = Number(module.showTime ?? 1) !== 0;
    const showPageView = Number(module.showPageView ?? 1) !== 0;
    const showLike = Number(module.showLike ?? 1) !== 0;

    return (
        <div
            className="module-ad-con"
            style={cssText(
                commonStyle.moduleStyle.boxPaddingBottom,
                commonStyle.moduleStyle.boxPaddingTop,
                commonStyle.moduleStyle.boxPaddingBottom,
                commonStyle.moduleStyle.boxPadding,
                commonStyle.moduleStyle.backgroundSize,
                commonStyle.moduleStyle.backgroundImage
            )}
        >
            <div
                className="module-ad-content"
                style={cssText(
                    commonStyle.moduleContentStyle.backgroundImage,
                    commonStyle.moduleContentStyle.backgroundSize,
                    commonStyle.moduleContentStyle.boxRadius2,
                    commonStyle.moduleContentStyle.boxShadow,
                    commonStyle.moduleContentStyle.innerPadding
                )}
            >
                {list.length === 0 ? (
                    <div className="h5-article-empty">{t('暂无文章')}</div>
                ) : (
                    <div className={`h5-article-list h5-article-list--style${style}`}>
                        {list.map((article, index) => {
                            const view = viewOf(article);
                            const like = likeOf(article);
                            const timeText = timeTextOf(article.addTime);
                            return (
                                <div className="h5-article-item" key={article.articleId ?? index}>
                                    {article.articleThumb ? (
                                        <img
                                            className="h5-article-item__thumb"
                                            src={imageFormat(article.articleThumb)}
                                            alt=""
                                            style={{ borderRadius: `${module.picRadius || 0}px` }}
                                        />
                                    ) : null}
                                    <div className="h5-article-item__main">
                                        <div
                                            className="h5-article-item__title"
                                            style={{ color: colors.articleNameColor || "#333", fontWeight: module.articleNameWeight || 400 }}
                                        >
                                            {article.articleTitle}
                                        </div>
                                        <div className="h5-article-item__meta">
                                            {showTime && timeText ? (
                                                <span style={{ color: colors.articleTimeColor || "#999" }}>{timeText}</span>
                                            ) : null}
                                            {showPageView && view !== undefined ? (
                                                <span style={{ color: colors.articleViewColor || "#999" }}>{view}</span>
                                            ) : null}
                                            {showLike ? (
                                                <span style={{ color: colors.articleLikeColor || "#999" }}>{like ?? 0}</span>
                                            ) : null}
                                        </div>
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                )}
            </div>
        </div>
    );
}
