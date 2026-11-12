/**
 * 模块标题右侧区（更多链接 / 倒计时）显示推导。
 *
 * 拼团、秒杀模块的标题右侧区由 title.showMore（总开关）+ title.moreType（内容类型）共同决定。
 * moreType 是后加的字段，老数据没有，需按各模块原先对 showMore 的语义降级推导，保证行为不变。
 * 与 NexTrade-Admin 的 editor/utils/titleMore.ts 保持一致。
 */

/** 标题右侧内容类型 */
export const MORE_TYPE_LINK = 1;
export const MORE_TYPE_COUNTDOWN = 2;

/**
 * 老数据语义，决定 moreType 缺失时的推导方式：
 * - countdown：首页新套拼团/秒杀，showMore=1 原本就是"显示顶部倒计时"
 * - link：自定义页拼团，showMore=1 原本是"显示查看更多"
 * - seckillCustom：自定义页秒杀，showMore=0 时也无条件显示倒计时，仅 showMore=1 时额外显示查看更多
 */
export type TitleMoreLegacySemantic = "countdown" | "link" | "seckillCustom";

/** 标题右侧区可读的最小配置形状 */
interface TitleMoreSource {
    showMore?: number;
    moreType?: number;
    countdownLabel?: string;
    moreLink?: unknown;
}

export interface TitleMoreResult {
    /** 标题右侧区是否渲染 */
    visible: boolean;
    /** 渲染哪种内容 */
    mode: typeof MORE_TYPE_LINK | typeof MORE_TYPE_COUNTDOWN;
    /** 倒计时是否渲染 */
    showCountdown: boolean;
    /** 更多链接是否渲染 */
    showLink: boolean;
}

/** moreLink 在装修数据里是宽松类型，只取其中的展示文字 */
function readMoreLinkTitle(moreLink: unknown): string {
    if (moreLink && typeof moreLink === "object" && "title" in moreLink) {
        const title = (moreLink as { title?: unknown }).title;
        if (typeof title === "string") return title;
    }
    return "";
}

export function resolveTitleMore(title: TitleMoreSource | null | undefined, legacy: TitleMoreLegacySemantic): TitleMoreResult {
    const showMore = Number(title?.showMore) === 1;
    const moreType = Number(title?.moreType);

    // 配了 moreType 的新数据：两种内容互斥，由 moreType 单选
    if (moreType === MORE_TYPE_LINK || moreType === MORE_TYPE_COUNTDOWN) {
        return {
            visible: showMore,
            mode: moreType,
            showCountdown: showMore && moreType === MORE_TYPE_COUNTDOWN,
            showLink: showMore && moreType === MORE_TYPE_LINK
        };
    }

    // 老数据：倒计时无条件显示，showMore=1 时在其右侧「叠加」查看更多，两者不是互斥关系
    if (legacy === "seckillCustom") {
        return {
            visible: true,
            mode: showMore ? MORE_TYPE_LINK : MORE_TYPE_COUNTDOWN,
            showCountdown: true,
            showLink: showMore
        };
    }

    const mode = legacy === "link" ? MORE_TYPE_LINK : MORE_TYPE_COUNTDOWN;
    return {
        visible: showMore,
        mode,
        showCountdown: showMore && mode === MORE_TYPE_COUNTDOWN,
        showLink: showMore && mode === MORE_TYPE_LINK
    };
}

/**
 * 倒计时前缀文字：只认 countdownLabel，未配置就用模块默认文案。
 *
 * 不从 moreLink.title 降级：该字段在首页模块里存的是倒计时前缀，在自定义页模块里存的却是
 * 「查看更多」这类链接文案，两种语义无法从数据本身区分，读它会让倒计时渲染成
 * 「查看更多: 02:11:33」。首页存量若改过前缀，需在装修面板的「前缀文字」重填一次。
 */
export function resolveCountdownLabel(title: TitleMoreSource | null | undefined, fallback: string): string {
    return title?.countdownLabel?.trim() || fallback;
}

/** "更多"链接的展示文字 */
export function resolveMoreLinkTitle(title: TitleMoreSource | null | undefined, fallback: string): string {
    return readMoreLinkTitle(title?.moreLink).trim() || fallback;
}
