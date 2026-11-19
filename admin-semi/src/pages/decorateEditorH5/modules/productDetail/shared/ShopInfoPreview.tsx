/**
 * 店铺信息预览（移植自 ref-editor mobileProductDetail/_shared/ShopInfoPreview.vue）
 * variant: vortmall（A1）/ o2o（A2）/ overseas（A3）；演示店铺「海澜之家」为原版硬编码。
 */
import { useI18n } from "../../../../../i18n";
import "./ShopInfoPreview.css";

const ShopGlyph = ({ size = 13 }: { size?: number }) => (
    <svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" style={{ marginRight: 4 }}>
        <path d="M4 7l1.2-3h13.6L20 7M4 7h16M4 7v13h16V7M9.5 20v-6h5v6" />
    </svg>
);

export default function ShopInfoPreview({ variant = "vortmall" }: { variant?: "vortmall" | "o2o" | "overseas" }) {
    const { t } = useI18n();
    return (
        <div className={`pd-shop-preview pd-shop-preview--${variant}`}>
            <div className="pd-shop-preview__title">
                <span className="pd-shop-preview__logo">
                    <img
                        className="pd-shop-preview__logo-image"
                        src="https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202508/1756457970fpZxKZJHdp6hTZovOz.jpeg"
                        alt={t("海澜之家")}
                    />
                </span>
                <span className="pd-shop-preview__name">{t("海澜之家")}</span>
            </div>
            <div className="pd-shop-preview__actions">
                <span>{t("收藏店铺")}</span>
                <span>
                    <ShopGlyph />
                    {t("进店逛逛")}
                </span>
            </div>
        </div>
    );
}
