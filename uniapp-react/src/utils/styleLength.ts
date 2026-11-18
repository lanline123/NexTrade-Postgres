import Taro from "@tarojs/taro";

/** PostCSS converts stylesheet rpx, but inline styles need conversion on H5. */
export function rpxToStyleLength(rpx: number): string {
    return process.env.TARO_ENV === "h5" ? Taro.pxTransform(rpx) : `${rpx}rpx`;
}
