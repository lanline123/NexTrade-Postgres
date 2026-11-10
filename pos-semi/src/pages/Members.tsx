import { useEffect, useState } from 'react';
import { api } from '../api';
import Dialog from '../components/Dialog';
import { toast } from '../toast';

interface MemberRow {
  userId: number;
  nickname: string;
  mobile: string;
  balance: string | number;
  points: string | number;
  addTime: string;
}

interface MemberPage {
  records: MemberRow[];
  total: number;
  page: number;
  size: number;
}

const PAGE_SIZE = 15;

export default function Members() {
  const [keyword, setKeyword] = useState('');
  const [rows, setRows] = useState<MemberRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [rechargeTarget, setRechargeTarget] = useState<MemberRow | null>(null);
  const [rechargeValue, setRechargeValue] = useState('');

  const load = async (nextPage: number, nextKeyword: string) => {
    setLoading(true);
    try {
      const result = await api.post<MemberPage>('/pos/user/list', {
        page: nextPage,
        size: PAGE_SIZE,
        keyword: nextKeyword.trim() || undefined,
      });
      setRows(result.records || []);
      setTotal(result.total || 0);
      setPage(result.page || nextPage);
    } catch (error) {
      toast(error instanceof Error ? error.message : '会员加载失败', 'error');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load(1, '');
  }, []);

  const submitRecharge = async () => {
    if (!rechargeTarget) return;
    const value = Number(rechargeValue);
    if (!Number.isFinite(value) || value <= 0) {
      toast('请输入正确的充值金额', 'error');
      return;
    }
    try {
      await api.post('/pos/user/rechargeOrder/create', { userId: rechargeTarget.userId, money: value });
      setRechargeTarget(null);
      await load(page, keyword);
      toast('充值成功');
    } catch (error) {
      toast(error instanceof Error ? error.message : '充值失败', 'error');
    }
  };

  const totalPage = Math.max(1, Math.ceil(total / PAGE_SIZE));

  return (
    <div className="container pos-member-panel">
      <div className="pos-order-toolbar">
        <div className="price-input pos-member-search">
          <div className="el-input">
            <div className="el-input__wrapper">
              <input
                className="el-input__inner"
                placeholder="手机号 / 昵称 / 用户ID"
                value={keyword}
                onChange={(event) => setKeyword(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') void load(1, keyword);
                }}
              />
            </div>
          </div>
          <button type="button" className="el-button" onClick={() => void load(1, keyword)}>
            查询
          </button>
        </div>
        <span>共 {total} 位会员</span>
      </div>

      <table className="pos-order-table">
        <thead>
          <tr>
            <th>用户ID</th>
            <th>昵称</th>
            <th>手机号</th>
            <th>余额</th>
            <th>积分</th>
            <th>注册时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.userId}>
              <td>{row.userId}</td>
              <td>{row.nickname || '-'}</td>
              <td>{row.mobile || '-'}</td>
              <td>¥{Number(row.balance ?? 0).toFixed(2)}</td>
              <td>{Number(row.points ?? 0)}</td>
              <td>{row.addTime || '-'}</td>
              <td>
                <button type="button" className="el-button" onClick={() => { setRechargeTarget(row); setRechargeValue(''); }}>
                  充值
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {!loading && rows.length === 0 ? <div className="pos-order-empty">暂无会员</div> : null}

      <div className="pos-order-pager">
        <button type="button" className="el-button" disabled={page <= 1} onClick={() => void load(page - 1, keyword)}>
          上一页
        </button>
        <span>
          {page} / {totalPage}
        </span>
        <button type="button" className="el-button" disabled={page >= totalPage} onClick={() => void load(page + 1, keyword)}>
          下一页
        </button>
      </div>

      <Dialog
        open={!!rechargeTarget}
        title="会员充值"
        width={460}
        onClose={() => setRechargeTarget(null)}
        footer={
          <>
            <button type="button" className="el-button" onClick={() => setRechargeTarget(null)}>
              <span>取消</span>
            </button>
            <button type="button" className="el-button el-button--primary" onClick={() => void submitRecharge()}>
              <span>确认充值</span>
            </button>
          </>
        }
      >
        <div className="member-row">
          <span>会员</span>
          <strong>{rechargeTarget?.nickname || rechargeTarget?.mobile || '-'}</strong>
        </div>
        <div className="member-row">
          <span>当前余额</span>
          <strong>¥{Number(rechargeTarget?.balance ?? 0).toFixed(2)}</strong>
        </div>
        <div className="price-input">
          <div className="el-input">
            <div className="el-input__wrapper">
              <input
                className="el-input__inner"
                placeholder="请输入充值金额"
                value={rechargeValue}
                onChange={(event) => setRechargeValue(event.target.value)}
              />
            </div>
          </div>
          <span className="price-unit">元</span>
        </div>
      </Dialog>
    </div>
  );
}
