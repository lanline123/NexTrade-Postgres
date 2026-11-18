/**
 * 小程序一键获取收货地址（Tigshop v5.8.62 对齐 · 批次D）
 *
 * - 仅微信小程序（TARO_ENV === "weapp"）可用：调用 Taro.chooseAddress() 读取用户
 *   在微信内保存的收货地址，免去逐项手填；H5 / 其他端一律返回 unsupported，
 *   UI 层也只在 weapp 渲染入口按钮。
 * - app.config.ts 已声明 requiredPrivateInfos: ["chooseAddress"] 与
 *   permission["scope.address"] 用途文案（隐私接口前置声明）。
 * - 微信返回的是省/市/区县「名称」，而现有地址保存接口要 region_ids 数字链：
 *   本模块按名称逐级匹配现有地区树（getRegionListByRegionId），匹配逻辑为纯函数
 *   （normalizeRegionName / findRegionMatch / matchWechatRegionIds），可单测。
 * - 匹配失败不阻断导入：regionMatched=false 时其余字段照常回填，UI 提示用户
 *   手动选择所在地区。
 * - 微信地址只有国内地址，countryCode 恒为 CN，不影响跨境地址逻辑。
 */
import Taro from "@tarojs/taro";

import { getCountryList, getRegionListByRegionId } from "@/api/region";

/** chooseAddress 原始返回（微信字段口径） */
export interface WechatAddressRaw {
    userName: string;
    telNumber: string;
    provinceName: string;
    cityName: string;
    countyName: string;
    detailInfo: string;
    postalCode: string;
    nationalCode?: string;
}

/** 标准化后的导入地址（与地址表单/保存接口字段对齐） */
export interface ImportedWechatAddress {
    consignee: string;
    mobile: string;
    provinceName: string;
    cityName: string;
    countyName: string;
    detailInfo: string;
    postalCode: string;
    /** 微信地址恒为国内地址 */
    countryCode: "CN";
    /** 名称匹配到的地区 ID 链（未匹配时为已匹配的前缀，可能为空） */
    regionIds: number[];
    /** 与 regionIds 对应的地区树标准名称 */
    regionNames: string[];
    /** 省/市/区县是否全部匹配成功 */
    regionMatched: boolean;
}

export type ImportWechatAddressResult =
    | { status: "ok"; address: ImportedWechatAddress }
    | { status: "cancelled" }
    | { status: "denied" }
    | { status: "unsupported" }
    | { status: "error"; message: string };

/** 当前端是否支持微信地址导入（仅 weapp） */
export const isWechatAddressImportSupported = (): boolean => process.env.TARO_ENV === "weapp";

/** 打开小程序设置页（用户拒绝授权后，引导其手动开启 scope.address） */
export const openWechatAddressSetting = (): void => {
    if (!isWechatAddressImportSupported()) return;
    void Taro.openSetting({});
};

// ──────────────────────────── 地区名称匹配（纯函数） ────────────────────────────

/** 地区树节点最小形态（与 api/region 的 ChildRegion 兼容） */
export interface RegionNodeLike {
    regionId: number;
    regionName: string;
}

/**
 * 行政区划后缀（长串在前，剥离时优先匹配最长后缀）。
 * 微信名称与地区树名称的差异主要在后缀有无（如「广东省」vs「广东」），
 * 两侧用同一规则归一后比较即可对齐。
 */
const REGION_NAME_SUFFIXES = [
    "维吾尔自治区",
    "壮族自治区",
    "回族自治区",
    "特别行政区",
    "自治区",
    "自治州",
    "自治县",
    "地区",
    "新区",
    "省",
    "市",
    "盟",
    "县",
    "区"
];

/**
 * 地区名归一化：去空白后反复剥离行政区划后缀。
 * 例：广东省→广东、内蒙古自治区→内蒙古、广西壮族自治区→广西、
 * 深圳市→深圳、朝阳区→朝阳、浦东新区→浦东。
 * 至少保留 1 个字符，避免把单字名剥空。
 */
export function normalizeRegionName(name: string): string {
    let result = String(name || "")
        .trim()
        .replace(/\s+/g, "");
    let changed = true;
    while (changed && result.length > 1) {
        changed = false;
        for (const suffix of REGION_NAME_SUFFIXES) {
            if (result.length > suffix.length && result.endsWith(suffix)) {
                result = result.slice(0, result.length - suffix.length);
                changed = true;
                break;
            }
        }
    }
    return result;
}

/**
 * 在一层地区节点中按名称找目标节点：
 * 先比原名精确相等，再比归一化相等，最后比归一化前缀包含（兜底
 * 「临沂」vs「临沂市」这类已归一仍差一截的写法）。
 */
export function findRegionMatch(nodes: RegionNodeLike[], targetName: string): RegionNodeLike | undefined {
    const target = String(targetName || "").trim();
    if (!target || !nodes.length) return undefined;
    const exact = nodes.find((node) => node.regionName === target);
    if (exact) return exact;
    const targetNorm = normalizeRegionName(target);
    if (!targetNorm) return undefined;
    const normalized = nodes.find((node) => normalizeRegionName(node.regionName) === targetNorm);
    if (normalized) return normalized;
    if (targetNorm.length >= 2) {
        return nodes.find((node) => {
            const nodeNorm = normalizeRegionName(node.regionName);
            return nodeNorm.length >= 2 && (nodeNorm.startsWith(targetNorm) || targetNorm.startsWith(nodeNorm));
        });
    }
    return undefined;
}

/** 逐级取子地区节点的函数（注入点：默认走现有地区接口，单测可传假数据） */
export type FetchRegionChildren = (parentId: number | string) => Promise<RegionNodeLike[]>;

export interface RegionMatchOutcome {
    regionIds: number[];
    /** 地区树中的标准名称（与 regionIds 一一对应） */
    regionNames: string[];
    /** 微信省/市/区县名称是否全部被消费（跳级匹配也算消费） */
    fullyMatched: boolean;
}

/**
 * 把微信省/市/区县名称逐级匹配为 region_ids。
 *
 * - 从 rootId（中国根节点）的子级开始逐级向下匹配；
 * - 直辖市处理：省名与市名同名时，若某一级名称在当前子级中找不到，
 *   会尝试用下一级名称在同一批子级中匹配（跳级），例如北京市的
 *   子级直接是区县时，市名「北京市」被跳过、区县照常命中；
 * - 任一级匹配失败即停，已匹配的前缀保留返回，fullyMatched=false。
 */
export async function matchWechatRegionIds(
    names: { provinceName?: string; cityName?: string; countyName?: string },
    rootId: number | string,
    fetchChildren: FetchRegionChildren
): Promise<RegionMatchOutcome> {
    const targets = [names.provinceName, names.cityName, names.countyName]
        .map((name) => String(name || "").trim())
        .filter(Boolean);
    const regionIds: number[] = [];
    const regionNames: string[] = [];
    if (!targets.length) return { regionIds, regionNames, fullyMatched: false };

    let parentId: number | string = rootId;
    let index = 0;
    while (index < targets.length) {
        let children: RegionNodeLike[] = [];
        try {
            children = (await fetchChildren(parentId)) || [];
        } catch {
            break;
        }
        if (!children.length) break;

        const matched = findRegionMatch(children, targets[index]!);
        if (matched) {
            regionIds.push(matched.regionId);
            regionNames.push(matched.regionName);
            parentId = matched.regionId;
            index += 1;
            continue;
        }
        // 跳级：当前级名称找不到时，用下一级名称在同一批子级中试匹配
        // （直辖市省/市同名、地区树缺市级节点等情形）。
        if (index + 1 < targets.length) {
            const skipped = findRegionMatch(children, targets[index + 1]!);
            if (skipped) {
                regionIds.push(skipped.regionId);
                regionNames.push(skipped.regionName);
                parentId = skipped.regionId;
                index += 2;
                continue;
            }
        }
        break;
    }
    return { regionIds, regionNames, fullyMatched: index >= targets.length && regionIds.length > 0 };
}

// ──────────────────────────── 地区树取数（默认实现） ────────────────────────────

const fetchRegionChildrenDefault: FetchRegionChildren = async (parentId) => {
    const result: any = await getRegionListByRegionId(parentId);
    const rows: any[] = Array.isArray(result) ? result : Array.isArray(result?.data) ? result.data : [];
    return rows
        .map((row) => ({ regionId: Number(row?.regionId), regionName: String(row?.regionName || "") }))
        .filter((row) => Number.isFinite(row.regionId) && row.regionId > 0 && row.regionName);
};

/** 取中国根地区节点 ID（国家列表中 standardCode=CN 的行）；失败返回 undefined */
export async function resolveChinaRootRegionId(): Promise<number | undefined> {
    try {
        const result: any = await getCountryList();
        const rows: any[] = Array.isArray(result) ? result : Array.isArray(result?.data) ? result.data : [];
        const china = rows.find((row) => String(row?.standardCode || "").toUpperCase() === "CN");
        const regionId = Number(china?.regionId);
        return Number.isFinite(regionId) && regionId > 0 ? regionId : undefined;
    } catch {
        return undefined;
    }
}

/** 对微信地址做地区匹配（取根节点 + 逐级匹配）；任何失败都降级为未匹配，不抛错 */
export async function matchImportedWechatRegion(raw: WechatAddressRaw): Promise<RegionMatchOutcome> {
    const rootId = await resolveChinaRootRegionId();
    if (!rootId) return { regionIds: [], regionNames: [], fullyMatched: false };
    return matchWechatRegionIds(raw, rootId, fetchRegionChildrenDefault);
}

// ──────────────────────────── 导入主流程 ────────────────────────────

/**
 * 调起微信地址选择并返回标准化结果。
 * - 用户取消（errMsg 含 cancel）→ cancelled，UI 应静默；
 * - 用户拒绝授权（errMsg 含 auth deny / denied）→ denied，UI 应引导 openSetting；
 * - 非 weapp → unsupported。
 */
export async function importWechatAddress(): Promise<ImportWechatAddressResult> {
    if (!isWechatAddressImportSupported()) return { status: "unsupported" };

    let raw: WechatAddressRaw;
    try {
        raw = (await (Taro as any).chooseAddress()) as WechatAddressRaw;
    } catch (error: any) {
        const message = String(error?.errMsg || error?.message || "");
        if (/cancel/i.test(message)) return { status: "cancelled" };
        if (/auth\s*deny|denied|deny/i.test(message)) return { status: "denied" };
        return { status: "error", message: message || "chooseAddress 调用失败" };
    }
    if (!raw || typeof raw !== "object") return { status: "error", message: "chooseAddress 未返回地址" };

    const outcome = await matchImportedWechatRegion(raw).catch(
        (): RegionMatchOutcome => ({ regionIds: [], regionNames: [], fullyMatched: false })
    );

    return {
        status: "ok",
        address: {
            consignee: String(raw.userName || "").trim(),
            mobile: String(raw.telNumber || "").trim(),
            provinceName: String(raw.provinceName || "").trim(),
            cityName: String(raw.cityName || "").trim(),
            countyName: String(raw.countyName || "").trim(),
            detailInfo: String(raw.detailInfo || "").trim(),
            postalCode: String(raw.postalCode || "").trim(),
            countryCode: "CN",
            regionIds: outcome.regionIds,
            regionNames: outcome.regionNames,
            regionMatched: outcome.fullyMatched
        }
    };
}
