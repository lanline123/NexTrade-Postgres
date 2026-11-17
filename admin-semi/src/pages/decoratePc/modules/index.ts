/**
 * PC 装修编辑器模块注册入口（Step 3 模块化）：
 * 导入全部模块目录触发各自 index.tsx 的 registerPcModule 自注册；
 * 预览组件按 type 汇成 PC_MODULE_PREVIEWS 供画布分发（无预览的旧版模块走摘要回退）。
 */
import type { ComponentType } from 'react';
import './pc-banner2';
import './pc-category-nav';
import './pc-category-nav3';
import './pc-divider';
import './pc-group-banner';
import './pc-home-coupon';
import './pc-home-groupon';
import './pc-home-hero';
import './pc-home-quick-nav';
import './pc-home-seckill';
import './pc-image-ad';
import './pc-image-hotarea';
import './pc-image-nav';
import './pc-image-square-ad';
import './pc-notice';
import './pc-product';
import './pc-product-group';
import './pc-recommend-product';
import './pc-spacer';
import './pc-title-ad';
import './pcBanner';
import './pcBanner2';
import './pcCatProductNormal';
import './pcCatProductSimple';
import './pcCatProductWide';
import './pcCatWide';
import './pcCategoryNav';
import './pcCategoryNav3';
import './pcCoupon';
import './pcCustomAd1';
import './pcCustomAd2';
import './pcGroupBanner';
import './pcGroupon';
import './pcImageAd';
import './pcLogin';
import './pcRecommendProduct';
import './pcSearchResult';
import './pcSeckill';
import PcBanner2Preview from './pc-banner2/Preview';
import PcCategoryNavPreview from './pc-category-nav/Preview';
import PcCategoryNav3Preview from './pc-category-nav3/Preview';
import PcDividerPreview from './pc-divider/Preview';
import PcGroupBannerPreview from './pc-group-banner/Preview';
import PcHomeCouponPreview from './pc-home-coupon/Preview';
import PcHomeGrouponPreview from './pc-home-groupon/Preview';
import PcHomeHeroPreview from './pc-home-hero/Preview';
import PcHomeQuickNavPreview from './pc-home-quick-nav/Preview';
import PcHomeSeckillPreview from './pc-home-seckill/Preview';
import PcImageAdPreview from './pc-image-ad/Preview';
import PcImageHotareaPreview from './pc-image-hotarea/Preview';
import PcImageNavPreview from './pc-image-nav/Preview';
import PcImageSquareAdPreview from './pc-image-square-ad/Preview';
import PcNoticePreview from './pc-notice/Preview';
import PcProductPreview from './pc-product/Preview';
import PcProductGroupPreview from './pc-product-group/Preview';
import PcRecommendProductPreview from './pc-recommend-product/Preview';
import PcSpacerPreview from './pc-spacer/Preview';
import PcTitleAdPreview from './pc-title-ad/Preview';
import PcloginPreview from './pcLogin/Preview';
import PcsearchresultPreview from './pcSearchResult/Preview';

export { allPcModuleDefs, getPcModuleDef } from '../contract/pcModuleContract';

export const PC_MODULE_PREVIEWS: Record<string, ComponentType<{ data: Record<string, any> }>> = {
    'pc-banner2': PcBanner2Preview,
    'pc-category-nav': PcCategoryNavPreview,
    'pc-category-nav3': PcCategoryNav3Preview,
    'pc-divider': PcDividerPreview,
    'pc-group-banner': PcGroupBannerPreview,
    'pc-home-coupon': PcHomeCouponPreview,
    'pc-home-groupon': PcHomeGrouponPreview,
    'pc-home-hero': PcHomeHeroPreview,
    'pc-home-quick-nav': PcHomeQuickNavPreview,
    'pc-home-seckill': PcHomeSeckillPreview,
    'pc-image-ad': PcImageAdPreview,
    'pc-image-hotarea': PcImageHotareaPreview,
    'pc-image-nav': PcImageNavPreview,
    'pc-image-square-ad': PcImageSquareAdPreview,
    'pc-notice': PcNoticePreview,
    'pc-product': PcProductPreview,
    'pc-product-group': PcProductGroupPreview,
    'pc-recommend-product': PcRecommendProductPreview,
    'pc-spacer': PcSpacerPreview,
    'pc-title-ad': PcTitleAdPreview,
    'pcLogin': PcloginPreview,
    'pcSearchResult': PcsearchresultPreview,
};
