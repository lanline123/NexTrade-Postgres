import { getLang, getLangList, getDefaultLang } from "@/api/common";
import type { LangListItem } from "@/types/common/common.d.ts";
import { isO2o } from "@/utils/util";
import { LOCALE_DOMAINS_STATE_KEY } from "~/utils/cookieDomain";
import { useRequestLocaleCode } from "~/composables/useRequestLocaleCode";

export const useI18nStore = defineStore("i18n", () => {
    const langList = ref<LangListItem[]>([]);

    const { t, locale, setLocale, setLocaleMessage, getLocaleMessage } = useI18n();

    // 必须保持 host-only：一旦跨域名共享，init 会走进 fetchLangData 分支沿用旧语言，域名判定失效
    const XLocaleCode = useCookie("X-Locale-Code");

    // SSR 请求内的统一语言源：已有 Cookie 访问先以 Cookie 初始化，首访会在 setXLocaleCode 时写入最终语言。
    const requestLocaleCode = useRequestLocaleCode();
    if (!requestLocaleCode.value && XLocaleCode.value) {
        requestLocaleCode.value = XLocaleCode.value;
    }

    const currentLanguage = useCookie("currentLanguage");

    const reacquireLanguage = useCookie("reacquireLanguage", {
        default: () => false // 默认值为 false
    });

    // 供跨域名共享的 Cookie 推导父域使用，见 utils/cookieDomain
    const localeDomains = useState<string[]>(LOCALE_DOMAINS_STATE_KEY, () => []);

    // SSR/多标签页语言切换失效修复（默认分支 bug 修复）
    const init = async () => {
        // 语言显示名与语言码必须保持一致。历史版本在跨域切换时可能只留下
        // 其中一个 Cookie，导致界面显示“中文”但实际加载 en-US 语言包。
        const languageText = String(currentLanguage.value || "").toLowerCase();
        const isChineseLanguage = /中文|简体|zh/.test(languageText);
        const isEnglishLanguage = /英文|英语|english|英語|en/.test(languageText);
        if (
            XLocaleCode.value &&
            ((isChineseLanguage && !XLocaleCode.value.toLowerCase().startsWith("zh")) ||
                (isEnglishLanguage && !XLocaleCode.value.toLowerCase().startsWith("en")))
        ) {
            const correctedLocale = isChineseLanguage ? "zh-CN" : "en-US";
            setXLocaleCode(correctedLocale);
            if (import.meta.client) localStorage.removeItem("cacheLang");
        }

        const langListPromise = fetchLangList();

        // 没有配置语言列表代表当前站点使用中文单语言模式（o2o）。
        // 此时忽略浏览器/历史 Cookie 中的语言，避免旧 en-US 缓存污染页面。
        if (isO2o()) {
            await langListPromise;
        }
        if (isO2o() && langList.value.length === 0) {
            setXLocaleCode("zh-CN");
            currentLanguage.value = "中文";
            if (import.meta.client) localStorage.removeItem("cacheLang");
            await fetchLangData("zh-CN");
            return;
        }

        if (import.meta.client) {
            const data = localStorage.getItem("cacheLang");
            // cacheLang 只在当前域名已经有明确语言 Cookie 时复用。
            // 跨语言域名/新域名首访时 Cookie 为空，旧缓存可能来自另一种语言，
            // 直接复用会让中文页面实际使用 en-US，出现中英混排。
            if (data && !reacquireLanguage.value && XLocaleCode.value) {
                try {
                    const { localeCode, langData, language } = JSON.parse(data);
                    // SSR 场景下：服务端优先依据 Cookie 渲染语言；若本地缓存语言与 Cookie 不一致，
                    // 客户端 hydration 时不能用旧 cacheLang 覆盖服务端语言，否则会表现为“语言切换无效”。
                    if (XLocaleCode.value && XLocaleCode.value !== localeCode) {
                        localStorage.removeItem("cacheLang");
                    } else {
                        setXLocaleCode(localeCode);
                        setLangData(localeCode, langData, false);
                        currentLanguage.value = language;
                        return;
                    }
                } catch (e) {
                    // cacheLang 解析失败时，直接丢弃，走正常拉取流程
                    localStorage.removeItem("cacheLang");
                }
            }
        }

        if (!XLocaleCode.value) {
            // SSR 以请求头为准，客户端只在没有 SSR 请求语言时使用浏览器语言，
            // 避免 navigator.language 无条件覆盖中文站点的语言判定。
            const requestLocale = useRequestHeaders()["accept-language"]?.split(",")[0] || "";
            const clientLocale = import.meta.client && typeof navigator !== "undefined" ? navigator.language : "";
            setXLocaleCode(requestLocale || clientLocale || "zh-CN");
            await fetchDefaultLangConfig();
        } else {
            await fetchLangData(XLocaleCode.value);
        }
    };

    // 获取语言列表
    const fetchLangList = async () => {
        try {
            const result = await getLangList();
            const normalizedResult = Array.isArray(result) ? result : [];
            langList.value = normalizedResult;
            localeDomains.value = normalizedResult.map((item) => item.domain?.trim() || "").filter(Boolean);
        } catch (error: any) {
            console.error(error.message);
        }
    };

    const fetchLangData = async (localeCode: string) => {
        let langData: Record<string, string> = {};
        try {
            langData = (await getLang(localeCode)) || {};
        } catch (error) {
            console.error(error);
        } finally {
            // 即使后端语言接口暂时不可用，也要激活对应的本地语言包，
            // 避免 i18n locale 停留在错误语言或直接显示原始 key。
            setXLocaleCode(localeCode);
            setLangData(localeCode, langData);
            // 在 SSR 下如果服务端提前把 reacquireLanguage 置回 false，
            // 客户端 hydration 阶段可能误用旧的 cacheLang 覆盖服务端语言。
            // 因此仅在客户端完成拉取后再清理该标记，确保 localStorage 会被更新。
            if (import.meta.client) {
                reacquireLanguage.value = false;
            }
        }
    };

    const fetchDefaultLangConfig = async () => {
        try {
            const result = await getDefaultLang(XLocaleCode.value!);

            // 语言配置未初始化时接口可能返回 null。保留静态语言包并使用当前
            // 已确定的语言，不能因空响应把页面留在错误的语言状态。
            if (!result?.localeCode) {
                // 当前站点未配置可用语言时，产品的默认语言是中文；不能沿用
                // 浏览器/历史 Cookie 中的 en-US，否则 o2o 首屏会出现中英混排。
                const fallbackLocale = "zh-CN";
                setXLocaleCode(fallbackLocale);
                currentLanguage.value = currentLanguage.value || "中文";
                await fetchLangData(fallbackLocale);
                return;
            }

            currentLanguage.value = result.language;

            await fetchLangData(result.localeCode);
        } catch (error) {
            console.error(error);
        }
    };

    const setXLocaleCode = (localeCode: string) => {
        XLocaleCode.value = localeCode;
        requestLocaleCode.value = localeCode;
    };

    const setReacquireLanguage = (value: boolean) => {
        reacquireLanguage.value = value;
    };

    // 设置语言数据
    const setLangData = (localeCode: string, langData: any, needCache = true) => {
        setLocale(localeCode);
        setLocaleMessage(localeCode, { ...getLocaleMessage(localeCode), ...langData });
        // localStorage 仅存在于客户端，SSR 侧写入会抛错
        if (needCache && import.meta.client) {
            const cacheLang = JSON.stringify({
                localeCode,
                langData,
                language: currentLanguage.value
            });
            localStorage.setItem("cacheLang", cacheLang);
        }
    };

    // 清空语言缓存（后台翻译版本变更时由 versionManage 调用）
    const clearCache = () => {
        if (import.meta.client) {
            localStorage.removeItem("cacheLang");
        }
        reacquireLanguage.value = true;
        langList.value = [];
        currentLanguage.value = "";
    };

    // 返回响应式属性和方法
    return {
        langList,
        reacquireLanguage,
        XLocaleCode,
        init,
        fetchLangData,
        fetchLangList,
        fetchDefaultLangConfig,
        setXLocaleCode,
        setReacquireLanguage,
        clearCache
    };
});
