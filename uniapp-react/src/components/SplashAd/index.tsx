import { useEffect, useRef, useState } from "react";
import { CoverView, Image, Video, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import { imageFormat, urlFormat } from "@/utils/format";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    adType: "image" | "video";
    adUrl: string;
    duration?: number;
    redirectType?: string;
    redirectUrl?: string;
    /** 对应旧 emit("close") */
    onClose?: () => void;
}

/**
 * 开屏广告（自旧 components/SplashAd/index.vue 131 行移植）。
 *
 * 行为逐条对齐旧实现：
 * - 素材加载完成（image @load / video loadedmetadata）后才开始倒计时并露出"跳过"；
 * - 倒计时归零自动关闭；video 播完（ended）也关闭（故 video 不设 loop，循环播放不会触发 ended）；
 * - 兜底：素材加载失败（image/video @error）立即关闭；挂载 10 秒仍未加载完成也自动关闭，
 *   避免坏素材/慢网络把用户卡在黑屏开屏上回不了首页（旧实现无此兜底）；
 * - 点击素材：redirectType == "1" 时按 redirectUrl 跳转并关闭，其余类型只打日志（旧代码注释掉的分支同样保留为注释语义）；
 * - 旧 close-btn 的端差异（weapp 左上 / h5 右上）改为 TARO_ENV 下不同 className。
 */
export default function SplashAd({ adType, adUrl, duration = 3, redirectType, redirectUrl, onClose }: Props) {
    const [show, setShow] = useState(true);
    const [loaded, setLoaded] = useState(false);
    const [countdown, setCountdown] = useState(() => {
        const d = Number(duration);
        return Number.isFinite(d) && d > 0 ? d : 3;
    });
    const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
    const watchdogRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const closedRef = useRef(false);
    const loadedRef = useRef(false);

    const clearTimer = () => {
        if (timerRef.current) {
            clearInterval(timerRef.current);
            timerRef.current = null;
        }
        if (watchdogRef.current) {
            clearTimeout(watchdogRef.current);
            watchdogRef.current = null;
        }
    };

    useEffect(() => {
        // 兜底看门狗：素材迟迟不加载完成（坏链/慢网）时自动关闭，不卡黑屏
        watchdogRef.current = setTimeout(() => {
            if (!loadedRef.current) closeAd();
        }, 10000);
        return () => clearTimer();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    const closeAd = () => {
        if (closedRef.current) return;
        closedRef.current = true;
        setShow(false);
        clearTimer();
        onClose?.();
    };

    const onLoaded = () => {
        if (loadedRef.current || closedRef.current) return;
        loadedRef.current = true;
        if (watchdogRef.current) {
            clearTimeout(watchdogRef.current);
            watchdogRef.current = null;
        }
        setLoaded(true);
        timerRef.current = setInterval(() => {
            setCountdown((prev) => {
                const next = prev - 1;
                if (next <= 0) {
                    closeAd();
                }
                return next;
            });
        }, 1000);
    };

    // 素材加载失败兜底：直接关闭开屏放行首页，绝不黑屏卡住
    const onLoadError = () => {
        closeAd();
    };

    const handleAdClick = () => {
        if (redirectType == "1") {
            Taro.navigateTo({ url: urlFormat(redirectUrl || "") });
            closeAd();
        }
    };

    const isWeapp = process.env.TARO_ENV === "weapp";
    const closeBtnClass = `close-btn${isWeapp ? " close-btn--weapp" : " close-btn--h5"}`;

    if (!show) return null;

    return (
        <View className="splash-ad">
            {adType === "image" ? (
                <Image className="ad-img" src={imageFormat(adUrl)} mode="aspectFill" onLoad={onLoaded} onError={onLoadError} onClick={handleAdClick} />
            ) : (
                <Video
                    className="ad-video"
                    src={imageFormat(adUrl)}
                    objectFit="cover"
                    autoplay
                    controls={false}
                    showCenterPlayBtn={false}
                    showPlayBtn={false}
                    showFullscreenBtn={false}
                    enableProgressGesture={false}
                    muted
                    onEnded={closeAd}
                    onError={onLoadError}
                    onLoadedMetaData={onLoaded}
                    onClick={handleAdClick}
                >
                    {loaded && (
                        <CoverView className={`${closeBtnClass} close-btn--cover`} onClick={(e) => { e.stopPropagation(); closeAd(); }}>
                            {countdown} {t("跳过")}
                        </CoverView>
                    )}
                </Video>
            )}
            {adType !== "video" && loaded && (
                <View className={closeBtnClass} onClick={(e) => { e.stopPropagation(); closeAd(); }}>
                    {countdown} {t("跳过")}
                </View>
            )}
        </View>
    );
}
