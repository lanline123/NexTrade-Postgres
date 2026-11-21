/**
 * 摄像头扫码（BarcodeScanner）：通用组件，按钮 + 弹窗形态（我方新增）。
 *
 * 识别链：`getUserMedia` 取后置摄像头画面 → 原生 `BarcodeDetector`
 * 逐帧识别一维条码/二维码（Chrome / Edge / Android WebView 内置，
 * 无第三方依赖）。不支持 `BarcodeDetector` 的浏览器（Safari /
 * Firefox）在弹窗内明确提示，并保留手动输入条码兜底，流程不断。
 *
 * 已接入：商品创建/编辑（商品条码回填）、商品入库/出库（扫码添加，
 * 连续模式）。USB 扫码枪为键盘设备，不经本组件（聚焦输入框直录）。
 */

import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Button, Input, Modal, Space, Toast, Typography } from '@douyinfe/semi-ui';
import { IconCamera } from '@douyinfe/semi-icons';
import { useI18n } from '../i18n';

const { Text } = Typography;

/** BarcodeDetector 尚未进入 TS DOM 标准库，这里声明最小可用面。 */
interface DetectedBarcode {
  rawValue: string;
  format: string;
}
interface BarcodeDetectorLike {
  detect: (source: HTMLVideoElement) => Promise<DetectedBarcode[]>;
}
type BarcodeDetectorCtor = new (options?: { formats?: string[] }) => BarcodeDetectorLike;

const DETECTOR_FORMATS = ['ean_13', 'ean_8', 'upc_a', 'upc_e', 'code_128', 'code_39', 'code_93', 'itf', 'qr_code'];

type ScanStatus = 'starting' | 'scanning' | 'unsupported' | 'error';

export interface BarcodeScanButtonProps {
  /** 识别到条码（或手动确认输入）后回调，入参为去除首尾空白的码值 */
  onScan: (code: string) => void;
  /** 连续模式：识别后不关弹窗，同一码 1.5 秒内不重复回调（入库连扫用） */
  continuous?: boolean;
  disabled?: boolean;
  /** 按钮文案，默认「扫码」 */
  children?: ReactNode;
  size?: 'small' | 'default' | 'large';
  theme?: 'solid' | 'borderless' | 'light' | 'outline';
  type?: 'primary' | 'secondary' | 'tertiary' | 'warning' | 'danger';
}

export default function BarcodeScanButton({
  onScan,
  continuous = false,
  disabled,
  children,
  size = 'small',
  theme = 'light',
  type = 'primary',
}: BarcodeScanButtonProps) {
  const { t } = useI18n();
  const [visible, setVisible] = useState(false);
  const [status, setStatus] = useState<ScanStatus>('starting');
  const [manualCode, setManualCode] = useState('');
  const [lastRead, setLastRead] = useState('');
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const stopRef = useRef(true);
  const lastRef = useRef<{ code: string; at: number }>({ code: '', at: 0 });
  const onScanRef = useRef(onScan);
  onScanRef.current = onScan;

  const stopCamera = () => {
    stopRef.current = true;
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((track) => track.stop());
      streamRef.current = null;
    }
    if (videoRef.current) {
      videoRef.current.srcObject = null;
    }
  };

  useEffect(() => {
    if (!visible) return undefined;
    stopRef.current = false;
    setStatus('starting');
    setLastRead('');
    lastRef.current = { code: '', at: 0 };

    const ctor = (window as unknown as { BarcodeDetector?: BarcodeDetectorCtor }).BarcodeDetector;
    if (!ctor || !navigator.mediaDevices?.getUserMedia) {
      setStatus(!ctor ? 'unsupported' : 'error');
      return () => stopCamera();
    }

    let detector: BarcodeDetectorLike;
    try {
      detector = new ctor({ formats: DETECTOR_FORMATS });
    } catch {
      try {
        detector = new ctor();
      } catch {
        setStatus('unsupported');
        return () => stopCamera();
      }
    }

    const emit = (raw: string) => {
      const code = raw.trim();
      if (!code) return;
      const now = Date.now();
      if (code === lastRef.current.code && now - lastRef.current.at < 1500) return;
      lastRef.current = { code, at: now };
      try {
        navigator.vibrate?.(80);
      } catch {
        /* 忽略：部分浏览器 vibrate 需用户手势 */
      }
      if (continuous) {
        setLastRead(code);
        onScanRef.current(code);
      } else {
        stopCamera();
        setVisible(false);
        onScanRef.current(code);
      }
    };

    const loop = async () => {
      const video = videoRef.current;
      if (stopRef.current || !video) return;
      if (video.readyState >= 2) {
        try {
          const results = await detector.detect(video);
          if (results.length > 0 && results[0].rawValue) {
            emit(results[0].rawValue);
            if (!continuous) return;
          }
        } catch {
          /* 单帧识别失败忽略，继续下一帧 */
        }
      }
      if (!stopRef.current) {
        window.setTimeout(loop, 180);
      }
    };

    navigator.mediaDevices
      .getUserMedia({ video: { facingMode: 'environment' }, audio: false })
      .then((stream) => {
        if (stopRef.current) {
          stream.getTracks().forEach((track) => track.stop());
          return;
        }
        streamRef.current = stream;
        if (videoRef.current) {
          videoRef.current.srcObject = stream;
          videoRef.current.play().catch(() => undefined);
        }
        setStatus('scanning');
        loop();
      })
      .catch(() => {
        if (!stopRef.current) setStatus('error');
      });

    return () => stopCamera();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible, continuous]);

  const close = () => {
    stopCamera();
    setVisible(false);
  };

  const confirmManual = () => {
    const code = manualCode.trim();
    if (!code) {
      Toast.warning({ content: t('请输入条码') });
      return;
    }
    setManualCode('');
    if (continuous) {
      lastRef.current = { code, at: Date.now() };
      setLastRead(code);
      onScan(code);
    } else {
      close();
      onScan(code);
    }
  };

  return (
    <>
      <Button icon={<IconCamera />} size={size} theme={theme} type={type} disabled={disabled} onClick={() => setVisible(true)}>
        {children || t('扫码')}
      </Button>
      <Modal
        title={t('摄像头扫码')}
        visible={visible}
        onCancel={close}
        width={440}
        footer={
          <Space style={{ width: '100%', justifyContent: 'space-between' }}>
            <Space>
              <Input
                style={{ width: 190 }}
                placeholder={t('手动输入条码')}
                value={manualCode}
                onChange={setManualCode}
                onEnterPress={confirmManual}
                showClear
              />
              <Button theme="solid" type="primary" onClick={confirmManual}>
                {t('确定')}
              </Button>
            </Space>
            <Button onClick={close}>{t('关闭')}</Button>
          </Space>
        }
      >
        <div
          style={{
            position: 'relative',
            borderRadius: 8,
            overflow: 'hidden',
            background: '#111',
            aspectRatio: '4 / 3',
          }}
        >
          <video ref={videoRef} style={{ width: '100%', height: '100%', objectFit: 'cover' }} playsInline muted />
          <div
            style={{
              position: 'absolute',
              inset: '18% 12%',
              border: '2px solid rgba(255,255,255,0.75)',
              borderRadius: 8,
              pointerEvents: 'none',
            }}
          />
        </div>
        <div style={{ marginTop: 10, minHeight: 22 }}>
          {status === 'scanning' && (
            <Text type="tertiary">{lastRead ? t('已读取：{code}', { code: lastRead }) : t('将条码对准框内，即可自动读取')}</Text>
          )}
          {status === 'starting' && <Text type="tertiary">{t('正在打开摄像头…')}</Text>}
          {status === 'unsupported' && (
            <Text type="warning">{t('当前浏览器不支持摄像头识别，请使用手动输入或扫码枪')}</Text>
          )}
          {status === 'error' && (
            <Text type="warning">{t('摄像头打开失败，请检查浏览器权限或使用手动输入')}</Text>
          )}
        </div>
      </Modal>
    </>
  );
}
