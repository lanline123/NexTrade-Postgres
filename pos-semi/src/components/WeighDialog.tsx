import { useEffect, useState } from 'react';
import Dialog from './Dialog';
import { scaleManager, useScale } from '../bluetooth/ScaleManager';

// 称重录入弹窗：蓝牙秤实时读数与手工录入双通道。
// - 已连接蓝牙秤时默认跟随秤读数（可取消跟随改手工）；
// - 未连接/浏览器不支持时纯手工录入，功能完整（克/公斤/斤三单位）。
// 金额预览与后端 weigh_line_amount 同口径：整数分计算、四舍五入到分。

interface WeighDialogProps {
  open: boolean;
  productName: string;
  /** 计价单价（元/计价单位） */
  unitPrice: number;
  /** 计价单位克数（如 500 = 元/500g） */
  unitGrams: number;
  initialWeightGrams?: number;
  confirmText?: string;
  onClose: () => void;
  onConfirm: (weightGrams: number) => void;
}

type InputUnit = 'g' | 'kg' | 'jin';

const UNIT_GRAMS: Record<InputUnit, number> = { g: 1, kg: 1000, jin: 500 };
const UNIT_LABELS: Record<InputUnit, string> = { g: '克', kg: '公斤', jin: '斤' };

/** 行金额预览（分，整数运算）：单价（分）× 克数 ÷ 单位克数，四舍五入。 */
export function weighAmountCents(unitPrice: number, weightGrams: number, unitGrams: number): number {
  if (weightGrams <= 0 || unitGrams <= 0) return 0;
  const unitPriceCents = Math.round(unitPrice * 100);
  return Math.round((unitPriceCents * weightGrams) / unitGrams);
}

export default function WeighDialog({
  open,
  productName,
  unitPrice,
  unitGrams,
  initialWeightGrams,
  confirmText = '加入购物车',
  onClose,
  onConfirm,
}: WeighDialogProps) {
  const scale = useScale();
  const [unit, setUnit] = useState<InputUnit>('g');
  const [input, setInput] = useState('');
  const [followScale, setFollowScale] = useState(true);

  useEffect(() => {
    if (open) {
      setUnit('g');
      setInput(initialWeightGrams ? String(initialWeightGrams) : '');
      setFollowScale(true);
    }
  }, [open, initialWeightGrams]);

  const connected = scale.status === 'connected';
  const scaleGrams = connected && scale.reading ? scale.reading.weightGrams : null;
  const manualValue = Number(input);
  const manualGrams =
    Number.isFinite(manualValue) && manualValue > 0 ? manualValue * UNIT_GRAMS[unit] : 0;
  const effectiveGrams = followScale && scaleGrams !== null ? scaleGrams : Math.round(manualGrams);
  const amountCents = weighAmountCents(unitPrice, effectiveGrams, unitGrams);
  const unstable = followScale && scaleGrams !== null && scale.reading && !scale.reading.stable;

  return (
    <Dialog
      open={open}
      title={`称重 · ${productName}`}
      width={480}
      onClose={onClose}
      footer={
        <>
          <button type="button" className="el-button" onClick={onClose}>
            <span>取消</span>
          </button>
          <button
            type="button"
            className="el-button el-button--primary"
            disabled={effectiveGrams <= 0}
            onClick={() => onConfirm(Math.round(effectiveGrams))}
          >
            <span>{confirmText}</span>
          </button>
        </>
      }
    >
      <div className="member-row">
        <span>计价单价</span>
        <strong>
          ¥{Number(unitPrice || 0).toFixed(2)} / {unitGrams}克
        </strong>
      </div>

      <div className="weigh-live-box">
        {connected ? (
          <>
            <div className="weigh-live-head">
              <span>
                蓝牙秤：{scale.deviceName || '已连接'}
                {scale.reading ? (
                  <span className={scale.reading.stable ? 'weigh-stable' : 'weigh-unstable'}>
                    {scale.reading.stable ? ' ● 已稳定' : ' ● 读数跳动中'}
                  </span>
                ) : null}
              </span>
              <label className="weigh-follow">
                <input
                  type="checkbox"
                  checked={followScale}
                  onChange={(event) => setFollowScale(event.target.checked)}
                />
                跟随秤读数
              </label>
            </div>
            <div className="weigh-live-value">
              {scaleGrams !== null ? `${scaleGrams}` : '--'}
              <span className="weigh-live-unit">克</span>
            </div>
          </>
        ) : (
          <div className="weigh-live-head">
            <span>{scale.status === 'unsupported' ? '当前浏览器不支持蓝牙秤（可手工录入）' : '蓝牙秤未连接（可手工录入）'}</span>
            {scale.status !== 'unsupported' ? (
              <button type="button" className="el-button weigh-connect-btn" onClick={() => void scaleManager.connect()}>
                <span>连接蓝牙秤</span>
              </button>
            ) : null}
          </div>
        )}
      </div>

      <div className="price-types">
        {(Object.keys(UNIT_GRAMS) as InputUnit[]).map((key) => (
          <button
            key={key}
            type="button"
            className={unit === key ? 'price-type is-active' : 'price-type'}
            onClick={() => setUnit(key)}
          >
            {UNIT_LABELS[key]}
          </button>
        ))}
      </div>
      <div className="price-input">
        <div className="el-input">
          <div className="el-input__wrapper">
            <input
              className="el-input__inner"
              placeholder={`手工录入重量（${UNIT_LABELS[unit]}）`}
              value={input}
              inputMode="decimal"
              onChange={(event) => {
                setInput(event.target.value);
                setFollowScale(false);
              }}
            />
          </div>
        </div>
        <span className="price-unit">{UNIT_LABELS[unit]}</span>
      </div>

      <div className="member-row">
        <span>本次重量</span>
        <strong>{effectiveGrams > 0 ? `${Math.round(effectiveGrams)} 克` : '—'}</strong>
      </div>
      <div className="member-row">
        <span>行金额</span>
        <strong className="weigh-amount">¥{(amountCents / 100).toFixed(2)}</strong>
      </div>
      {unstable ? <div className="weigh-warn">秤读数尚未稳定，确认前请等读数稳定（变绿）</div> : null}
    </Dialog>
  );
}
