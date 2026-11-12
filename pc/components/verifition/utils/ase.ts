import CryptoJS from "crypto-js";

/**
 * 对内容进行 AES 加密。
 *
 * @param word 要加密的内容
 * @param keyWord 服务器随机返回的关键字，默认值为 "XwKsGlMcdPMEhR1B"
 * @returns 加密后的字符串
 */
export function aesEncrypt(word: string | number, keyWord: string = "XwKsGlMcdPMEhR1B"): string {
    const key = CryptoJS.enc.Utf8.parse(keyWord);
    const srcs = CryptoJS.enc.Utf8.parse(word);
    const encrypted = CryptoJS.AES.encrypt(srcs, key, { mode: CryptoJS.mode.ECB, padding: CryptoJS.pad.Pkcs7 });
    return encrypted.toString();
}
