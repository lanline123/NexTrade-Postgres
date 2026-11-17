/**
 * 文章分类级联选择器（复刻 view/admin components/select/src/SelectArticleCategory.vue 口径）
 *
 * 级联多级选择、任意层级可选（checkStrictly ↔ Semi changeOnSelect）、可清空；
 * 数据走管理端现有接口 GET /content/articleCategory/tree，节点字段
 * articleCategoryId / articleCategoryName / children 与原版 cascaderProps 一致。
 * 读写口径与原版一致：值为分类 ID 路径数组（如 [1, 3]），清空写回 []。
 */
import { useEffect, useState } from "react";
import { Cascader, Toast } from "@douyinfe/semi-ui";
import { apiGet } from "../../../../../api";
import { useI18n } from "../../../../../i18n";

interface CategoryNode {
    articleCategoryId: number;
    articleCategoryName: string;
    children?: CategoryNode[];
}

interface CascaderNode {
    label: string;
    value: number;
    children?: CascaderNode[];
}

let treeCache: CascaderNode[] | null = null;
let treePromise: Promise<CascaderNode[]> | null = null;

function toCascader(nodes: CategoryNode[]): CascaderNode[] {
    return (nodes || []).map((node) => ({
        label: node.articleCategoryName,
        value: node.articleCategoryId,
        children: node.children && node.children.length ? toCascader(node.children) : undefined
    }));
}

function loadCategoryTree(): Promise<CascaderNode[]> {
    if (treeCache) return Promise.resolve(treeCache);
    if (!treePromise) {
        treePromise = apiGet<any>("/content/articleCategory/tree")
            .then((result) => {
                const list = Array.isArray(result) ? result : result?.records || result?.list || [];
                treeCache = toCascader(list);
                return treeCache;
            })
            .catch((error) => {
                treePromise = null;
                throw error;
            });
    }
    return treePromise;
}

export default function SelectArticleCategory({
    value,
    onChange
}: {
    value: any;
    onChange: (ids: Array<number | string>) => void;
}) {
    const { t } = useI18n();
    const [treeData, setTreeData] = useState<CascaderNode[]>(treeCache ?? []);

    useEffect(() => {
        let alive = true;
        loadCategoryTree()
            .then((tree) => {
                if (alive) setTreeData(tree);
            })
            .catch((error: any) => {
                if (alive) Toast.error(error?.message || t("加载文章分类失败"));
            });
        return () => {
            alive = false;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // 存量值兼容：数组即路径；标量（旧口径单值）包一层展示，变更后统一写回数组
    const path: Array<number | string> = Array.isArray(value) ? value : value === undefined || value === null || value === "" || value === 0 ? [] : [value];

    return (
        <Cascader
            style={{ width: "100%" }}
            treeData={treeData}
            value={path.length ? (path as any) : undefined}
            placeholder={t("请选择分类")}
            changeOnSelect
            showClear
            onChange={(next: any) => onChange(Array.isArray(next) ? next : next === undefined || next === null ? [] : [next])}
        />
    );
}
