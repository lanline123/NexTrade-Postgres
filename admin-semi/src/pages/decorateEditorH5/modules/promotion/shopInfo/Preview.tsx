/**
 * 店铺信息 预览（移植自 view/admin editor/modules/mobilePromotion/shopInfo/Preview.vue）
 *
 * 根 class module-store_info ad-store_style__<style>，cap-store-* 样式全部走全局 module.css（已移植）；
 * 背景图：backgroundDefault==0 且 customPic 有图时用自定义图，否则取预设 1..10（styles/img/bg 打包资源）；
 * 店招为原版写死的演示内容（Apple智慧生活专卖店 / 全部商品 20 / 新品 12 / 收藏按钮 / 搜商品 / 精选·商品·分类导航）。
 * 搜索图标与收藏图标为原版 iconfont/module_ico 类，本实现以同尺寸内联 SVG/字符承接。
 */
import { useMemo } from "react";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { imageFormat } from "../../../shared/format";
import { shopInfoDefaults } from "./index";
import { SHOP_INFO_BACKGROUNDS } from "./backgrounds";

const SHOP_LOGO = "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202406/17181844953OiwzdtLSXKLeU9oi7.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200";

function SearchIcon() {
    return (
        <svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor" aria-hidden="true" style={{ flexShrink: 0 }}>
            <path d="M945.6 854.4L721.9 630.7c34.5-46.5 54.9-104.1 54.9-166.4C776.8 243.1 597.7 64 376.5 64S-23.8 243.1-23.8 464.3c0 221.2 179.1 400.3 400.3 400.3 62.3 0 119.9-20.4 166.4-54.9l223.7 223.7c6.2 6.2 14.6 9.4 22.6 9.4 8 0 16.4-3.2 22.6-9.4 12.5-12.5 12.5-32.5 0-45zM64.2 464.3c0-172.3 140-312.3 312.3-312.3 172.3 0 312.3 140 312.3 312.3 0 83.5-32.5 162-91.5 221-59 59-137.5 91.5-221 91.5-172.3 0-312.3-140-312.3-312.3z" transform="translate(24 0)" />
        </svg>
    );
}

export default function ShopInfoPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, shopInfoDefaults);
        return merged;
    }, [data]);

    const backgroundImg =
        module.backgroundDefault === 0 && module.customPic?.picUrl
            ? imageFormat(module.customPic.picUrl)
            : SHOP_INFO_BACKGROUNDS[(module.backgroundDefault ?? 1) - 1];

    return (
        <div className={`module-ad-con module-store_info ad-store_style__${module.style ?? 1}`}>
            <div className="module-ad-content">
                <div className="cap-store-banner">
                    <div className="cap-store-banner__cover" style={{ backgroundImage: `url(${backgroundImg})` }}>
                        <div className="cap-store-banner__cover-mask" />
                    </div>
                    <div className="cap-store-nav-warp">
                        <div className="cap-store-nav-con">
                            <div className="store-search-con">
                                <div id="search">
                                    <SearchIcon />
                                    <input type="text" className="search-input" value="" placeholder="搜商品" readOnly />
                                </div>
                            </div>
                            <div className="store-nav-con">
                                <div className="store-nav-item active">
                                    <a className="store-nav-item-a">精选</a>
                                </div>
                                <div className="store-nav-item">
                                    <a className="store-nav-item-a">商品</a>
                                </div>
                                <div className="store-nav-item">
                                    <a className="store-nav-item-a">分类</a>
                                </div>
                            </div>
                            <div className="store-all-cat-con" style={{ display: "none" }}>
                                <span className="module_ico module-ico-fenlei" />
                            </div>
                        </div>
                    </div>
                    <div className="cap-store-banner__inner">
                        <div className="cap-store-banner__content">
                            <div className="cap-store-banner__logo">
                                <img src={SHOP_LOGO} alt="" />
                            </div>
                            <div className="cap-store-banner__right-content">
                                <h3 className="cap-store-banner__right-content-title--middle">Apple智慧生活专卖店</h3>
                                <div className="cap-store-banner__sum-content">
                                    <a>
                                        <div className="cap-store-banner__sum-content-total">全部商品 20</div>
                                    </a>
                                    <a>
                                        <div className="cap-store-banner__sum-content-total">新品 12</div>
                                    </a>
                                </div>
                            </div>
                            <div className="store-collect-button-con">
                                <div className="store-collect-button">
                                    <span style={{ marginRight: 2 }}>★</span>
                                    收藏
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
