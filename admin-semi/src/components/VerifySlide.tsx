/**
 * 行为验证码（滑块拼图），管理端版。
 *
 * 协议与 C 端一致（后端共用 nextrade_services::captcha 实现），
 * 端点走管理端匿名白名单 /adminapi/common/verification/**：
 * - GET  /adminapi/common/verification/captcha → { originalImageBase64, jigsawImageBase64, token, secretKey, captchaType }
 * - POST /adminapi/common/verification/check    → { captchaType, pointJson, token }
 *   pointJson = AES-ECB(secretKey, JSON.stringify({x, y:5.0}))，Base64
 * - 校验成功后 captchaVerification = AES-ECB(secretKey, `${token}---${pointJson 明文}`)，
 *   登录时随 captchaVerification 提交，后端一次性校验（防重放）。
 *
 * 触发时机：后台登录失败次数达到后端阈值后，登录接口返回 code=1002，
 * 登录页据此弹出本组件，成功后带令牌重试登录（此前界面缺失该弹窗，
 * 触发阈值后账号会被锁死只能等后端重启，现补齐）。
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { Toast } from '@douyinfe/semi-ui';
import { aesEncrypt } from '../lib/crypto';

interface CaptchaData {
  originalImageBase64?: string;
  jigsawImageBase64?: string;
  token?: string;
  secretKey?: string;
  captchaType?: string;
}

interface VerifySlideProps {
  /** 成功回调，参数为 captchaVerification */
  onSuccess: (verifyToken: string) => void;
  onError?: () => void;
  open: boolean;
  onClose: () => void;
}

const IMG_WIDTH = 310;
const IMG_HEIGHT = 155;

export default function VerifySlide({ onSuccess, onError, open, onClose }: VerifySlideProps) {
  const [captcha, setCaptcha] = useState<CaptchaData | null>(null);
  const [loading, setLoading] = useState(false);
  const [checking, setChecking] = useState(false);
  const [moveLeft, setMoveLeft] = useState(0);
  const [status, setStatus] = useState<'idle' | 'moving' | 'success' | 'fail'>('idle');
  const trackRef = useRef<HTMLDivElement | null>(null);
  const moveLeftRef = useRef(0);
  const startXRef = useRef(0);
  const draggingRef = useRef(false);
  const statusRef = useRef<'idle' | 'moving' | 'success' | 'fail'>('idle');

  const setMove = (value: number) => {
    moveLeftRef.current = value;
    setMoveLeft(value);
  };

  const setStatusBoth = (next: 'idle' | 'moving' | 'success' | 'fail') => {
    statusRef.current = next;
    setStatus(next);
  };

  const loadCaptcha = useCallback(async () => {
    setLoading(true);
    setStatusBoth('idle');
    setMove(0);
    try {
      const response = await fetch('/adminapi/common/verification/captcha', { credentials: 'include' });
      const json = await response.json();
      const data = (json?.data ?? json) as CaptchaData;
      if (!data?.token) throw new Error(json?.msg || '验证码加载失败');
      setCaptcha(data);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : '验证码加载失败' });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) void loadCaptcha();
  }, [open, loadCaptcha]);

  const startDrag = (clientX: number) => {
    if (statusRef.current === 'success' || statusRef.current === 'fail' || checking) return;
    draggingRef.current = true;
    startXRef.current = clientX - moveLeftRef.current;
  };

  const moveDrag = (clientX: number) => {
    if (!draggingRef.current) return;
    const track = trackRef.current;
    if (!track) return;
    const max = Math.max(0, track.clientWidth - 48);
    const next = Math.max(0, Math.min(max, clientX - startXRef.current));
    setMove(next);
    setStatusBoth('moving');
  };

  const endDrag = () => {
    if (!draggingRef.current) return;
    draggingRef.current = false;
    if (statusRef.current !== 'moving') return;
    void verify(moveLeftRef.current);
  };

  useEffect(() => {
    if (!open) return undefined;
    const onWinMove = (event: MouseEvent) => moveDrag(event.clientX);
    const onWinUp = () => endDrag();
    const onTouchMove = (event: TouchEvent) => {
      const touch = event.touches[0];
      if (touch) moveDrag(touch.clientX);
    };
    const onTouchEnd = () => endDrag();
    window.addEventListener('mousemove', onWinMove);
    window.addEventListener('mouseup', onWinUp);
    window.addEventListener('touchmove', onTouchMove, { passive: true });
    window.addEventListener('touchend', onTouchEnd);
    return () => {
      window.removeEventListener('mousemove', onWinMove);
      window.removeEventListener('mouseup', onWinUp);
      window.removeEventListener('touchmove', onTouchMove);
      window.removeEventListener('touchend', onTouchEnd);
    };
  }, [open ]);

  const verify = useCallback(
    async (left: number) => {
      const token = captcha?.token;
      const secretKey = captcha?.secretKey;
      if (!token) return;
      setChecking(true);
      const track = trackRef.current;
      const imgWidth = track?.clientWidth || IMG_WIDTH;
      const x = (left * IMG_WIDTH) / imgWidth;
      const point = JSON.stringify({ x, y: 5.0 });
      const pointJson = secretKey ? aesEncrypt(point, secretKey) : point;
      try {
        const response = await fetch('/adminapi/common/verification/check', {
          method: 'POST',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ captchaType: captcha?.captchaType || 'blockPuzzle', pointJson, token }),
        });
        const json = await response.json();
        if (json?.code !== undefined && json.code !== 0) {
          throw new Error(json?.msg || json?.message || '验证失败');
        }
        setStatusBoth('success');
        const verification = secretKey ? aesEncrypt(`${token}---${point}`, secretKey) : `${token}---${point}`;
        setTimeout(() => {
          onSuccess(verification);
          onClose();
        }, 400);
      } catch (error) {
        setStatusBoth('fail');
        Toast.error({ content: error instanceof Error ? error.message : '验证失败，请拖动滑块对齐拼图' });
        onError?.();
        setTimeout(() => {
          void loadCaptcha();
        }, 1000);
      } finally {
        setChecking(false);
        draggingRef.current = false;
      }
    },
    [captcha, loadCaptcha, onClose, onError, onSuccess],
  );

  if (!open) return null;

  return (
    <div className="pc-verify-mask" onClick={onClose}>
      <div className="pc-verify" onClick={(event) => event.stopPropagation()}>
        <div className="pc-verify__head">
          <span className="pc-verify__title">请完成安全验证</span>
          <button type="button" className="pc-verify__close" onClick={onClose} aria-label="关闭">×</button>
        </div>

        <div className="pc-verify__body">
          {loading ? (
            <div className="pc-verify__loading">加载中...</div>
          ) : (
            <>
              <div
                className="pc-verify__canvas"
                style={{ width: IMG_WIDTH, height: IMG_HEIGHT }}
                ref={trackRef}
                onMouseDown={(event) => startDrag(event.clientX)}
                onMouseMove={(event) => moveDrag(event.clientX)}
                onMouseUp={endDrag}
                onMouseLeave={endDrag}
                onTouchStart={(event) => startDrag(event.touches[0]?.clientX ?? 0)}
                onTouchMove={(event) => {
                  const touch = event.touches[0];
                  if (touch) moveDrag(touch.clientX);
                }}
                onTouchEnd={endDrag}
              >
                {captcha?.originalImageBase64 ? (
                  <img className="pc-verify__bg" src={`data:image/png;base64,${captcha.originalImageBase64}`} alt="" draggable={false} />
                ) : null}
                {captcha?.jigsawImageBase64 ? (
                  <img
                    className={`pc-verify__block${status === 'success' ? ' is-success' : ''}${status === 'fail' ? ' is-fail' : ''}`}
                    src={`data:image/png;base64,${captcha.jigsawImageBase64}`}
                    alt=""
                    style={{ left: `${moveLeft}px` }}
                    draggable={false}
                  />
                ) : null}
                <button type="button" className="pc-verify__refresh" onClick={() => void loadCaptcha()} aria-label="刷新">
                  ↻
                </button>
              </div>

              <div className="pc-verify__track">
                <div className="pc-verify__bar" style={{ width: `${moveLeft + 48}px` }} />
                <div
                  className={`pc-verify__slider${status === 'success' ? ' is-success' : ''}${status === 'fail' ? ' is-fail' : ''}`}
                  style={{ left: `${moveLeft}px` }}
                  onMouseDown={(event) => startDrag(event.clientX)}
                  onMouseMove={(event) => moveDrag(event.clientX)}
                  onMouseUp={endDrag}
                  onTouchStart={(event) => startDrag(event.touches[0]?.clientX ?? 0)}
                  onTouchEnd={endDrag}
                >
                  {status === 'success' ? '✓' : status === 'fail' ? '✕' : '⇢'}
                </div>
                <span className="pc-verify__tip">
                  {status === 'success' ? '验证成功' : status === 'fail' ? '验证失败，请重试' : '拖动滑块完成拼图'}
                </span>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
