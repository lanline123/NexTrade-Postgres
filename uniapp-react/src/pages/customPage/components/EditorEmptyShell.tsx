import { useEffect, useRef, useState, type ReactNode } from "react";
import { View, Text } from "@tarojs/components";

import "./EditorEmptyShell.scss";

/**
 * 编辑器空模块占位壳（编辑器嵌入预览专用，2026-11-21）。
 *
 * 背景：H5 编辑器默认真实渲染（iframe 嵌入 decoratePreview），分发器按前台
 * 口径渲染模块——模块没填内容时渲染器返回 null（如 imageAd 空 picList）或
 * 只剩 frame padding（如 imageSquareAd 空 picList 约 0-20px），编辑器按
 * decorate-layout 如实上报 ~0 高：模块在画布上不可见、不可点，相邻模块贴
 * 在一起（用户 2026-11-21 在页面 10106 实测「模块内没填内容，两个模块之间
 * 的高度还是会塌掉」）。2026-11-20 的空态清扫（7d6568d）只修了兼容渲染面
 * （admin Preview 组件），真实渲染面当时明确不在范围内——本壳补这一面。
 * 原版编辑器各 Preview 对空态恒有 .module-ad-empty 占位块（约 224 设计 px，
 * 见原版 imageAd/Preview.vue 等），空模块在编辑器里必须有可见、可点的占位
 * 高度，此壳即真实渲染面的等价物（来源标注：原版有·我方补齐真实渲染面）。
 *
 * 机制：只量测、不猜数据。内容渲染后量其真实高度，多轮确认判空才挂占位
 * （≤4px 直接判空；≤14px 的近空带还须子树无图无字，覆盖只剩 frame padding
 * 的魔方类空态；分割线/空白辅助豁免近空带）；内容一旦出现（用户在右侧面板
 * 填入、或异步取数落地）立即撤占位；内容被清空后经巡检复挂。仅当分发器收到 moduleEmptyPlaceholder 时才挂壳（仅
 * decoratePreview 传入），前台页面不传——不挂壳、DOM 与行为逐字不变。
 * 已有单模块先例：imageHotarea 的 inEditorEmbed 空态占位（2026-11-20），
 * 本壳把同一口径收口到分发器共享层，全模块统一生效。
 */

/** 模块类型到中文名（与 admin 编辑器模块库 label 同源，供占位文案） */
export const MODULE_TYPE_LABELS: Record<string, string> = {
    account: "账号",
    adImage: "广告图",
    "ad-image": "广告图",
    article: "文章列表",
    banner: "首焦轮播",
    bannerA2: "首焦轮播A2",
    catNav: "分类导航",
    categoryNav: "类目导航",
    categoryNavA1: "类目导航A1",
    categoryNavA2: "类目导航A2",
    categoryNavA3: "类目导航A3",
    categoryRec: "分类推荐",
    categoryRecA1: "分类推荐A1",
    comboSuggestions: "组合推荐",
    commonTitle: "通用标题",
    countdown: "倒计时",
    coupon: "优惠券",
    customProduct: "个性商品",
    fixedFunctionalArea: "固定功能区",
    "fixed-functional-area": "固定功能区",
    groupSliding: "分组滑动",
    groupon: "拼团",
    imageAd: "图片广告1",
    imageAd2: "图片广告2",
    imageHotarea: "热图绘制",
    imageNav: "图文导航",
    imageShowcase: "图片橱窗",
    imageSquareAd: "图片魔方",
    imageTextComponent: "图文介绍",
    limitedSpecial: "限时特惠",
    live: "小程序直播",
    mixedNotice: "公告混排",
    mixedProductDisplay: "商品混排",
    myService: "我的服务",
    "my-service": "我的服务",
    myWallet: "我的钱包",
    "my-wallet": "我的钱包",
    myWalletV2: "我的钱包2",
    "my-wallet-v2": "我的钱包2",
    navbar: "页面导航",
    newcomerCombo: "新人二重礼",
    notice: "公告",
    noticeA2: "商城公告2",
    noticeA3: "商城公告3",
    orderEntry: "我的订单",
    "order-entry": "我的订单",
    orderEntryV2: "我的订单2",
    "order-entry-v2": "我的订单2",
    product: "商品",
    product2: "商品2",
    productA1: "商品A1",
    productCarousel: "商品轮播",
    productGrouping: "商品分组",
    productSlide: "商品滑动",
    searchBar: "商品搜索",
    searchBar2: "商品搜索2",
    seckill: "秒杀",
    seckill2: "秒杀2",
    shopInfo: "店铺信息",
    shopTips: "商品提示",
    slideLargeImage: "大图滑动",
    slidingImage: "小图滑动",
    specialOffer: "天天特价",
    storeNav: "门店导航",
    storeOrderCard: "点单卡片",
    tagNav: "标签导航",
    titleAd: "文本标题",
    userInfo: "用户信息",
    "user-info": "用户信息",
    userInfoV2: "用户信息2",
    "user-info-v2": "用户信息2",
    userInfo3: "用户信息3",
    "user-info3": "用户信息3",
    videoNumber: "视频号",
    whiteBlank: "空白辅助",
    whiteLine: "分割线"
};

/** 内容高（px）不超过此值直接视为空（0 高或仅剩亚像素边框残量） */
const EMPTY_MAX_HEIGHT = 4;
/**
 * 近空带（px）：高度在此带内、且内容子树里没有任何可见内容元素（无图片/
 * 画布/视频/输入框、无非空文本）时同样视为空——覆盖 imageSquareAd 这类
 * 空时只剩 frame padding（默认上下各 5 设计 px ≈ 10px）的模块。
 * 分割线/空白辅助的「内容」本身就是一条线/一段空白（无图无字但属真实
 * 内容），列入豁免，永不按近空带处理。
 */
const NEAR_EMPTY_MAX_HEIGHT = 14;
const NEAR_EMPTY_EXEMPT_TYPES = new Set(["whiteLine", "whiteBlank"]);
/** 首轮判定节奏（ms）：覆盖懒加载分块到达与异步取数落地的窗口 */
const FIRST_CHECK_DELAYS = [350, 900, 1800, 3000];
/** 巡检间隔（ms）：占位挂上后内容出现即撤、内容被清空后复挂 */
const WATCH_INTERVAL = 1200;
/** 巡检中连续多少次量到空才复挂占位（避开切换/动画瞬间误报） */
const WATCH_EMPTY_TICKS = 3;

interface Props {
    moduleType: string;
    /** 模块数据对象：换新引用即重新判定（与 ModuleErrorBoundary 的 resetKey 同口径） */
    resetKey?: unknown;
    children: ReactNode;
}

export default function EditorEmptyShell({ moduleType, resetKey, children }: Props) {
    const contentRef = useRef<any>(null);
    const [empty, setEmpty] = useState(false);
    const emptyRef = useRef(false);

    useEffect(() => {
        let cancelled = false;
        const timers: ReturnType<typeof setTimeout>[] = [];
        let zeroTicks = 0;

        const applyEmpty = (v: boolean) => {
            if (cancelled || emptyRef.current === v) return;
            emptyRef.current = v;
            setEmpty(v);
        };
        const measureIsEmpty = (): boolean => {
            const el = contentRef.current as HTMLElement | undefined;
            if (!el || typeof el.getBoundingClientRect !== "function") return true;
            const height = el.getBoundingClientRect().height;
            if (height <= EMPTY_MAX_HEIGHT) return true;
            if (height > NEAR_EMPTY_MAX_HEIGHT || NEAR_EMPTY_EXEMPT_TYPES.has(moduleType)) return false;
            if (el.querySelector("img, canvas, video, input, textarea")) return false;
            return (el.textContent ?? "").trim().length === 0;
        };

        FIRST_CHECK_DELAYS.forEach((delay, i) => {
            timers.push(
                setTimeout(() => {
                    if (cancelled) return;
                    if (!measureIsEmpty()) {
                        zeroTicks = 0;
                        applyEmpty(false);
                    } else if (i === FIRST_CHECK_DELAYS.length - 1) {
                        applyEmpty(true);
                    }
                }, delay)
            );
        });
        const interval = setInterval(() => {
            if (cancelled) return;
            if (!measureIsEmpty()) {
                zeroTicks = 0;
                applyEmpty(false);
            } else {
                zeroTicks += 1;
                if (zeroTicks >= WATCH_EMPTY_TICKS) applyEmpty(true);
            }
        }, WATCH_INTERVAL);

        return () => {
            cancelled = true;
            timers.forEach(clearTimeout);
            clearInterval(interval);
        };
    }, [resetKey, moduleType]);

    const label = MODULE_TYPE_LABELS[moduleType] || "该模块";
    return (
        <View className="editor-empty-shell">
            <View ref={contentRef} className="editor-empty-shell__content">
                {children}
            </View>
            {empty ? (
                <View className="editor-empty-shell__placeholder">
                    <View className="editor-empty-shell__box" />
                    <Text className="editor-empty-shell__text">{label} · 暂无内容</Text>
                    <Text className="editor-empty-shell__hint">选中模块后在右侧面板添加内容</Text>
                </View>
            ) : null}
        </View>
    );
}
