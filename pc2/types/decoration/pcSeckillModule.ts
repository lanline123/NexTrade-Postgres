/** PC 秒杀装修模块（activity/pcSeckill 与 decorate 包装组件共用） */

export interface PcSeckillTitleDecor {
    showTitle?: number;
    titleStyle?: number;
    titleText?: string;
    descText?: string;
    descColor?: string;
    titleColor?: string;
    titleRadius?: number;
    titlePaddingTop?: number;
    titlePaddingBottom?: number;
    titleBackground?: string;
    titleBackground2?: string;
    titleBackgroundPic?: { picUrl?: string } | null;
    showMore?: number;
    /** 标题右侧内容类型：1=显示更多链接，2=显示倒计时。缺失时按老语义推导（见 decorate/titleMore.ts） */
    moreType?: number;
    /** 倒计时前缀文字（与"更多"的 moreLink.title 相互独立） */
    countdownLabel?: string;
    moreLink?: unknown;
    moreColor?: string;
}

export interface PcSeckillFrameDecor {
    boxPadding?: number;
    boxPaddingTop?: number;
    boxPaddingBottom?: number;
    leftAndRightPadding?: number;
    innerPadding?: number;
    backgroundColor?: string;
    boxRadius?: number;
}

/** 装修后台下发的秒杀模块 JSON（字段随版本扩展，一律可选） */
export interface PcSeckillModuleDecor {
    style?: number;
    goodsStyle?: number;
    goodsRadioStyle?: number;
    textAlign?: number;
    textWeight?: number;
    goodsNameRow?: number;
    goodsNamePadding?: number;
    buyBtnStyle?: number;
    showName?: number;
    showPrice?: number;
    goodsGap?: number;
    goodsPadding?: number;
    backgroundColor?: string;
    boxRadius?: number;
    boxPadding?: number;
    boxPaddingTop?: number;
    boxPaddingBottom?: number;
    innerPadding?: number;
    seckillTitle?: string;
    title?: PcSeckillTitleDecor;
    frame?: PcSeckillFrameDecor;
}
