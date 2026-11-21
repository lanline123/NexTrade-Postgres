import { View, Image, Text } from "@tarojs/components";

import { t } from "@/utils/i18n";

import "./QrEntryPopup.scss";

/**
 * 扫码入口弹窗（我方新增，论坛 B 类缓做项 #25）。
 *
 * 通用组件：底部导航 / 悬浮入口中 entryType = 'qrcode' 的条目点击后弹出，
 * 展示企微 / 公众号二维码大图，用户长按识别（微信内置浏览器与小程序
 * 均由系统原生处理长按识别，本组件只负责呈现）。
 * 消费方：CustomTabbar、TigTabbar（两套底部导航渲染共用，保证两端一致）。
 * PC 端对应实现为 GlobalSideTools 的 gst__qr-panel（同字段同结构）。
 */
export interface QrEntryPopupProps {
    visible: boolean;
    /** 标题（导航项文案，如「联系我们」）；缺省时用类型名 */
    title?: string;
    /** 二维码类型：'wecom' 企业微信 / 'official' 公众号 */
    qrType?: string;
    /** 二维码图片 URL */
    qrImage?: string;
    onClose: () => void;
}

export default function QrEntryPopup({ visible, title, qrType, qrImage, onClose }: QrEntryPopupProps) {
    if (!visible) {
        return null;
    }
    const typeLabel = qrType === "official" ? t("微信公众号") : t("企业微信");

    return (
        <View className="qr-entry-popup" onClick={onClose}>
            <View
                className="qr-entry-popup__card"
                onClick={(e: any) => {
                    e.stopPropagation();
                }}
            >
                <Text className="qr-entry-popup__title">{title || typeLabel}</Text>
                <Text className="qr-entry-popup__type">{typeLabel}</Text>
                {qrImage ? (
                    <Image className="qr-entry-popup__img" src={qrImage} mode="aspectFit" showMenuByLongpress />
                ) : (
                    <View className="qr-entry-popup__img qr-entry-popup__img--empty" />
                )}
                <Text className="qr-entry-popup__hint">{t("长按识别二维码")}</Text>
                <View className="qr-entry-popup__close" onClick={onClose}>
                    <Text className="qr-entry-popup__close-icon">×</Text>
                </View>
            </View>
        </View>
    );
}
