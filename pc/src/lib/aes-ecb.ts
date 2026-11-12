/**
 * 极简 AES-128-ECB（仅加密）+ PKCS#7 + Base64。
 *
 * 用途：商城滑动验证码 `common/verification/check` 要求把点位 JSON 用服务端下发的
 * secretKey 做 AES/ECB/PKCS7 加密后再提交（Vue 端原实现用 crypto-js）。
 * 这里用纯 JS 实现，免掉 crypto-js 依赖；S-box 由 GF(2^8) 逆元 + 仿射变换现算，不硬编码表。
 */

const SBOX = (() => {
    const gmul = (a: number, b: number) => {
        let p = 0;
        for (let i = 0; i < 8; i += 1) {
            if (b & 1) p ^= a;
            const hi = a & 0x80;
            a = (a << 1) & 0xff;
            if (hi) a ^= 0x1b;
            b >>= 1;
        }
        return p & 0xff;
    };
    const rotl8 = (x: number, n: number) => ((x << n) | (x >> (8 - n))) & 0xff;
    const box = new Uint8Array(256);
    for (let i = 0; i < 256; i += 1) {
        let inv = 0;
        if (i !== 0) {
            // 逆元 = a^254（费马小定理）
            inv = 1;
            let base = i;
            let exp = 254;
            while (exp > 0) {
                if (exp & 1) inv = gmul(inv, base);
                base = gmul(base, base);
                exp >>= 1;
            }
        }
        box[i] = (inv ^ rotl8(inv, 1) ^ rotl8(inv, 2) ^ rotl8(inv, 3) ^ rotl8(inv, 4) ^ 0x63) & 0xff;
    }
    return box;
})();

const RCON = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

function expandKey(key: Uint8Array): Uint8Array {
    if (key.length !== 16) throw new Error('AES-128 需要 16 字节密钥');
    const w = new Uint8Array(176);
    w.set(key, 0);
    for (let i = 16; i < 176; i += 4) {
        const prev = w.subarray(i - 4, i);
        const t = [prev[0], prev[1], prev[2], prev[3]];
        if (i % 16 === 0) {
            t.push(t.shift() as number); // RotWord
            for (let k = 0; k < 4; k += 1) t[k] = SBOX[t[k]]; // SubWord
            t[0] ^= RCON[i / 16 - 1];
        }
        const base = w.subarray(i - 16, i - 12);
        for (let k = 0; k < 4; k += 1) w[i + k] = base[k] ^ t[k];
    }
    return w;
}

const xtime = (a: number) => (((a << 1) ^ (a & 0x80 ? 0x1b : 0)) & 0xff);

function encryptBlock(block: Uint8Array, w: Uint8Array): Uint8Array {
    const s = Uint8Array.from(block);
    const addRoundKey = (round: number) => {
        for (let i = 0; i < 16; i += 1) s[i] ^= w[round * 16 + i];
    };

    addRoundKey(0);
    for (let round = 1; round <= 10; round += 1) {
        for (let i = 0; i < 16; i += 1) s[i] = SBOX[s[i]]; // SubBytes（flat 索引即 4c+r）
        const shifted = Uint8Array.from(s); // ShiftRows
        for (let c = 0; c < 4; c += 1) {
            for (let r = 1; r < 4; r += 1) {
                shifted[4 * c + r] = s[4 * ((c + r) % 4) + r];
            }
        }
        s.set(shifted);
        if (round !== 10) {
            for (let c = 0; c < 4; c += 1) {
                const a0 = s[4 * c];
                const a1 = s[4 * c + 1];
                const a2 = s[4 * c + 2];
                const a3 = s[4 * c + 3];
                s[4 * c] = xtime(a0) ^ (xtime(a1) ^ a1) ^ a2 ^ a3;
                s[4 * c + 1] = a0 ^ xtime(a1) ^ (xtime(a2) ^ a2) ^ a3;
                s[4 * c + 2] = a0 ^ a1 ^ xtime(a2) ^ (xtime(a3) ^ a3);
                s[4 * c + 3] = (xtime(a0) ^ a0) ^ a1 ^ a2 ^ xtime(a3);
            }
        }
        addRoundKey(round);
    }
    return s;
}

const B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

function toBase64(bytes: Uint8Array): string {
    let out = '';
    for (let i = 0; i < bytes.length; i += 3) {
        const b0 = bytes[i];
        const b1 = bytes[i + 1];
        const b2 = bytes[i + 2];
        out += B64[b0 >> 2];
        out += B64[((b0 & 0x03) << 4) | ((b1 ?? 0) >> 4)];
        out += i + 1 < bytes.length ? B64[((b1 & 0x0f) << 2) | ((b2 ?? 0) >> 6)] : '=';
        out += i + 2 < bytes.length ? B64[b2 & 0x3f] : '=';
    }
    return out;
}

/** AES-128-ECB/PKCS7 加密后转 Base64，等价于 CryptoJS.AES.encrypt(word, Utf8.parse(key), {mode:ECB,padding:Pkcs7}).toString() */
export function aesEncrypt(text: string, key: string): string {
    const plain = new TextEncoder().encode(text);
    const keyBytes = new TextEncoder().encode(key);
    const padded = new Uint8Array(Math.ceil((plain.length + 1) / 16) * 16);
    padded.set(plain, 0);
    const pad = padded.length - plain.length;
    for (let i = plain.length; i < padded.length; i += 1) padded[i] = pad;

    const w = expandKey(keyBytes);
    const out = new Uint8Array(padded.length);
    for (let offset = 0; offset < padded.length; offset += 16) {
        out.set(encryptBlock(padded.subarray(offset, offset + 16), w), offset);
    }
    return toBase64(out);
}
