import { NavLink, Navigate, Route, Routes } from 'react-router-dom';
import Cashier from './pages/Cashier';
import Placeholder from './pages/Placeholder';
import { readSession } from './session';

const NAV_ITEMS = [
  { to: '/', label: '收银', end: true },
  { to: '/tables', label: '桌台' },
  { to: '/orders', label: '订单' },
  { to: '/reservation', label: '预约' },
  { to: '/verification', label: '核销' },
  { to: '/returns', label: '售后' },
  { to: '/members', label: '会员' },
];

export default function App() {
  const session = readSession();
  if (!session) return <SessionGate />;

  return (
    <div className="pos-app">
      <header className="pos-header">
        <div className="pos-brand">
          <strong>NexTrade</strong>
          <span>POS 收银</span>
        </div>
        <nav className="pos-nav">
          {NAV_ITEMS.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              className={({ isActive }) => (isActive ? 'pos-nav-item is-active' : 'pos-nav-item')}
            >
              {item.label}
            </NavLink>
          ))}
        </nav>
        <div className="pos-account">门店 {session.shopId ?? '未绑定'}</div>
      </header>

      <main className="pos-main">
        <Routes>
          <Route path="/" element={<Cashier />} />
          <Route
            path="/tables"
            element={<Placeholder title="桌台" note="桌台开台、并台、换台与结账，接口 /pos/table/*。" />}
          />
          <Route
            path="/orders"
            element={<Placeholder title="订单" note="挂单、取单与退单，接口 /pos/cart/pendingOrder*、/pos/order/*。" />}
          />
          <Route
            path="/reservation"
            element={<Placeholder title="预约" note="预约与库存日历，接口 /pos/order/OrderReservation/*。" />}
          />
          <Route path="/verification" element={<Placeholder title="核销" note="券码与提货核销。" />} />
          <Route path="/returns" element={<Placeholder title="售后" note="退货退款与售后单。" />} />
          <Route
            path="/members"
            element={<Placeholder title="会员" note="会员查询、充值与优惠券，接口 /pos/posManager/*、/pos/user/*。" />}
          />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
    </div>
  );
}

function SessionGate() {
  const adminUrl = import.meta.env.VITE_ADMIN_URL || '/';
  return (
    <div className="pos-gate">
      <h1>收银台未登录</h1>
      <p>请先登录管理后台，再从「收银工作台」进入收银台。</p>
      <a href={adminUrl}>前往管理后台</a>
    </div>
  );
}
