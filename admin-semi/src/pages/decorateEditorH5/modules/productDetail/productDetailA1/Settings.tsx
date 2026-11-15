/** 商品详情 A1 设置（原版 productDetailA1/Settings.vue：无价格颜色、购买栏 1=简约/2=经典） */
import ProductDetailSettings from "../shared/ProductDetailSettings";

export default function Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return <ProductDetailSettings data={data} onChange={onChange} />;
}
