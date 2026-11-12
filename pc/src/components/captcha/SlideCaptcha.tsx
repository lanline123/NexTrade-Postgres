'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { Modal, Spin } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiPost } from '@/lib/client-api';
import { aesEncrypt } from '@/lib/aes-ecb';

/**
 * Vue 基准：components/verifition/Verify.vue + Verify/VerifySlide.vue（anji-plus 滑动拼图）
 *
 * 真实接口（实测）：
 * - POST common/verification/captcha {captchaType:"blockPuzzle"}
 *   → { originalImageBase64(310x155), jigsawImageBase64(47x155), token, secretKey, captchaType }
 * - POST common/verification/check
 *   { captchaType, pointJson: AES(JSON.stringify({x,y}), secretKey), token }
 *
 * 成功后回传的 verifyToken 不是服务端 token，而是
 * AES(token + "---" + JSON.stringify({x,y}), secretKey)，登录接口原样带上即可。
 */
const IMG_W = 310;
const IMG_H = 155;
const PIECE_W = 47;
const TRACK_PAD = 6;

interface CaptchaData {
    originalImageBase64?: string;
    jigsawImageBase64?: string;
    token?: string;
    secretKey?: string;
}

interface Props {
    visible: boolean;
    onClose: () => void;
    onSuccess: (verifyToken: string) => void;
}

export default function SlideCaptcha({ visible, onClose, onSuccess }: Props) {
    const { t } = useTranslation();
    const [data, setData] = useState<CaptchaData>({});
    const [loading, setLoading] = useState(false);
    const [offset, setOffset] = useState(0);
    const [dragging, setDragging] = useState(false);
    const [tip, setTip] = useState<{ text: string; ok: boolean } | null>(null);
    const startX = useRef(0);
    const trackRef = useRef<HTMLDivElement | null>(null);

    const load = useCallback(async () => {
        setLoading(true);
        setOffset(0);
        setTip(null);
        try {
            const result = await apiPost<CaptchaData>('common/verification/captcha', { captchaType: 'blockPuzzle' });
            setData(result ?? {});
        } catch (e) {
            setTip({ text: (e as Error).message, ok: false });
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        if (visible) void load();
    }, [visible, load]);

    useEffect(() => {
        if (!dragging) return;
        const onMove = (event: MouseEvent | TouchEvent) => {
            const point = 'touches' in event ? event.touches[0] : event;
            const move = point.clientX - startX.current;
            setOffset(Math.max(0, Math.min(IMG_W - PIECE_W, move)));
        };
        const onUp = () => setDragging(false);
        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
        window.addEventListener('touchmove', onMove, { passive: true });
        window.addEventListener('touchend', onUp);
        return () => {
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
            window.removeEventListener('touchmove', onMove);
            window.removeEventListener('touchend', onUp);
        };
    }, [dragging]);

    const start = (event: React.MouseEvent | React.TouchEvent) => {
        if (!data.originalImageBase64) return;
        const point = 'touches' in event ? event.touches[0] : event;
        // 以滑块自身当前位置为基准，避免二次拖动跳动
        startX.current = point.clientX - offset;
        setDragging(true);
    };

    const verify = useCallback(async () => {
        if (!data.token || !data.secretKey) return;
        const pointJson = aesEncrypt(JSON.stringify({ x: offset, y: 5.0 }), data.secretKey);
        try {
            await apiPost('common/verification/check', {
                captchaType: 'blockPuzzle',
                pointJson,
                token: data.token,
            });
            const verifyToken = aesEncrypt(`${data.token}---${JSON.stringify({ x: offset, y: 5.0 })}`, data.secretKey);
            setTip({ text: t('验证成功'), ok: true });
            setTimeout(() => onSuccess(verifyToken), 300);
        } catch (e) {
            setTip({ text: (e as Error).message || t('验证失败'), ok: false });
            void load();
        }
    }, [data, offset, onSuccess, t, load]);

    // 松手即校验
    const wasDragging = useRef(false);
    useEffect(() => {
        if (dragging) {
            wasDragging.current = true;
            return;
        }
        if (wasDragging.current) {
            wasDragging.current = false;
            if (offset > 0) void verify();
        }
    }, [dragging, offset, verify]);

    const progress = Math.round((offset / (IMG_W - PIECE_W)) * 100);

    return (
        <Modal
            visible={visible}
            title={t('请完成安全验证')}
            footer={null}
            width={IMG_W + 60}
            onCancel={onClose}
            maskClosable={false}
        >
            <Spin spinning={loading}>
                <div className="pc-captcha">
                    <div className="pc-captcha-stage" style={{ width: IMG_W, height: IMG_H }}>
                        {data.originalImageBase64 ? (
                            <img
                                className="pc-captcha-bg"
                                src={`data:image/png;base64,${data.originalImageBase64}`}
                                alt=""
                                draggable={false}
                            />
                        ) : null}
                        {data.jigsawImageBase64 ? (
                            <img
                                className="pc-captcha-piece"
                                src={`data:image/png;base64,${data.jigsawImageBase64}`}
                                style={{ left: offset, width: PIECE_W, height: IMG_H }}
                                alt=""
                                draggable={false}
                            />
                        ) : null}
                        <span className="pc-captcha-refresh" onClick={() => void load()}>
                            {t('刷新')}
                        </span>
                        {tip ? (
                            <span className={`pc-captcha-tip${tip.ok ? ' is-ok' : ' is-err'}`}>{tip.text}</span>
                        ) : null}
                    </div>

                    <div ref={trackRef} className="pc-captcha-track" style={{ width: IMG_W }}>
                        <div className="pc-captcha-fill" style={{ width: offset + PIECE_W / 2 }} />
                        <div
                            className={`pc-captcha-handle${dragging ? ' is-dragging' : ''}`}
                            style={{ left: offset }}
                            onMouseDown={start}
                            onTouchStart={start}
                        >
                            {offset >= IMG_W - PIECE_W ? '✓' : '→'}
                        </div>
                        <span className="pc-captcha-hint">{offset > 0 ? `${progress}%` : t('拖动滑块完成拼图')}</span>
                    </div>
                </div>
            </Spin>
        </Modal>
    );
}
