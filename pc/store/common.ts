import { normalizeRawImageConfig } from "@/utils/productRawImageDownload";
import { getNav, getCatFloor } from "@/api/common";
import { getBaseConfig, getThemeSettings } from "@/api/common";
import type { DecoratePageConfig } from "@/types/common/common.d.ts";
import type { ApiNavItemVO } from "@/types/decoration/decorationClientVo";
import { useThemeStore } from "@/store/theme";
import { MAP_PROVIDER_AMAP, resolveMapProvider } from "~/utils/map/provider";
// 翻译版本自动刷新
import versionManage from "~/utils/versionManage";
import { isOverseas } from "~/utils/util";

// 分类楼层节点：后端字段命名不统一（id/name 与 categoryId/categoryName 并存），故用宽松类型
type CategoryNodeLike = Record<string, any> & { children?: CategoryNodeLike[] };
type CatFloorLike = Record<string, any> & { catList?: CategoryNodeLike[] };

/**
 * 经营许可证展示开关仅接受 0/1；老版本接口缺失或返回非法值时默认开启，
 * 确保升级后不会意外隐藏商城资质信息。
 */
function normalizeBusinessLicenseEnabled(value: unknown): number {
    const normalized = typeof value === "string" && value.trim() !== "" ? Number(value) : value;
    return normalized === 0 || normalized === 1 ? Number(normalized) : 1;
}

/**
 * 解析可能是 JSON 字符串的 name 字段
 * 后端 pc-cat-floor 的 catList 中 name 可能是序列化的 categoryNames 对象，
 * 形如 '{"categoryTitleName":"你好","categoryLink":"cart"}'，需拆出展示名与自定义链接
 */
function extractNameAndLink(value: unknown): { displayName?: string; categoryLink?: string } {
    if (typeof value !== "string") return {};
    if (!value.startsWith("{")) return { displayName: value };
    try {
        const parsed = JSON.parse(value);
        const displayName = parsed.categoryTitleName ?? parsed.CategoryTitleName ?? undefined;
        const rawLink = parsed.categoryLink ?? parsed.CategoryLink;
        const categoryLink = typeof rawLink === "string" && rawLink.trim() ? rawLink.trim() : undefined;
        return { displayName, categoryLink };
    } catch {
        return { displayName: value };
    }
}

function normalizeCategoryNode(node: CategoryNodeLike | null | undefined): CategoryNodeLike | null | undefined {
    if (!node || typeof node !== "object") return node;
    const normalized: CategoryNodeLike = { ...node };
    if (normalized.categoryId === undefined && normalized.id !== undefined) {
        normalized.categoryId = normalized.id;
    }
    if (normalized.categoryName === undefined && normalized.name !== undefined) {
        normalized.categoryName = normalized.name;
    }
    // 兜底空串，避免 urlFormat 拿到 undefined 拼出脏链接
    if (normalized.seoLink === undefined) {
        normalized.seoLink = "";
    }
    if (Array.isArray(normalized.children)) {
        normalized.children = normalized.children.map(normalizeCategoryNode).filter((n): n is CategoryNodeLike => Boolean(n));
    }
    return normalized;
}

function normalizeCatFloor(catFloor: CatFloorLike[]): CatFloorLike[] {
    if (!Array.isArray(catFloor)) return [];
    return catFloor.map((floor) => {
        const out: CatFloorLike = { ...floor };
        if (Array.isArray(out.catList)) {
            out.catList = out.catList.map((cat: CategoryNodeLike) => {
                const catOut: CategoryNodeLike = { ...cat };
                const rawName = catOut.name ?? catOut.categoryName;
                const { displayName, categoryLink } = extractNameAndLink(rawName);
                if (displayName !== undefined && displayName !== rawName) {
                    catOut.name = displayName;
                    catOut.categoryName = displayName;
                }
                if (categoryLink) {
                    catOut.categoryLink = categoryLink;
                }
                if (catOut.name === undefined && catOut.categoryName === undefined) {
                    const titleName = catOut.categoryTitleName ?? catOut.CategoryTitleName;
                    if (typeof titleName === "string") {
                        catOut.name = titleName;
                        catOut.categoryName = titleName;
                    }
                }
                if (catOut.seoLink === undefined) catOut.seoLink = "";
                if (Array.isArray(catOut.children)) {
                    catOut.children = catOut.children.map(normalizeCategoryNode).filter((n): n is CategoryNodeLike => Boolean(n));
                }
                return catOut;
            });
        } else {
            out.catList = [];
        }
        return out;
    });
}

let navLoadingPromise: Promise<void> | null = null;

export const useCommonStore = defineStore("common", {
    state: () => {
        return {
            shopName: "", //商城名称
            shopTitle: "", //商城标题
            shopTitleSuffix: "", //商城标题后缀
            shopKeywords: "",
            shopDesc: "",
            shopLogo: "", //商城logo
            storageUrl: "", // 存储地址
            dollarSign: "", //货币符号
            dollarSignCn: "", //货币单位
            icoImg: "", //图标
            mainNav: [] as ApiNavItemVO[],
            topBarNav: [],
            bottomNav: [],
            sidebarNav: [],
            topRegion: [],
            navLoaded: false,
            allCate: null as CatFloorLike[] | null,
            allCateLoadingPromise: null as Promise<void> | null,
            // 分类楼层 icon 字体定义（后端动态 @font-face CSS 地址），供分类字体图标渲染
            catIcoCss: "" as string,
            decoratePageConfig: {
                headerStyle: 0
            } as DecoratePageConfig,
            defaultHeaderStyle: 1,
            personApplyEnabled: 0, //是否允许个人入驻
            provinceId: 0,
            canInvoice: 0,
            invoiceAdded: 0,
            autoRedirect: 1,
            h5Domain: "",
            shopIcpNo: "",
            shopIcpNoUrl: "",
            shop110No: "",
            shop110Link: "",
            shopCompanyTxt: "",
            companyAddress: "",
            kefuPhone: "",
            kefuTime: "",
            kefuAddress: "",
            isEnterprise: "",
            deCopyright: 0,
            poweredByStatus: 0,
            poweredBy: 0,
            defaultShopName: "",
            defaultLogisticsName: "",
            isOpenMobileAreaCode: 0,
            showMarketprice: 1,
            showSelledCount: 1,
            companyDataType: 1,
            companyDataTips: "",
            adminDomain: "", // 后台地址路径
            isIdentity: 0,
            isEnquiry: 0,
            closeOrder: 0,
            shopRegClosed: 0,
            useCoupon: 0,
            usePoints: 0,
            useSurplus: 0,
            memberWithdrawalEnabled: 0, // 会员余额提现开关，0-关闭，1-开通（D1107）
            memberWithdrawalReceiptMethod: null as number[] | null, // 提现到账方式 1银行卡 2支付宝 3微信（D1107）；null=后端未下发(老后端)按全显示兜底，[]=后台全部取消勾选
            memberWithdrawalDescription: "", // 会员余额提现说明 HTML（D1197）
            openWechatPcLogin: 0,
            wechatRegisterBindPhone: 0,
            openWechatOauth: 0,
            googleLoginOn: 0,
            facebookLoginOn: 0,
            version: "",
            translationVersion: 0,
            twitterLoginOn: 0,
            defaultTechSupport: "",
            poweredByLogo: "",
            openEmailRegister: 0,
            integralName: "",
            closeAuth: 0,
            bulkPurchase: 0,
            enableAttributeFilter: 0,
            // 默认 1 而非 0：老后端不下发这两个字段时，评价功能必须保持可见，不能因升级而消失（D1146）
            productCommentEnabled: 1,
            ...normalizeRawImageConfig(),
            anonymousCommentEnabled: 1,
            defaultAvatar: "",
            searchSelfStore: 0,
            /** 首页是否开启“搜门店”：1开启，0关闭 */
            indexSearchShop: 0,
            businessLicense: "",
            ediLicense: "",
            businessLicenseType: [] as string[],
            businessLicenseEnabled: 1,
            // 分类抽屉模式/样式（PC 首页 Hero 分类入口装修配置）
            categoryDrawerMode: 1,
            categoryDrawerStyle: 1,
            mapProvider: MAP_PROVIDER_AMAP as string,
            // 搜索页筛选分类缓存（PC 搜索页筛选组件回填）
            searchCate: [] as any[]
        };
    },
    getters: {
        pageTitle: (state) => {
            return state.shopTitleSuffix ? " - " + state.shopTitleSuffix : " ";
        }
    },
    actions: {
        async loadNav() {
            if (this.navLoaded) return;
            if (navLoadingPromise) {
                await navLoadingPromise;
                return;
            }
            navLoadingPromise = (async () => {
                try {
                    const result: any = await getNav();
                    this.mainNav = result.mainNav ?? [];
                    this.topBarNav = result.topBarNav ?? [];
                    this.bottomNav = result.bottomNav ?? [];
                    this.sidebarNav = result.sidebarNav ?? [];
                    this.navLoaded = true;
                } catch (error) {
                    console.error("[loadNav]", error);
                }
                navLoadingPromise = null;
            })();
            await navLoadingPromise;
        },
        /**
         * 获取全部分类（分类楼层）并做字段归一化
         * @description 数据来源 common/pc/getCatFloor；并发只发起一次。
         *  若 allCate 已被其它组件填充则直接复用，避免重复请求
         */
        async ensureAllCate(): Promise<void> {
            if (this.allCate !== null) return;
            if (this.allCateLoadingPromise) return await this.allCateLoadingPromise;

            this.allCateLoadingPromise = (async () => {
                try {
                    const result: any = await getCatFloor();
                    const catFloor = result?.catFloor ?? result?.data?.catFloor ?? [];
                    this.allCate = normalizeCatFloor(catFloor);
                    const ico = result?.icoDefinedCss ?? result?.data?.icoDefinedCss ?? "";
                    if (ico) this.catIcoCss = ico;
                } catch (error) {
                    console.error("[common.ensureAllCate] 获取分类楼层失败:", error);
                    this.allCate = null;
                } finally {
                    this.allCateLoadingPromise = null;
                }
            })();

            return await this.allCateLoadingPromise;
        },
        async loadBaseConfig(params = {}) {
            try {
                const result = await getBaseConfig(params);
                this.companyDataTips = result.companyDataTips;
                this.companyDataType = result.companyDataType;
                this.personApplyEnabled = result.personApplyEnabled;
                this.shopName = result.shopName;
                this.canInvoice = result.canInvoice;
                this.invoiceAdded = result.invoiceAdded;
                this.shopTitle = result.shopTitle;
                this.shopTitleSuffix = result.shopTitleSuffix ?? "";
                this.shopLogo = result.shopLogo;
                this.shopKeywords = result.shopKeywords;
                this.shopDesc = result.shopDesc;
                this.storageUrl = result.storageUrl;
                this.dollarSign = result.dollarSign;
                this.dollarSignCn = result.dollarSignCn;
                this.icoImg = result.icoImg;
                this.decoratePageConfig = result.decoratePageConfig || ({ headerStyle: 1 } as DecoratePageConfig);
                this.defaultHeaderStyle = result.defaultHeaderStyle || 1;
                this.autoRedirect = result.autoRedirect ?? 0;
                this.h5Domain = result.h5Domain;
                this.shopIcpNo = result.shopIcpNo;
                this.shopIcpNoUrl = result.shopIcpNoUrl;
                this.shop110No = result.shop110No;
                this.shop110Link = result.shop110Link;
                this.shopCompanyTxt = result.shopCompanyTxt;
                this.companyAddress = result.companyAddress;
                this.kefuPhone = result.kefuPhone;
                this.kefuTime = result.kefuTime;
                this.kefuAddress = result.kefuAddress;
                this.deCopyright = result.deCopyright;
                this.poweredByStatus = result.poweredByStatus;
                this.poweredBy = result.poweredBy;
                this.defaultShopName = result.defaultShopName;
                // i18n-ignore-next-line 兜底值作为 key 存入，展示处统一 $t(defaultLogisticsName)
                this.defaultLogisticsName = result.defaultLogisticsName ?? "商家配送";
                this.isOpenMobileAreaCode = result.isOpenMobileAreaCode;
                this.showMarketprice = result.showMarketprice;
                this.showSelledCount = result.showSelledCount;
                this.adminDomain = result.adminDomain;
                this.isIdentity = result.isIdentity;
                this.isEnquiry = result.isEnquiry;
                this.closeOrder = result.closeOrder;
                this.shopRegClosed = Number(result.shopRegClosed ?? 0);
                this.useCoupon = result.useCoupon;
                this.usePoints = result.usePoints;
                this.useSurplus = result.useSurplus;
                this.memberWithdrawalEnabled = Number(result.memberWithdrawalEnabled ?? 0);
                this.memberWithdrawalReceiptMethod = Array.isArray(result.memberWithdrawalReceiptMethod) ? result.memberWithdrawalReceiptMethod : null;
                this.memberWithdrawalDescription = result.memberWithdrawalDescription ?? "";
                this.openWechatPcLogin = result.openWechatPcLogin;
                this.wechatRegisterBindPhone = result.wechatRegisterBindPhone;
                this.openWechatOauth = result.openWechatOauth;
                this.googleLoginOn = result.googleLoginOn;
                this.facebookLoginOn = result.facebookLoginOn;
                this.twitterLoginOn = result.twitterLoginOn ?? 0;
                this.defaultTechSupport = result.defaultTechSupport;
                this.poweredByLogo = result.poweredByLogo;
                this.openEmailRegister = result.openEmailRegister;
                this.integralName = result.integralName;
                this.closeAuth = result.closeAuth;
                this.bulkPurchase = result.bulkPurchase;
                this.enableAttributeFilter = result.enableAttributeFilter;
                this.productCommentEnabled = Number(result.productCommentEnabled ?? 1);
                Object.assign(this, normalizeRawImageConfig(result));
                this.anonymousCommentEnabled = Number(result.anonymousCommentEnabled ?? 1);
                this.defaultAvatar = result.defaultAvatar;
                this.searchSelfStore = result.searchSelfStore ?? 0;
                this.indexSearchShop = result.indexSearchShop ?? 0;
                this.businessLicense = result.businessLicense;
                this.ediLicense = result.ediLicense;
                this.businessLicenseType = Array.isArray(result.businessLicenseType) ? result.businessLicenseType : [];
                this.businessLicenseEnabled = normalizeBusinessLicenseEnabled(result.businessLicenseEnabled);
                this.categoryDrawerMode = (result as any).categoryDrawerMode ?? 1;
                this.categoryDrawerStyle = (result as any).categoryDrawerStyle ?? 1;
                this.mapProvider = resolveMapProvider(result.mapProvider);
                // 翻译版本自动刷新
                this.version = (result as any).version ?? "";
                this.translationVersion = (result as any).translationVersion ?? 0;
                if (import.meta.client && isOverseas()) {
                    await versionManage();
                }
            } catch (error) {
                console.error(error);
            }
        },

        async loadThemeSettings() {
            const themeStore = useThemeStore();
            const themeColor = useCookie<any>("themeColor");
            try {
                const result = await getThemeSettings();
                let theme = JSON.parse(result?.themeStyle);
                if (themeColor.value) {
                    // 演示站才有的功能
                    themeStore.themeId = themeColor.value.themeId;
                    themeStore.themeColor = themeColor.value;
                } else {
                    let { themeId, check, ...themeColor } = theme;
                    // 正式部署都是通过接口返回主题色
                    themeStore.themeId = themeId;
                    themeStore.themeColor = themeColor; // 主题色~~~~~
                }
            } catch (error) {
                console.error(error);
            }
        }
    }
});
