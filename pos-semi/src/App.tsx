import type { CSSProperties } from 'react';
import { Navigate, Route, Routes } from 'react-router-dom';
import Cashier from './pages/Cashier';
import Orders from './pages/Orders';
import Tables from './pages/Tables';
import Placeholder from './pages/Placeholder';
import { readSession } from './session';

const PRIMARY_STYLE = {
  '--el-button-text-color': '#fff',
  '--el-button-hover-text-color': '#fff',
  '--el-button-active-text-color': '#fff',
  '--el-button-bg-color': '#fe9b1c',
  '--el-button-active-bg-color': '#fe9b1c',
  '--el-button-active-border-color': '#fe9b1c',
  '--el-button-hover-bg-color': 'rgba(254, 155, 28, 0.8)',
  '--el-button-hover-border-color': 'rgba(254, 155, 28, 0.8)',
  '--tig-primary': '#fe9b1c',
} as CSSProperties;

export default function App() {
  const session = readSession();
  if (!session) return <SessionGate />;

  return (
    <div className="layout-basic">
      <div className="layout-main">
        <div className="layout-top">
          <div className="page-top shop-admin">
            <div className="page-top-warp">
              <div className="top-bar-left flex flex-align-center" />
              <div className="top-bar-right">
                <div className="top-bar-right flex flex-align-center">
                  <div className="el-select bound-device-select light">
                    <div className="el-select__wrapper is-filterable">
                      <div className="el-select__selection">
                        <div className="el-select__selected-item el-select__placeholder is-transparent">
                          <span>设备未绑定</span>
                        </div>
                      </div>
                      <div className="el-select__suffix">
                        <i className="el-icon el-select__caret">
                          <svg viewBox="0 0 1024 1024" width="1em" height="1em">
                            <path
                              fill="currentColor"
                              d="M831.872 340.864 512 652.672 192.128 340.864a30.59 30.59 0 0 0-42.752 0 29.12 29.12 0 0 0 0 41.6L489.664 714.24a32 32 0 0 0 44.672 0l340.288-331.712a29.12 29.12 0 0 0 0-41.728 30.59 30.59 0 0 0-42.752 0z"
                            />
                          </svg>
                        </i>
                      </div>
                    </div>
                  </div>

                  <div className="dialog-link" style={{ display: 'inline-flex' }}>
                    <button type="button" className="el-button el-button--primary el-button--medium is-plain handover-btn light">
                      <span>交班</span>
                    </button>
                  </div>

                  <div className="top-bar-item top-bar-user topMenu light">
                    <div className="admin-user-warp">
                      <a className="top-bar-btn">
                        <span className="admin-user-photo">
                          <span className="pos-avatar">收</span>
                        </span>
                        <div className="text">收银员：{session.username}</div>
                        <i className="iconfont icon-xiala" />
                      </a>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="layout-container no-padding">
          <div className="el-scrollbar layout-content-warp">
            <div className="el-scrollbar__wrap el-scrollbar__wrap--hidden-default">
              <div className="el-scrollbar__view">
                <Routes>
                  <Route path="/" element={<Cashier primaryStyle={PRIMARY_STYLE} />} />
                  <Route path="/tables" element={<Tables />} />
                  <Route path="/orders" element={<Orders />} />
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
              </div>
            </div>
          </div>
        </div>
      </div>
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
