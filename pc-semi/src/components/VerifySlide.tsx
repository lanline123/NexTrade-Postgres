'use client';

/**
 * 行为验证码（滑块拼图）。
 *
 * 源码对照：java-b2b2c-overseas-v5.8.61/view/pc/components/verifition/Verify/VerifySlide.vue
 * （419 行 AJ-Captcha 封装）。
 *
 * 协议（与后端 crates/services/src/system/captcha.rs 对齐）：
 * - GET  /api/common/verification/captcha → { originalImageBase64, jigsawImageBase64,
 *                                            token, secretKey, captchaType }
 * - POST /api/common/verification/check    → { captchaType, pointJson, token }
 *   pointJson = AES-ECB(secretKey, JSON.stringify({x, y:5.0}))，Base64
 * - 校验成功后生成 captchaVerification = AES-ECB(secretKey, `${token}---${JSON}`)
 *   （后端 verify_captcha_verification 解密比对），登录/发短信时随 verifyToken 提交。
 *
 * 旧版是「失败才弹」（登录接口返回 code 1002 时 verify.show()），这里保持同样策略：
 * 由父组件通过 open() 主动打开。
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { Button, Toast } from '@douyinfe/semi-ui';

import { useTranslation } from 'react-i18next';
import { aesEncrypt } from '@/lib/crypto';

interface CaptchaData {
  originalImageBase64?: string;
  jigsawImageBase64?: string;
  token?: string;
  secretKey?: string;
  captchaType?: string;
}

interface VerifySlideProps {
  /** 成功回调，参数为 captchaVerification（旧版 okCallback 的 e.verifyToken） */
  onSuccess: (verifyToken: string) => void;
  /** 校验失败回调 */
  onError?: () => void;
  /** 受控显示 */
  open: boolean;
  onClose: () => void;
}

const IMG_WIDTH = 310;
const IMG_HEIGHT = 155;

export default function VerifySlide({ onSuccess, onError, open, onClose }: VerifySlideProps) {
    const { t } = useTranslation();
  const [captcha, setCaptcha] = useState<CaptchaData | null>(null);
  const [loading, setLoading] = useState(false);
  const [checking, setChecking] = useState(false);
  const [moveLeft, setMoveLeft] = useState(0);
  const [status, setStatus] = useState<'idle' | 'moving' | 'success' | 'fail'>('idle');
  const trackRef = useRef<HTMLDivElement | null>(null);

  /**
   * 拖动位置同时存 state（渲染）与 ref（校验时读取最新值）。
   *
   * 旧版 VerifySlide.vue 用 window 级 mousemove/mouseup 监听：拖出滑块轨道后
   * 仍能继续拖动，且 mouseup 一定触发。纯 React 的 onMouseMove 只在元素内生效，
   * 拖到边缘会「卡住」，故这里用 window 监听对齐旧版行为。
   *
   * 用 ref 而非 state 是因为 mouseup 时 setState 尚未生效，闭包里的 moveLeft
   * 是上一次渲染的旧值，会导致校验坐标错误。
   */
  const moveLeftRef = useRef(0);
  const startXRef = useRef(0);
  const draggingRef = useRef(false);
  const statusRef = useRef<'idle' | 'moving' | 'success' | 'fail'>('idle');
  /** 拖动开始时缓存的轨道宽度：拖动中不再逐帧读 clientWidth（强制同步布局） */
  const trackWidthRef = useRef(0);
  /** 待应用的拖动位置 + 已排队的动画帧句柄（mousemove 合帧用） */
  const pendingLeftRef = useRef<number | null>(null);
  const rafRef = useRef(0);

  const setMove = (value: number) => {
    moveLeftRef.current = value;
    setMoveLeft(value);
  };

  /** 状态变更同时同步 ref（拖动判定读 ref，不读 state） */
  const setStatusBoth = (next: 'idle' | 'moving' | 'success' | 'fail') => {
    statusRef.current = next;
    setStatus(next);
  };

  const loadCaptcha = useCallback(async () => {
    setLoading(true);
    setStatusBoth('idle');
    setMove(0);
    try {
      const response = await fetch('/api/common/verification/captcha', { credentials: 'include' });
      const json = await response.json();
      const data = (json?.data ?? json) as CaptchaData;
      if (!data?.token) throw new Error(json?.msg || '验证码加载失败');
      setCaptcha(data);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('验证码加载失败') });
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    if (open) void loadCaptcha();
  }, [open, loadCaptcha]);

  /** 按下开始拖动：缓存轨道宽度，并把全页光标钉为 grabbing（见样式侧 body.pc-verify-dragging） */
  const startDrag = (clientX: number) => {
    if (statusRef.current === 'success' || statusRef.current === 'fail' || checking) return;
    draggingRef.current = true;
    startXRef.current = clientX - moveLeftRef.current;
    trackWidthRef.current = trackRef.current?.clientWidth || IMG_WIDTH;
    document.body.classList.add('pc-verify-dragging');
  };

  /**
   * 拖动中更新位置。
   *
   * mousemove 在快速拖动时每秒可达上百次：位置只入队、每动画帧最多应用
   * 一次（setMove → 整面板重渲染 ≤60 次/秒），且宽度用 startDrag 的缓存值，
   * 不再逐帧读 clientWidth 造成读写交替的强制同步布局（旧实现的主要卡顿源）。
   */
  const moveDrag = (clientX: number) => {
    if (!draggingRef.current) return;
    const max = Math.max(0, (trackWidthRef.current || IMG_WIDTH) - 48);
    const next = Math.max(0, Math.min(max, clientX - startXRef.current));
    pendingLeftRef.current = next;
    if (statusRef.current !== 'moving') setStatusBoth('moving');
    if (!rafRef.current) {
      rafRef.current = requestAnimationFrame(() => {
        rafRef.current = 0;
        if (pendingLeftRef.current !== null) {
          setMove(pendingLeftRef.current);
          pendingLeftRef.current = null;
        }
      });
    }
  };

  /**
   * 松开触发校验。
   *
   * 旧版 VerifySlide.vue 在 window 上挂 mousemove/mouseup/touchmove/touchend：
   * 拖出滑块轨道后仍能继续拖动，且松开一定触发校验。
   * 这里同样在 window 监听（见下方 useEffect），元素上的 onMouseMove 只是兜底。
   * 校验前先把排队中的最后一帧位置同步落下，保证提交坐标就是释放点。
   */
  const endDrag = () => {
    if (!draggingRef.current) return;
    draggingRef.current = false;
    document.body.classList.remove('pc-verify-dragging');
    if (rafRef.current) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = 0;
    }
    if (pendingLeftRef.current !== null) {
      setMove(pendingLeftRef.current);
      pendingLeftRef.current = null;
    }
    if (statusRef.current !== 'moving') return;
    void verify(moveLeftRef.current);
  };

  /**
   * window 级拖动监听（对齐旧版 VerifySlide.vue:206-217）。
   *
   * 面板打开期间常驻监听、处理器内部用 draggingRef 判定 —— 避免拖动过程中
   * 每次 setState 触发 effect 重挂监听（性能浪费 + 事件可能漏挂）。
   * 处理器一律经 handlersRef 转发到最新一次渲染的 moveDrag/endDrag：旧实现
   * 直接闭包捕获打开瞬间的处理器，其 verify 闭包里的 captcha 还是 null，
   * 在面板外松手时校验永远发不出去（界面停在 moving 态、看起来像卡死），
   * 只能靠元素自身的 onMouseUp 兜底才偶尔成功。
   */
  const handlersRef = useRef({ moveDrag, endDrag });
  useEffect(() => {
    handlersRef.current = { moveDrag, endDrag };
  });

  useEffect(() => {
    if (!open) return undefined;
    const onWinMove = (event: MouseEvent) => handlersRef.current.moveDrag(event.clientX);
    const onWinUp = () => handlersRef.current.endDrag();
    const onTouchMove = (event: TouchEvent) => {
      const touch = event.touches[0];
      if (touch) handlersRef.current.moveDrag(touch.clientX);
    };
    const onTouchEnd = () => handlersRef.current.endDrag();
    window.addEventListener('mousemove', onWinMove);
    window.addEventListener('mouseup', onWinUp);
    window.addEventListener('touchmove', onTouchMove, { passive: true });
    window.addEventListener('touchend', onTouchEnd);
    return () => {
      window.removeEventListener('mousemove', onWinMove);
      window.removeEventListener('mouseup', onWinUp);
      window.removeEventListener('touchmove', onTouchMove);
      window.removeEventListener('touchend', onTouchEnd);
      document.body.classList.remove('pc-verify-dragging');
      if (rafRef.current) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = 0;
      }
      pendingLeftRef.current = null;
    };
  }, [open]);

  /** 鼠标按下 */
  const onMouseDown = (event: React.MouseEvent) => {
    startDrag(event.clientX);
  };

  /** 鼠标移动 */
  const onMouseMove = (event: React.MouseEvent) => {
    moveDrag(event.clientX);
  };

  /** 鼠标松开 */
  const onMouseUp = () => {
    endDrag();
  };

  /** 校验 */
  const verify = useCallback(
    async (left: number) => {
      const token = captcha?.token;
      const secretKey = captcha?.secretKey;
      if (!token) return;
      setChecking(true);
      // 旧版：moveLeftDistance = (滑块左偏移 * 310) / 图片实际宽度
      // 宽度优先用 startDrag 缓存值（与拖动全程同一口径），未缓存时回退实测
      const imgWidth = trackWidthRef.current || trackRef.current?.clientWidth || IMG_WIDTH;
      const x = (left * IMG_WIDTH) / imgWidth;
      const point = JSON.stringify({ x, y: 5.0 });
      const pointJson = secretKey ? aesEncrypt(point, secretKey) : point;
      try {
        const response = await fetch('/api/common/verification/check', {
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
        const verification = secretKey
          ? aesEncrypt(`${token}---${point}`, secretKey)
          : `${token}---${point}`;
        setTimeout(() => {
          onSuccess(verification);
          onClose();
        }, 400);
      } catch (error) {
        setStatusBoth('fail');
        Toast.error({ content: error instanceof Error ? error.message : t('验证失败，请拖动滑块对齐拼图') });
        onError?.();
        setTimeout(() => {
          void loadCaptcha();
        }, 1000);
      } finally {
        setChecking(false);
        draggingRef.current = false;
      }
    },
    [captcha, loadCaptcha, onClose, onError, onSuccess, t],
  );

  if (!open) return null;

  return (
    <div className="pc-verify-mask" onClick={onClose}>
      <div className="pc-verify" onClick={(event) => event.stopPropagation()}>
        <div className="pc-verify__head">
          <span className="pc-verify__title">{t('请完成安全验证')}</span>
          <button type="button" className="pc-verify__close" onClick={onClose} aria-label={t('关闭')}>×</button>
        </div>

        <div className="pc-verify__body">
          {loading ? (
            <div className="pc-verify__loading">{t('加载中...')}</div>
          ) : (
            <>
              <div
                className="pc-verify__canvas"
                style={{ width: IMG_WIDTH, height: IMG_HEIGHT }}
                ref={trackRef}
                onMouseDown={onMouseDown}
                onMouseMove={onMouseMove}
                onMouseUp={onMouseUp}
                onMouseLeave={onMouseUp}
                onTouchStart={(event) => startDrag(event.touches[0]?.clientX ?? 0)}
                onTouchMove={(event) => {
                  const touch = event.touches[0];
                  if (touch) moveDrag(touch.clientX);
                }}
                onTouchEnd={endDrag}
              >
                {captcha?.originalImageBase64 ? (
                  <img
                    className="pc-verify__bg"
                    src={`data:image/png;base64,${captcha.originalImageBase64}`}
                    alt=""
                    draggable={false}
                  />
                ) : null}
                {captcha?.jigsawImageBase64 ? (
                  <img
                    className={`pc-verify__block${status === 'success' ? ' is-success' : ''}${status === 'fail' ? ' is-fail' : ''}`}
                    src={`data:image/png;base64,${captcha.jigsawImageBase64}`}
                    alt=""
                    style={{ transform: `translateX(${moveLeft}px)` }}
                    draggable={false}
                  />
                ) : null}
                <button type="button" className="pc-verify__refresh" onClick={() => void loadCaptcha()} aria-label={t('刷新')}>
                  ↻
                </button>
              </div>

              <div className="pc-verify__track">
                <div className="pc-verify__bar" style={{ width: `${moveLeft + 48}px` }} />
                <div
                  className={`pc-verify__slider${status === 'success' ? ' is-success' : ''}${status === 'fail' ? ' is-fail' : ''}`}
                  style={{ left: `${moveLeft}px` }}
                  onMouseDown={onMouseDown}
                  onMouseMove={onMouseMove}
                  onMouseUp={onMouseUp}
                  onTouchStart={(event) => startDrag(event.touches[0]?.clientX ?? 0)}
                  onTouchEnd={endDrag}
                >
                  {status === 'success' ? '✓' : status === 'fail' ? '✕' : '⇢'}
                </div>
                <span className="pc-verify__tip">
                  {status === 'success' ? t('验证成功') : status === 'fail' ? t('验证失败，请重试') : t('拖动滑块完成拼图')}
                </span>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
