/** 商品详情 A2 设置（原版 productDetailA2/Settings.vue：含价格颜色、购买栏 2=简约/1=经典且圆角在 style===2 显示） */
import ProductDetailSettings from "../shared/ProductDetailSettings";

export default function Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return <ProductDetailSettings data={data} onChange={onChange} showPriceColor buyBarSwapped />;
}
