import { getProductsList } from "@/api/common";

type DecorateProductOptions = {
    shopId?: number;
    extraParams?: Record<string, any>;
    allowEmptyParams?: boolean;
};

const paramsValueMap: Record<number, string[]> = {
    1: ["productIds"],
    2: ["productCategoryId", "productNumber"],
    3: ["productTag", "productNumber"],
    // 4 全部商品（我方新增，论坛 B 类 237824）：只带 productNumber→size，
    // 无 ids/分类/标签过滤，按现有分页口径取全站上架商品（含门店商品）
    4: ["productNumber"]
};

const paramsKeyMap: Record<string, string> = {
    productIds: "ids",
    productCategoryId: "categoryId",
    productNumber: "size",
    productTag: "introType"
};

export const buildDecorateProductParams = (products: any, options: DecorateProductOptions = {}) => {
    const param: Record<string, any> = {};

    if (products && typeof products === "object" && products.productSelectType) {
        const type = Number(products.productSelectType);
        const keys = paramsValueMap[type];
        if (!keys) return null;

        if (type === 1) {
            const ids = Array.isArray(products.productIds) ? products.productIds : [];
            if (!ids.length) return null;
            param[paramsKeyMap[keys[0]]] = ids.join(",");
            param.size = ids.length;
        } else {
            param.page = 1;
            keys.forEach((key) => {
                param[paramsKeyMap[key]] = products[key];
            });
            // 全部商品：数量未配（0/未填）时不传 size，走后端默认分页（20），
            // 避免 size=0 被后端 clamp 成 1 件
            if (type === 4 && !(Number(param.size) > 0)) delete param.size;
        }
    } else if (!options.allowEmptyParams) {
        return null;
    }

    if (options.shopId && options.shopId !== -1) {
        param.shopId = options.shopId;
    }

    return {
        ...(options.extraParams || {}),
        ...param
    };
};

export const getDecorateProducts = async (products: any, options: DecorateProductOptions = {}) => {
    const params = buildDecorateProductParams(products, options);
    if (!params) return [];

    const result = await getProductsList(params);
    return result.records ?? [];
};
