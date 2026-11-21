import { useEffect, useMemo, useState } from "react";
import { Image, Text, View } from "@tarojs/components";

import { formatCommonStyle, designPxToRpx } from "@/components/modules";
import { getArticleList } from "@/api/article/article";
import { redirect } from "@/utils";
import { imageFormat } from "@/utils/format";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

/**
 * 文章列表装修模块（type=article，本线新增渲染入口）。
 * 数据契约以 admin 精确版 articleDefaults 为准（style/articleCategoryId/
 * articleNum/showTime/showPageView/showLike/颜色组）；admin 侧该模块
 * visible:false（旧 ToolBar 已停用），本入口主要保证存量装修数据可渲染。
 * 分类落地说明：后端 article/article/list 的筛选参数只有 categorySn，
 * 不支持分类 ID 数组；此处拉取较大页后按记录自带的 articleCategoryId
 * 在前端过滤（兼容旧编辑器标量 0 写法），无分类时直接取 articleNum 条。
 * 元信息口径（2026-11-21 articlelist-meta 修复，与编辑器 Preview 对齐）：
 * - 浏览量：后端列表 VO 实际下发的是 clickCount（原 Java ArticleVO 同），
 *   viewOf 兜底到 clickCount，此前只认 pageView/viewNum/viewCount 致永不显示；
 * - 日期：后端下发 epoch 秒（原 Java VO 为秒数字符串），渲染前格式化为
 *   YYYY-MM-DD HH:mm，此前原样输出成 10 位时间戳；
 * - 点赞数：原版与我方均无点赞数据源（Article 实体/VO 无点赞字段、无点赞
 *   表），记录带 likeNum/likeCount/likes 时显示其值，否则开关开启时显示
 *   真实零值 0——避免 showLike 开关永久失效（口径变更，见修复报告）；
 * - 容器样式：本模块数据契约是 moduleStyle/contentStyle（同批模块
 *   storeNav/imageTextComponent 同），此前误读 module.frame 致扩展设置
 *   的模块样式永不生效，改走 formatCommonStyle 与编辑器 Preview 一致；
 * - 三种 style 按原版命名差异化：1 大图展示=单列大图卡（图置顶全宽）、
 *   2 两列展示(纵向)=两列纵向卡、3 两列展示(横向)=横向行式卡（图左文右），
 *   此前 style1 实为小图行式、style2/3 渲染完全相同。
 * 尺寸口径：样式表与 picRadius 均按 375 设计像素基准（rpx/designPxToRpx），
 * 2026-11-21 审计收口——此前 SCSS 误写 px 字面量致真机渲染整体减半
 * （见 index.scss 头注与 articlelist-audit/REPORT.md）。
 */
export default function ArticleModule({ module }: Props) {
    const d = useMemo(() => {
        const data = module?.data && typeof module.data === "object" ? module.data : {};
        return { ...module, ...data };
    }, [module]);

    const commonStyle = useMemo(
        () => formatCommonStyle(d.moduleStyle ?? {}, d.contentStyle ?? {}, d.subTitle ?? {}),
        [d]
    );
    const [list, setList] = useState<any[]>([]);

    const style = Number(d.style ?? 1);
    const articleNum = Number(d.articleNum ?? 10) || 10;
    const categoryIds = useMemo(() => {
        const raw = d.articleCategoryId;
        const arr = Array.isArray(raw) ? raw : raw ? [raw] : [];
        return arr.map((x: any) => Number(x)).filter((x: number) => x > 0);
    }, [d.articleCategoryId]);

    useEffect(() => {
        let cancelled = false;
        (async () => {
            try {
                const size = categoryIds.length ? Math.max(articleNum * 5, 50) : articleNum;
                const res: any = await getArticleList({ page: 1, size, categorySn: "" } as any);
                const records: any[] = res?.records || res?.data?.records || [];
                const filtered = categoryIds.length
                    ? records.filter((r) => categoryIds.includes(Number(r.articleCategoryId)))
                    : records;
                if (!cancelled) setList(filtered.slice(0, articleNum));
            } catch {
                if (!cancelled) setList([]);
            }
        })();
        return () => {
            cancelled = true;
        };
    }, [articleNum, categoryIds]);

    if (!list.length) return null;

    const colors = d.moduleColor || {};
    const viewOf = (r: any) => r.pageView ?? r.viewNum ?? r.viewCount ?? r.clickCount;
    const likeOf = (r: any) => r.likeNum ?? r.likeCount ?? r.likes;
    /** addTime：后端下发 epoch 秒（数字或数字字符串），格式化为 YYYY-MM-DD HH:mm；已格式化字符串原样；0/空不显示 */
    const timeTextOf = (value: any): string => {
        if (value === undefined || value === null || value === "" || value === 0 || value === "0") return "";
        const num = typeof value === "number" ? value : /^\d+$/.test(String(value)) ? Number(value) : NaN;
        if (!Number.isFinite(num)) return String(value);
        const dt = new Date(num * 1000);
        const p = (n: number) => String(n).padStart(2, "0");
        return `${dt.getFullYear()}-${p(dt.getMonth() + 1)}-${p(dt.getDate())} ${p(dt.getHours())}:${p(dt.getMinutes())}`;
    };

    return (
        <View
            className={`module-ad-con module-article article-style__${style}`}
            style={
                commonStyle.moduleStyle.boxPaddingBottom +
                commonStyle.moduleStyle.boxPaddingTop +
                commonStyle.moduleStyle.boxPaddingBottom +
                commonStyle.moduleStyle.boxPadding +
                commonStyle.moduleStyle.backgroundSize +
                commonStyle.moduleStyle.backgroundImage
            }
        >
            <View
                className="module-ad-content"
                style={
                    commonStyle.moduleContentStyle.backgroundImage +
                    commonStyle.moduleContentStyle.backgroundSize +
                    commonStyle.moduleContentStyle.boxRadius2 +
                    commonStyle.moduleContentStyle.boxShadow +
                    commonStyle.moduleContentStyle.innerPadding
                }
            >
                {list.map((article) => {
                    const timeText = timeTextOf(article.addTime);
                    return (
                        <View
                            key={article.articleId}
                            className="article-item"
                            onClick={() => redirect({ url: `/pages/article/news/detail?id=${article.articleId}` })}
                        >
                            {article.articleThumb ? (
                                <Image className="article-item__thumb" src={imageFormat(article.articleThumb)} mode="aspectFill" style={`border-radius: ${designPxToRpx(d.picRadius || 0)};`} />
                            ) : null}
                            <View className="article-item__main">
                                <Text
                                    className="article-item__title"
                                    style={{ color: colors.articleNameColor || "#333", fontWeight: d.articleNameWeight || 400 }}
                                >
                                    {article.articleTitle}
                                </Text>
                                <View className="article-item__meta">
                                    {Number(d.showTime ?? 1) !== 0 && timeText ? (
                                        <Text style={{ color: colors.articleTimeColor || "#999" }}>{timeText}</Text>
                                    ) : null}
                                    {Number(d.showPageView ?? 1) !== 0 && viewOf(article) !== undefined ? (
                                        <Text style={{ color: colors.articleViewColor || "#999" }}>{viewOf(article)}</Text>
                                    ) : null}
                                    {Number(d.showLike ?? 1) !== 0 ? (
                                        <Text style={{ color: colors.articleLikeColor || "#999" }}>{likeOf(article) ?? 0}</Text>
                                    ) : null}
                                </View>
                            </View>
                        </View>
                    );
                })}
            </View>
        </View>
    );
}
