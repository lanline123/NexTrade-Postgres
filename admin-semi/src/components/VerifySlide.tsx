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
  /** 拖动开始时缓存的轨道宽度（拖动中不再逐帧读 clientWidth 触发强制同步布局） */
  const trackWidthRef = useRef(0);
  /** 待应用的拖动位置 + 已排队的动画帧句柄（mousemove 合帧用） */
  const pendingLeftRef = useRef<number | null>(null);
  const rafRef = useRef(0);

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
    trackWidthRef.current = trackRef.current?.clientWidth || IMG_WIDTH;
    // 拖动期间把全页光标钉为 grabbing（样式侧 body.pc-verify-dragging），指针划过任何子元素都不丢光标
    document.body.classList.add('pc-verify-dragging');
  };

  // mousemove 每秒可达上百次：位置只入队、每动画帧最多应用一次，
  // 且宽度用 startDrag 缓存值，不再逐帧读 clientWidth（旧实现的主要卡顿源，与 PC 版同修）
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

  const endDrag = () => {
    if (!draggingRef.current) return;
    draggingRef.current = false;
    document.body.classList.remove('pc-verify-dragging');
    if (rafRef.current) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = 0;
    }
    // 校验前先把排队中的最后一帧位置同步落下，保证提交坐标就是释放点
    if (pendingLeftRef.current !== null) {
      setMove(pendingLeftRef.current);
      pendingLeftRef.current = null;
    }
    if (statusRef.current !== 'moving') return;
    void verify(moveLeftRef.current);
  };

  // 处理器经 handlersRef 转发到最新一次渲染的 moveDrag/endDrag：旧实现直接闭包
  // 捕获打开瞬间的处理器，其 verify 闭包里的 captcha 还是 null，在面板外松手时
  // 校验永远发不出去（界面停在 moving 态、看起来像卡死），与 PC 版同修。
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

  const verify = useCallback(
    async (left: number) => {
      const token = captcha?.token;
      const secretKey = captcha?.secretKey;
      if (!token) return;
      setChecking(true);
      const imgWidth = trackWidthRef.current || trackRef.current?.clientWidth || IMG_WIDTH;
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
                    style={{ transform: `translateX(${moveLeft}px)` }}
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
