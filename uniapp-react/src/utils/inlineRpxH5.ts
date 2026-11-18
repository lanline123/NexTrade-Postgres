/**
 * H5 内联样式 rpx 收口（2026-11-18 全量修复）。
 *
 * 背景：原版 uni-app 的内联 :style 里 rpx 由框架运行时换算；Taro H5 无此换算，
 * 内联样式又不经 PostCSS，rpx 字符串被浏览器按无效值整段丢弃、属性回退到继承值
 * （分类页价格三段字号回退根字号显大即此因，全仓内联 rpx 约 700 处同病）。
 * 样式表内的 rpx 由 pxtransform 构建期换算、不受影响；小程序端 rpx 原生有效、不装此补丁。
 *
 * 口径与 pxtransform 完全一致：designWidth 750、deviceRatio[750]=1、baseFontSize 20，
 * 即 NN rpx → NN/40 rem（180rpx→4.5rem、32rpx→0.8rem，保留 5 位小数去尾零）。
 *
 * 收口点：Taro H5 渲染器最终经 CSSStyleDeclaration.setProperty 写真实 DOM 样式
 * （产物 bundle 实测），另补 cssText 赋值与 setAttribute('style') 两条旁路。
 * 只改写含 rpx 的字符串值，其余原样透传；已换算的值不含 rpx、不会二次处理。
 */

const RPX_RE = /(-?\d*\.?\d+)rpx/g;

export function convertInlineRpx(value: string): string {
    if (!value || value.indexOf("rpx") === -1) return value;
    return value.replace(RPX_RE, (_, n: string) => {
        const num = parseFloat(n);
        if (!isFinite(num)) return _;
        return `${Number((num / 40).toFixed(5))}rem`;
    });
}

let installed = false;

export function installInlineRpxH5(): void {
    if (installed) return;
    installed = true;
    if (process.env.TARO_ENV !== "h5" || typeof window === "undefined") return;
    try {
        const proto = (window as any).CSSStyleDeclaration?.prototype;
        if (proto) {
            const origSetProperty = proto.setProperty;
            if (typeof origSetProperty === "function") {
                proto.setProperty = function (prop: string, value: string, priority?: string) {
                    const v = typeof value === "string" ? convertInlineRpx(value) : value;
                    return origSetProperty.call(this, prop, v, priority);
                };
            }
            const cssTextDesc = Object.getOwnPropertyDescriptor(proto, "cssText");
            if (cssTextDesc?.set && cssTextDesc?.get) {
                Object.defineProperty(proto, "cssText", {
                    configurable: true,
                    enumerable: cssTextDesc.enumerable,
                    get: cssTextDesc.get,
                    set(v: string) {
                        cssTextDesc.set!.call(this, typeof v === "string" ? convertInlineRpx(v) : v);
                    }
                });
            }
        }
        const elProto = (window as any).Element?.prototype;
        if (elProto?.setAttribute) {
            const origSetAttribute = elProto.setAttribute;
            elProto.setAttribute = function (name: string, value: string) {
                if (name === "style" && typeof value === "string") {
                    value = convertInlineRpx(value);
                }
                return origSetAttribute.call(this, name, value);
            };
        }
    } catch {
        // 补丁失败不阻断应用启动：未收口的内联 rpx 维持原回退行为
    }
}
