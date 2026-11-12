// 跨境独有：内部以 isOverseas() && isDemo() 守卫，非跨境站直接 return
/**
 * 三方脚本受同意约束的懒加载入口（仅客户端）
 *
 * 使用规范：未来接入任何第三方分析 / 广告 / 客服 SDK 时，都不要直接 import
 * 或者直接写 <script>，而是在本文件里按下面示例新增一个加载函数，并按
 * 对应类别的 hasConsent 触发。
 *
 * 当前项目尚未接入任何外部 SDK，因此本文件保留为骨架，仅做监听。
 */
import { watch } from "vue";
import { isOverseas, isDemo } from "@/utils/util";

export default defineNuxtPlugin(() => {
    // 仅在海外演示站走该流程
    if (!isOverseas() || !isDemo()) return;

    const { hasConsent, detail } = useConsent();

    const loaded = {
        analytics: false,
        marketing: false,
        functional: false
    };

    const loadAnalytics = () => {
        if (loaded.analytics) return;
        loaded.analytics = true;
        // TODO: 接入分析类 SDK 时在此初始化，例如：
        // const s = document.createElement("script");
        // s.async = true;
        // s.src = "https://www.googletagmanager.com/gtag/js?id=GA_MEASUREMENT_ID";
        // document.head.appendChild(s);
        // window.dataLayer = window.dataLayer || [];
        // window.gtag = function () { window.dataLayer.push(arguments); };
        // window.gtag("js", new Date());
        // window.gtag("config", "GA_MEASUREMENT_ID");
    };

    const loadMarketing = () => {
        if (loaded.marketing) return;
        loaded.marketing = true;
        // TODO: 接入广告 / 像素类 SDK 时在此初始化，例如 Meta Pixel、TikTok Pixel
    };

    const loadFunctional = () => {
        if (loaded.functional) return;
        loaded.functional = true;
        // TODO: 接入功能性三方 SDK 时在此初始化，例如客服聊天窗口
    };

    const apply = () => {
        if (hasConsent("analytics")) loadAnalytics();
        if (hasConsent("marketing")) loadMarketing();
        if (hasConsent("functional")) loadFunctional();
    };

    apply();

    // 用户后续在弹窗里调整选项时，自动按需补加载
    watch(
        () => detail.value,
        () => {
            apply();
        },
        { deep: true }
    );
});

