/** 商品详情 A3 设置（原版 productDetailA3/Settings.vue：含价格颜色 + 海外版「页面头部」组、无工具显隐行） */
import ProductDetailSettings from "../shared/ProductDetailSettings";

export default function Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return <ProductDetailSettings data={data} onChange={onChange} showPriceColor buyBarSwapped showTools={false} showHeaderRow />;
}
