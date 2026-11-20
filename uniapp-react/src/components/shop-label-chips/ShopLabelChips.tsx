import { View, Text } from "@tarojs/components";

export interface ShopLabelData {
    labelId?: number;
    labelName?: string;
    color?: string;
}

/**
 * 店铺标签徽章（本平台新增，论坛帖 207878955346177229）。
 * 平台分配的展示标签，紧贴店铺名渲染；颜色取标签库配置，未配色回退品牌红。
 * 与门店服务提示 tipNames 是两套概念。无标签时不渲染。
 */
export default function ShopLabelChips({ labels }: { labels?: ShopLabelData[] | null }) {
    const list = (labels ?? []).filter((item) => item && item.labelName);
    if (list.length === 0) return null;
    return (
        <View style={{ display: "flex", flexDirection: "row", flexWrap: "wrap", alignItems: "center" }}>
            {list.map((item, index) => (
                <View
                    key={item.labelId ?? index}
                    style={{
                        backgroundColor: item.color || "#E93B3D",
                        height: "32rpx",
                        lineHeight: "32rpx",
                        paddingLeft: "10rpx",
                        paddingRight: "10rpx",
                        marginLeft: index === 0 ? "0" : "8rpx",
                        marginTop: "4rpx",
                        marginBottom: "4rpx",
                        flexShrink: 0
                    }}
                >
                    <Text style={{ color: "#fff", fontSize: "20rpx" }}>{item.labelName}</Text>
                </View>
            ))}
        </View>
    );
}
