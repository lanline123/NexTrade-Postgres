import { useState } from 'react';
import { api } from '../api';
import { toast } from '../toast';

interface PickupOrder {
  orderId: number;
  orderSn: string;
  consignee: string;
  mobile: string;
  totalAmount: string | number;
  orderStatusName: string;
  addTime: string;
}

export default function Verification() {
  const [code, setCode] = useState('');
  const [result, setResult] = useState<PickupOrder | null>(null);
  const [loading, setLoading] = useState(false);

  const writeOff = async () => {
    const pickupSn = code.trim();
    if (!pickupSn) {
      toast('请输入核销码', 'error');
      return;
    }
    setLoading(true);
    try {
      const order = await api.post<PickupOrder>('/order/order/writeOff', undefined, { pickupSn });
      setResult(order);
      toast('核销成功');
    } catch (error) {
      toast(error instanceof Error ? error.message : '核销失败', 'error');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="container pos-verify-panel">
      <h2>核销</h2>
      <p className="pos-verify-tip">输入提货码/核销码完成自提订单核销。</p>
      <div className="price-input pos-verify-search">
        <div className="el-input">
          <div className="el-input__wrapper">
            <input
              className="el-input__inner"
              placeholder="请输入核销码"
              value={code}
              onChange={(event) => setCode(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') void writeOff();
              }}
            />
          </div>
        </div>
        <button type="button" className="el-button el-button--primary" disabled={loading} onClick={() => void writeOff()}>
          核销
        </button>
      </div>

      {result ? (
        <div className="pos-verify-result">
          <div className="member-row"><span>订单号</span><strong>{result.orderSn}</strong></div>
          <div className="member-row"><span>下单时间</span><span>{result.addTime || '-'}</span></div>
          <div className="member-row"><span>客户</span><span>{result.consignee || '-'} {result.mobile || ''}</span></div>
          <div className="member-row"><span>金额</span><strong>¥{Number(result.totalAmount ?? 0).toFixed(2)}</strong></div>
          <div className="member-row"><span>状态</span><span>{result.orderStatusName || '-'}</span></div>
        </div>
      ) : null}
    </div>
  );
}
