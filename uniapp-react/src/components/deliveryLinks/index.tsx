import { Text, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import { copy } from "@/utils";
import { t } from "@/utils/i18n";

import "./index.scss";

/**
 * 从交付内容 HTML 中抽取 URL（<a href> 与裸 http(s) 链接），去重保序。
 * 与 PC 端 pc-semi/src/lib/deliveryLinks.ts 同口径。
 */
export function extractUrls(html?: string | null): string[] {
    if (!html) return [];
    const found: string[] = [];
    const seen = new Set<string>();
    const push = (raw: string) => {
        const url = raw.replace(/&amp;/g, "&").replace(/[\])）。，；、！？'"」]+$/u, "").trim();
        if (/^https?:\/\//i.test(url) && !seen.has(url)) {
            seen.add(url);
            found.push(url);
        }
    };
    let m: RegExpExecArray | null;
    const hrefRe = /href\s*=\s*["']([^"']+)["']/gi;
    while ((m = hrefRe.exec(html)) !== null) push(m[1]);
    const bareRe = /https?:\/\/[^\s<>"']+/gi;
    while ((m = bareRe.exec(html)) !== null) push(m[0]);
    return found;
}

/**
 * 交付链接行（付费内容 type=4 / 虚拟内容 type=2 的交付区专用）：
 * 把内容里的 URL 抽成独立链接行，带「打开 / 复制」，不依赖 RichText
 * 锚点——小程序 rich-text 不支持链接跳转与长按复制（论坛帖
 * 80793616636238732 的原始诉求），H5 浏览器里也给一条明确的打开通道。
 * 非 H5 端（未来 weapp）「打开」降级为复制 + 提示到浏览器打开。
 * 无 URL 时不渲染。本组件不使用 rich-text，weapp 可直接复用。
 */
export default function DeliveryLinks({ html }: { html?: string | null }) {
    const urls = extractUrls(html);
    if (!urls.length) return null;

    const openUrl = (url: string) => {
        if (process.env.TARO_ENV === "h5") {
            window.open(url, "_blank");
        } else {
            copy(url, () => {
                Taro.showToast({ title: t("链接已复制，请在浏览器中打开"), icon: "none" });
            });
        }
    };

    return (
        <View className="delivery-links">
            {urls.map((url) => (
                <View className="delivery-link-row" key={url}>
                    <Text className="delivery-link-url">{url}</Text>
                    <View className="delivery-link-btn" onClick={() => openUrl(url)}>
                        {t("打开")}
                    </View>
                    <View
                        className="delivery-link-btn"
                        onClick={() => {
                            copy(url, () => {
                                Taro.showToast({ title: t("链接已复制"), icon: "none" });
                            });
                        }}
                    >
                        {t("复制")}
                    </View>
                </View>
            ))}
        </View>
    );
}
