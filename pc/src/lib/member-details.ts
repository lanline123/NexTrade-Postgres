/**
 * 会员详情页配置表。
 * 每个详情页 = 一个接口 + 一组字段，页面本身只有几行（见 app/member/**）。
 * 字段给的 fallbacks 是对后端字段名不确定时的多候选兜底，显示为「-」而不是报错。
 */
export type DetailFieldType = 'text' | 'price' | 'datetime';

export type DetailField = {
    key: string;
    label: string;
    type?: DetailFieldType;
    fallbacks?: string[];
};

export type DetailConfig = {
    api: string;
    title: string;
    idParam?: string;
    fields: DetailField[];
    /** 原 Vue 文件，页面底部展示，便于 1:1 对照 */
    source: string;
};

export const MEMBER_DETAILS: Record<string, DetailConfig> = {
    'return/info': {
        api: 'user/aftersales/detail',
        title: '售后详情',
        fields: [
            { key: 'aftersalesSn', label: '售后单号', fallbacks: ['aftersalesNumber', 'id'] },
            { key: 'orderSn', label: '订单号' },
            { key: 'statusName', label: '状态', fallbacks: ['aftersalesStatusName'] },
            { key: 'productName', label: '商品' },
            { key: 'refundAmount', label: '退款金额', type: 'price' },
            { key: 'reason', label: '售后原因', fallbacks: ['aftersalesReason'] },
            { key: 'description', label: '问题描述', fallbacks: ['content'] },
            { key: 'addTime', label: '申请时间', type: 'datetime', fallbacks: ['createTime'] }
        ],
        source: 'pages/member/return/Info.vue'
    },
    'return/log': {
        api: 'user/aftersales/getRecord',
        title: '售后进度',
        fields: [
            { key: 'statusName', label: '当前状态' },
            { key: 'addTime', label: '更新时间', type: 'datetime', fallbacks: ['createTime'] },
            { key: 'remark', label: '处理说明', fallbacks: ['content'] }
        ],
        source: 'pages/member/return/Log.vue'
    },
    'return/loginfo': {
        api: 'user/aftersales/getRecord',
        title: '进度详情',
        fields: [
            { key: 'statusName', label: '状态' },
            { key: 'operator', label: '处理人', fallbacks: ['adminName'] },
            { key: 'addTime', label: '时间', type: 'datetime', fallbacks: ['createTime'] },
            { key: 'remark', label: '说明', fallbacks: ['content'] }
        ],
        source: 'pages/member/return/LogInfo.vue'
    },
    'comment/info': {
        api: 'user/comment/detail',
        title: '评价详情',
        fields: [
            { key: 'productName', label: '商品' },
            { key: 'score', label: '评分', fallbacks: ['commentScore'] },
            { key: 'content', label: '评价内容', fallbacks: ['commentContent'] },
            { key: 'replyContent', label: '商家回复', fallbacks: ['reply'] },
            { key: 'addTime', label: '评价时间', type: 'datetime', fallbacks: ['createTime'] }
        ],
        source: 'pages/member/comment/Info.vue'
    },
    'feedback/info': {
        api: 'user/feedback/list',
        title: '反馈详情',
        fields: [
            { key: 'typeName', label: '类型', fallbacks: ['type'] },
            { key: 'content', label: '反馈内容' },
            { key: 'replyContent', label: '回复', fallbacks: ['reply'] },
            { key: 'addTime', label: '提交时间', type: 'datetime', fallbacks: ['createTime'] }
        ],
        source: 'pages/member/feedback/Info.vue'
    },
    'inquiry/info': {
        api: 'user/priceInquiry/detail',
        title: '询价详情',
        fields: [
            { key: 'productName', label: '商品' },
            { key: 'quantity', label: '数量' },
            { key: 'price', label: '报价', type: 'price', fallbacks: ['quotePrice'] },
            { key: 'statusName', label: '状态' },
            { key: 'remark', label: '备注', fallbacks: ['content'] },
            { key: 'addTime', label: '提交时间', type: 'datetime', fallbacks: ['createTime'] }
        ],
        source: 'pages/member/inquiry/Info.vue'
    },
    'group/info': {
        api: 'promotion/groupon/record/details',
        title: '团购详情',
        fields: [
            { key: 'productName', label: '商品' },
            { key: 'statusName', label: '状态' },
            { key: 'grouponNum', label: '成团人数', fallbacks: ['peopleNum'] },
            { key: 'joinNum', label: '已参团人数' },
            { key: 'addTime', label: '开团时间', type: 'datetime', fallbacks: ['createTime'] }
        ],
        source: 'pages/member/group/Info.vue'
    },
    'pin/group': {
        api: 'promotion/groupon/record/details',
        title: '拼团详情',
        fields: [
            { key: 'productName', label: '商品' },
            { key: 'statusName', label: '状态' },
            { key: 'grouponNum', label: '成团人数', fallbacks: ['peopleNum'] },
            { key: 'addTime', label: '开团时间', type: 'datetime', fallbacks: ['createTime'] }
        ],
        source: 'pages/member/pin/Group.vue'
    },
    'order/claiminfo': {
        api: 'user/order/offlinePaymentClaim/detail',
        title: '线下支付认款',
        fields: [
            { key: 'statusName', label: '认款状态' },
            { key: 'amount', label: '认款金额', type: 'price' },
            { key: 'rejectReason', label: '驳回原因', fallbacks: ['offlinePaymentClaimRejectReason'] },
            { key: 'addTime', label: '提交时间', type: 'datetime', fallbacks: ['createTime'] }
        ],
        source: 'pages/member/order/claimInfo.vue'
    },
    'orderInvoice/info': {
        api: 'user/orderInvoice/detail',
        title: '发票详情',
        idParam: 'id',
        fields: [
            { key: 'orderSn', label: '订单号' },
            { key: 'invoiceTypeName', label: '发票类型' },
            { key: 'statusName', label: '状态' },
            { key: 'amount', label: '开票金额', type: 'price', fallbacks: ['invoiceAmount'] },
            { key: 'companyName', label: '抬头' },
            { key: 'addTime', label: '申请时间', type: 'datetime', fallbacks: ['createTime'] }
        ],
        source: 'pages/member/orderInvoice/Info.vue'
    },
    'userInvoice/info': {
        api: 'user/invoice/detail',
        title: '发票抬头',
        fields: [
            { key: 'titleTypeName', label: '抬头类型', fallbacks: ['titleType'] },
            { key: 'companyName', label: '抬头名称' },
            { key: 'taxNumber', label: '税号', fallbacks: ['taxNo'] },
            { key: 'companyAddress', label: '注册地址' },
            { key: 'companyPhone', label: '注册电话' },
            { key: 'bankName', label: '开户银行' },
            { key: 'bankAccount', label: '银行账号' }
        ],
        source: 'pages/member/userInvoice/Info.vue'
    },
    'userInvoice/confirminfo': {
        api: 'user/invoice/detail',
        title: '确认发票信息',
        fields: [
            { key: 'companyName', label: '抬头名称' },
            { key: 'taxNumber', label: '税号', fallbacks: ['taxNo'] },
            { key: 'invoiceTypeName', label: '发票类型', fallbacks: ['invoiceType'] },
            { key: 'amount', label: '开票金额', type: 'price' }
        ],
        source: 'pages/member/userInvoice/confirmInfo.vue'
    }
};
