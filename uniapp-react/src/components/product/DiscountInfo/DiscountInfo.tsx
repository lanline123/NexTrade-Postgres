import { View, Text, Image } from "@tarojs/components";
import { Popup } from "@nutui/nutui-react-taro";

import FormatPrice from "@/components/format-price/FormatPrice";
import { staticResource } from "@/utils";
import { t } from "@/utils/i18n";

import "./DiscountInfo.scss";

interface Props {
    show: boolean;
    onShowChange?: (show: boolean) => void;
    total?: any;
    discountType?: string;
}

/**
 * 优惠明细弹窗（自旧 components/product/discountInfo.vue 移植）。
 * 曾在购物车页（pages/cart/src）做成局部组件；订单确认页（pages/order/check → totalCard）
 * 移植时按原计划上移到 components/product/ 共享，购物车页改为引用本组件。
 */
export default function DiscountInfo({ show, onShowChange, total = {}, discountType = "" }: Props) {
    // 购物车传「已勾选件数」，订单确认页只有 totalCount，取不到则为 0（模板里不渲染件数）
    const itemCount = Number((total as any)?.checkedCount ?? (total as any)?.totalCount ?? 0) || 0;

    // P1：原版 tig-popup 的可见圆角由 .discount 自身 28rpx 承担；NutUI 容器为方形白底会把圆角衬成直角，
    // 故容器同步 28rpx 上圆角 + overflow:hidden 对齐（不使用 NutUI round，其默认圆角值与原版不一致）
    return (
        <Popup
            visible={show}
            position="bottom"
            zIndex={999}
            closeable={false}
            onClose={() => onShowChange?.(false)}
            style={{ backgroundColor: "#fff", borderRadius: "12rpx 12rpx 0 0", overflow: "hidden" }}
        >
            <View className="discount">
                <View className="discount-title">
                    <View className="title-text">{t("优惠明细")}</View>
                    <View className="close-icon" onClick={() => onShowChange?.(false)}>
                        <Image src={staticResource("order/close.png")} />
                    </View>
                </View>
                <View className="discount-content">
                    <View className="discount-item-box">
                        <View className="item-label fontWeight">
                            {t("商品总额")}
                            {itemCount > 0 && <Text className="item-label-text">{`(${itemCount}${t("件")})`}</Text>}
                        </View>
                        <View className="item-value">
                            <FormatPrice showText={false} isBottom={false} withExact priceData={(total as any).productAmount} />
                        </View>
                    </View>
                    <View className="line" />
                    {/* serviceFee 以 Number(...)>0 单独成门：原 `x && x>0 &&` 链在 x 为数字 0 时会漏渲染文本「0」
                        （原版 Vue v-if 无此问题，React 直译引入）；线上金额为字符串时两式等价，渲染口径不变 */}
                    {Number((total as any).serviceFee) > 0 && (
                        <View className="discount-item-box">
                            <View className="item-label">{t("附加费用")}</View>
                            <View className="item-value">
                                <FormatPrice showText={false} isBottom={false} withExact priceData={(total as any).serviceFee} />
                            </View>
                        </View>
                    )}
                    <View className="discount-item-box">
                        <View className="item-label">{t("共优惠")}</View>
                        <View className="item-value discounts-value" style={{ color: "var(--general)" } as any}>
                            -
                            <FormatPrice showText={false} isBottom={false} withExact priceData={(total as any).discounts} />
                        </View>
                    </View>
                    {(total as any).discountCouponAmount > 0 && (
                        <View className="discount-item-box">
                            <View className="item-label">{t("优惠券")}</View>
                            <View className="item-value discounts-value">
                                -
                                <FormatPrice showText={false} isBottom={false} withExact priceData={(total as any).discountCouponAmount} />
                            </View>
                        </View>
                    )}
                    {(total as any).discountDiscountAmount > 0 && (
                        <View className="discount-item-box">
                            <View className="item-label">{t("其他优惠")}</View>
                            <View className="item-value discounts-value">
                                -
                                <FormatPrice showText={false} isBottom={false} withExact priceData={(total as any).discountDiscountAmount} />
                            </View>
                        </View>
                    )}
                    <View className="line" />
                    <View className="discount-item-box discount-total">
                        <View className="item-label fontWeight">{t("合计")}</View>
                        <View className="item-value">
                            <FormatPrice showText={false} isBottom={false} withExact priceData={(total as any).discountAfter || (total as any).totalAmount} fontStyle={{}} />
                        </View>
                    </View>
                    {!discountType && <View className="discount-desc">{t("以上优惠不包含运费，实际优惠金额请以确认订单页为准")}</View>}
                </View>
            </View>
        </Popup>
    );
}
