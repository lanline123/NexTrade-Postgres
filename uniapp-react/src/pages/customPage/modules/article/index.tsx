import { useEffect, useMemo, useState } from "react";
import { Image, Text, View } from "@tarojs/components";

import { formatFrame } from "@/components/modules";
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
 * 阅读/点赞数不在列表返回类型 ArticleFilterState 中，记录实际下发时
 * 才展示（pageView/likeCount/viewNum/likeNum 字段存在才渲染）。
 */
export default function ArticleModule({ module }: Props) {
    const d = useMemo(() => {
        const data = module?.data && typeof module.data === "object" ? module.data : {};
        return { ...module, ...data };
    }, [module]);

    const frameFormat = useMemo(() => formatFrame(d.frame ?? {}), [d]);
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
    const viewOf = (r: any) => r.pageView ?? r.viewNum ?? r.viewCount;
    const likeOf = (r: any) => r.likeNum ?? r.likeCount ?? r.likes;

    return (
        <View
            className={`module-ad-con module-article article-style__${style}`}
            style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom}
        >
            <View className="module-ad-content" style={frameFormat.innerPadding + frameFormat.backgroundColor + frameFormat.boxRadius}>
                {list.map((article) => (
                    <View
                        key={article.articleId}
                        className="article-item"
                        onClick={() => redirect({ url: `/pages/article/news/detail?id=${article.articleId}` })}
                    >
                        {article.articleThumb ? (
                            <Image className="article-item__thumb" src={imageFormat(article.articleThumb)} mode="aspectFill" style={{ borderRadius: `${d.picRadius || 0}px` }} />
                        ) : null}
                        <View className="article-item__main">
                            <Text
                                className="article-item__title"
                                style={{ color: colors.articleNameColor || "#333", fontWeight: d.articleNameWeight || 400 }}
                            >
                                {article.articleTitle}
                            </Text>
                            <View className="article-item__meta">
                                {d.showTime !== 0 && article.addTime ? (
                                    <Text style={{ color: colors.articleTimeColor || "#999" }}>{article.addTime}</Text>
                                ) : null}
                                {d.showPageView !== 0 && viewOf(article) !== undefined ? (
                                    <Text style={{ color: colors.articleViewColor || "#999" }}>{viewOf(article)}</Text>
                                ) : null}
                                {d.showLike !== 0 && likeOf(article) !== undefined ? (
                                    <Text style={{ color: colors.articleLikeColor || "#999" }}>{likeOf(article)}</Text>
                                ) : null}
                            </View>
                        </View>
                    </View>
                ))}
            </View>
        </View>
    );
}
