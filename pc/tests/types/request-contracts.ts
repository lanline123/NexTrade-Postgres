import { request, asyncRequest } from "~/utils/request";
import { getLocationByIp, getLocationByLocation } from "~/api/region";
import { getActiveStyle, batchActiveStyle } from "~/api/product/productStyle";
import { getProductDetail, getProductSkuDetail } from "~/api/product/product";
import { getProductList, getHomeRecommendProductData } from "~/api/home/home";
import { getArticle } from "~/api/article/article";
import type { ByLocationData } from "~/types/region";
import type { ProductActiveStyleVO, BatchActiveStyleData } from "~/types/product/productStyle";
import type { ProductFilterResult, SkuDetail } from "~/types/product/product";
import type { SearchProductListResult } from "~/types/search/search";
import type { HomeCategoryRecommendData } from "~/types/home/home";
import type { ArticleFormResult } from "~/types/article/article";

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends (<T>() => T extends B ? 1 : 2) ? true : false;
declare function check<T extends true>(): void;

export function requestContracts() {
    const ssr = request<{ id: number }>({ url: "/fixture", method: "get" });
    const client = asyncRequest<{ id: number }>({ url: "/fixture", method: "get" });
    check<Equal<typeof ssr, Promise<{ id: number }>>>();
    check<Equal<typeof client, Promise<{ id: number }>>>();
    check<Equal<ReturnType<typeof getLocationByIp>, Promise<ByLocationData>>>();
    check<Equal<ReturnType<typeof getLocationByLocation>, Promise<ByLocationData>>>();
    check<Equal<ReturnType<typeof getActiveStyle>, Promise<ProductActiveStyleVO>>>();
    check<Equal<ReturnType<typeof batchActiveStyle>, Promise<BatchActiveStyleData>>>();
    check<Equal<ReturnType<typeof getProductDetail>, Promise<ProductFilterResult>>>();
    check<Equal<ReturnType<typeof getProductSkuDetail>, Promise<SkuDetail>>>();
    check<Equal<ReturnType<typeof getProductList>, Promise<SearchProductListResult>>>();
    check<Equal<ReturnType<typeof getHomeRecommendProductData>, Promise<HomeCategoryRecommendData>>>();
    check<Equal<ReturnType<typeof getArticle>, Promise<ArticleFormResult>>>();
    // @ts-expect-error Region data is already unwrapped.
    getLocationByIp().then((data) => data.data);
    // @ts-expect-error SKU identifiers are numeric.
    getProductSkuDetail(1, { skuId: "sku" });
}

import { getHomeGrouponList } from "~/api/home/home";
import { getLang } from "~/api/common";
import { orderPayInfo, checkPayStatus } from "~/api/order/pay";
import type { GrouponItemListResult } from "~/types/home/groupon";

export function moduleContracts() {
    check<Equal<ReturnType<typeof getHomeGrouponList>, Promise<GrouponItemListResult>>>();
    check<Equal<ReturnType<typeof getLang>, Promise<Record<string, string>>>>();
    check<Equal<ReturnType<typeof checkPayStatus>, Promise<number>>>();
    getHomeGrouponList({ grouponPromotionId: 3 }).then((result) => result.records[0].productName);
    orderPayInfo({ id: "1" }).then((result) => result.offlinePaymentList.offlinePayBank);
    // @ts-expect-error Groupon pagination is numeric.
    getHomeGrouponList({ page: "1" });
    // @ts-expect-error Orders cannot be selected by a boolean ID.
    orderPayInfo({ id: true });
}
