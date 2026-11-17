/**
 * 公告混排 预览（移植自 view/admin editor/modules/mobilePro/mixedNotice/Preview.vue）
 *
 * 每组一条公告时静态展示，多条时纵向 Swiper 轮播（图 + 标题 + 卖点副标题）：
 * 原版 swiperOption = { pagination: false, direction: "vertical", speed: 800 }，
 * 可视窗口高 40px、一次只露一条（.swiper{height:40px}）。
 * NutUI 的根节点类名是 .nut-swiper 而非 .swiper，且 direction 默认 horizontal、
 * 高度默认 100%——不显式传 direction="vertical" + className="swiper" + 定高时，
 * 全部公告会平铺成多行并横向滑动（2026-11-17 实测高 130px、两组重叠，已修）。
 */
import { useMemo } from "react";
import { Swiper, SwiperItem } from "@nutui/nutui-react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { mixedNoticeDefaults } from "./index";
import "./Preview.css";

interface NoticeItem {
    picUrl?: string;
    picTitle?: string;
    picDesc?: string;
}

export default function MixedNoticePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, mixedNoticeDefaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {});
    const groups: Array<{ notices?: NoticeItem[] }> = module.noticeGroups || [];

    const renderItem = (notice: NoticeItem, key: React.Key) => (
        <div className="notices-item" key={key}>
            <div className="notices-item-img">
                <img src={imageFormat(notice.picUrl)} alt="" />
            </div>
            <div className="notices-item-content">
                {/* 原版 .content-title/.content-subtitle 颜色写死（#505259/#888b94），Preview 不应用 moduleColor */}
                <div className="content-title">{notice.picTitle}</div>
                <div className="content-subtitle">{notice.picDesc}</div>
            </div>
        </div>
    );

    return (
        <div
            className="module-ad-con"
            style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}
        >
            <div
                className="module-ad-content"
                style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}
            >
                <div className="mixed-notice">
                    <div className="notices">
                        {groups.map((group, index) => {
                            const notices = group.notices || [];
                            return (
                                <div className="notices-item-box" key={index}>
                                    {notices.length <= 1 ? (
                                        notices.map((notice, subIndex) => renderItem(notice, subIndex))
                                    ) : (
                                        <Swiper
                                            className="swiper"
                                            style={{ height: 40 }}
                                            direction="vertical"
                                            autoplay={3000}
                                            loop={notices.length > 1}
                                            indicator={false}
                                        >
                                            {notices.map((notice, subIndex) => (
                                                <SwiperItem key={subIndex}>{renderItem(notice, subIndex)}</SwiperItem>
                                            ))}
                                        </Swiper>
                                    )}
                                </div>
                            );
                        })}
                    </div>
                </div>
            </div>
        </div>
    );
}
