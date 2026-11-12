import { getProductsList } from "@/api/common";
import type { ProductsListResult } from "@/types/common/common";

type data = {
    productSelectType?: number;
    productIds?: string[];
    productCategoryId?: string;
    productNumber?: string;
    productTag?: string;
    shopId?: string;
};

export default function useComProParams(data: data) {
    const paramsValueMap: { [key: number]: string[] } = {
        1: ["productIds"],
        2: ["productCategoryId", "productNumber"],
        3: ["productTag", "productNumber"]
    };
    const paramsKeyMap: { [key: string]: string } = {
        productIds: "ids",
        productCategoryId: "categoryId",
        productNumber: "size",
        productTag: "introType"
    };
    const productList = ref<ProductsListResult[]>([]);

    const params = computed(() => {
        let param: any = {};
        if (typeof data === "object") {
            if (data.productSelectType) {
                const type = data.productSelectType;
                const keys = paramsValueMap[type];
                if (type === 1) {
                    param[paramsKeyMap[keys[0]]] = data.productIds.join(",");
                } else {
                    param.page = 1;
                    keys.forEach((key) => {
                        param[paramsKeyMap[key]] = data[key];
                    });
                }
            }
            if (data.shopId) {
                param.shopId = data.shopId;
            }
        }

        return param;
    });

    const getProductsListData = async () => {
        try {
            const result = await getProductsList(params.value);
            productList.value = result.records;
        } catch (error) {
            console.log(error);
        }
    };

    onMounted(async () => {
        await getProductsListData();
    });

    return {
        productList
    };
}
