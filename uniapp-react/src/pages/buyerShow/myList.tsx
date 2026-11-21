import { useRef } from "react";
import { View } from "@tarojs/components";
import Taro, { useLoad } from "@tarojs/taro";

import TigLayout from "@/components/tig/tig-layout/TigLayout";
import EmptyBox from "@/components/empty-box/EmptyBox";
import LoadingBox from "@/components/loading-box";
import WaterfallLayout from "./buyerShowCompnents/WaterfallLayout";
import BuyerShowItem from "./src/BuyerShowItem";
import { buyerShowList } from "@/api/buyerShow/buyerShow";
import type { BuyerShowFilterState, BuyerShowListParams } from "@/types/buyerShow/buyerShow";
import { useConfigStore } from "@/store/config";
import { useUserStore } from "@/store/user";
import useList from "@/hooks/useList";
import { redirect } from "@/utils";
import { handleLogin } from "@/utils/handleLogin";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";
import { prefetchVideoFeedEntry } from "./composables/videoFeedPrefetch";
import { isVideoBuyerShow } from "./utils/videoFeed";

import "./index.scss";

/**
 * 我的买家秀（论坛 757744 个人买家秀列表）
 * - 数据走买家秀列表接口的仅本人模式（mine=1）：后端以登录令牌的 user_id
 *   过滤，只返回本人发布的买家秀（含待审核 / 未显示的本人作品），
 *   前端不传、也无法指定他人 user_id
 * - 未登录时不发请求，展示登录引导（后端对该模式同样返回需登录）
 * - 列表复用买家秀公共列表的瀑布流与卡片组件，点击进现有详情 / 视频流
 * - 列表 VO 无审核状态字段，故本页不呈现审核状态（不造字段）
 */
export default function MyBuyerShowList() {
    usePageGuards();
    const configStore = useConfigStore();
    const userToken = useUserStore((s) => s.token);

    const hasToken = Boolean(userToken || Taro.getStorageSync("token"));

    const filterParamsRef = useRef<BuyerShowListParams>({
        page: 1,
        size: 10,
        mine: 1
    });

    const {
        getList,
        data: myListData,
        isLoading,
        isLoadMore,
        total
    } = useList<any, BuyerShowListParams>(buyerShowList as any, {
        params: filterParamsRef.current,
        path: {
            dataKey: "records"
        },
        immediate: false
    });

    useLoad(() => {
        if (hasToken) {
            void getList();
        }
    });

    /**
     * 点击列表项：与公共买家秀列表同口径——视频类进视频流并定位到该条，
     * 图文类进图文详情
     */
    const onProductClick = (item: BuyerShowFilterState) => {
        if (isVideoBuyerShow(item)) {
            prefetchVideoFeedEntry(item);
            redirect({ url: "/pages/buyerShow/videoFeed", param: { buyerShowId: item.buyerShowId } });
            return;
        }
        redirect({ url: "/pages/buyerShow/buyerShowDetail", param: { buyerShowId: item.buyerShowId } });
    };

    const addBuyerShow = () => {
        redirect({ url: "/pages/buyerShow/addBuyerShow" });
    };

    return (
        <TigLayout title={t("我的买家秀")}>
            <View className="buyerShowBox">
                <View className="container" style={{ marginTop: "calc(var(--nav-height))" }}>
                    <WaterfallLayout
                        list={myListData}
                        hasMore
                        loading={false}
                        layout={configStore.buyerShowTemplateStyle ? "single" : "double"}
                        onItemClick={(item) => onProductClick(item)}
                        renderItem={(item, index) => (
                            <BuyerShowItem buyerShowItem={item} buyerShowIndex={index} buyerShowListData={myListData} />
                        )}
                        renderItemRight={(item, index) => (
                            <BuyerShowItem buyerShowItem={item} buyerShowIndex={index} buyerShowListData={myListData} />
                        )}
                    />
                </View>
                {hasToken ? (
                    <View className="publish" style={{ bottom: "40rpx" }} onClick={addBuyerShow}>
                        <View className="iconfont-h5 icon-xiangji1" />
                        <View className="text">{t("发布")}</View>
                    </View>
                ) : null}
                <LoadingBox modelValue={isLoadMore} page={filterParamsRef.current.page} length={myListData.length} />
            </View>
            {!hasToken && (
                <View className="empty-box" onClick={handleLogin}>
                    <EmptyBox icon="salesman/no_order.png" text={t("请先登录后查看我的买家秀")} />
                </View>
            )}
            {hasToken && !isLoading && total === 0 && (
                <View className="empty-box">
                    <EmptyBox icon="salesman/no_order.png" text={t("暂无买家秀")} />
                </View>
            )}
        </TigLayout>
    );
}
