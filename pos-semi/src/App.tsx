import type { CSSProperties } from 'react';
import { useEffect, useState } from 'react';
import { Navigate, Route, Routes } from 'react-router-dom';
import { api } from './api';
import Dialog from './components/Dialog';
import Cashier from './pages/Cashier';
import Members from './pages/Members';
import Orders from './pages/Orders';
import Tables from './pages/Tables';
import Verification from './pages/Verification';
import Placeholder from './pages/Placeholder';
import Reservations from './pages/Reservations';
import { readSession } from './session';
import { toast } from './toast';

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

interface Device {
  id: number;
  deviceName?: string;
  deviceCode?: string;
  status?: number;
}

interface ShiftStats {
  orderCount?: number;
  totalAmount?: string | number;
  today?: boolean;
}

export default function App() {
  const session = readSession();
  const [devices, setDevices] = useState<Device[]>([]);
  const [deviceMenuOpen, setDeviceMenuOpen] = useState(false);
  const [deviceId, setDeviceId] = useState(() => Number(localStorage.getItem('posDeviceId') || 0));
  const [shiftOpen, setShiftOpen] = useState(false);
  const [shiftStats, setShiftStats] = useState<ShiftStats | null>(null);
  const [shiftBusy, setShiftBusy] = useState(false);

  useEffect(() => {
    if (!session) return;
    api
      .get<Device[]>('/pos/device/all')
      .then((list) => setDevices(Array.isArray(list) ? list : []))
      .catch(() => undefined);
  }, [session]);

  if (!session) return <SessionGate />;

  const selectedDevice = devices.find((item) => item.id === deviceId);

  const chooseDevice = (device: Device) => {
    localStorage.setItem('posDeviceId', String(device.id));
    localStorage.setItem('posDeviceCode', device.deviceCode || '');
    setDeviceId(device.id);
    setDeviceMenuOpen(false);
    toast(`已选择设备：${device.deviceName || device.deviceCode || device.id}`);
  };

  const openShift = async () => {
    setShiftOpen(true);
    try {
      const stats = await api.get<ShiftStats>('/pos/posSwitch/posStatistics');
      setShiftStats(stats || null);
    } catch {
      setShiftStats(null);
    }
  };

  const confirmShift = async () => {
    setShiftBusy(true);
    try {
      await api.post('/pos/posSwitch/workOff');
      await api.post('/pos/posSwitch/work');
      setShiftOpen(false);
      toast('交班成功，已开始新班次');
    } catch (error) {
      toast(error instanceof Error ? error.message : '交班失败', 'error');
    } finally {
      setShiftBusy(false);
    }
  };

  return (
    <div className="layout-basic">
      <div className="layout-main">
        <div className="layout-top">
          <div className="page-top shop-admin">
            <div className="page-top-warp">
              <div className="top-bar-left flex flex-align-center" />
              <div className="top-bar-right">
                <div className="top-bar-right flex flex-align-center">
                  <div
                    className="el-select bound-device-select light pos-device-select"
                    onClick={() => {
                      setDeviceMenuOpen((open) => !open);
                      if (!devices.length) {
                        api
                          .get<Device[]>('/pos/device/all')
                          .then((list) => setDevices(Array.isArray(list) ? list : []))
                          .catch(() => undefined);
                      }
                    }}
                  >
                    <div className="el-select__wrapper is-filterable">
                      <div className="el-select__selection">
                        <div className="el-select__selected-item el-select__placeholder is-transparent">
                          <span>{selectedDevice?.deviceName || selectedDevice?.deviceCode || '设备未绑定'}</span>
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
                    {deviceMenuOpen ? (
                      <div className="pos-device-menu" onClick={(event) => event.stopPropagation()}>
                        {devices.length === 0 ? (
                          <div className="pos-device-empty">暂无设备，请到后台「设备管理」添加</div>
                        ) : (
                          devices.map((device) => (
                            <button
                              key={device.id}
                              type="button"
                              className={device.id === deviceId ? 'pos-device-item is-active' : 'pos-device-item'}
                              onClick={() => chooseDevice(device)}
                            >
                              {device.deviceName || device.deviceCode || `设备 ${device.id}`}
                            </button>
                          ))
                        )}
                      </div>
                    ) : null}
                  </div>

                  <div className="dialog-link" style={{ display: 'inline-flex' }}>
                    <button
                      type="button"
                      className="el-button el-button--primary el-button--medium is-plain handover-btn light"
                      onClick={() => void openShift()}
                    >
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
                  <Route path="/reservation" element={<Reservations />} />
                  <Route path="/verification" element={<Verification />} />
                  <Route path="/returns" element={<Placeholder title="售后" note="退货退款与售后单。" />} />
                  <Route path="/members" element={<Members />} />
                  <Route path="*" element={<Navigate to="/" replace />} />
                </Routes>
              </div>
            </div>
          </div>
        </div>
      </div>

      <Dialog
        open={shiftOpen}
        title="交班"
        width={460}
        onClose={() => setShiftOpen(false)}
        footer={
          <>
            <button type="button" className="el-button" onClick={() => setShiftOpen(false)}>
              <span>取消</span>
            </button>
            <button
              type="button"
              className="el-button el-button--primary"
              style={PRIMARY_STYLE}
              disabled={shiftBusy}
              onClick={() => void confirmShift()}
            >
              <span>确认交班</span>
            </button>
          </>
        }
      >
        <div className="member-row">
          <span>当前收银员</span>
          <strong>{session.username}</strong>
        </div>
        <div className="member-row">
          <span>今日订单</span>
          <strong>{shiftStats?.orderCount ?? 0}</strong>
        </div>
        <div className="member-row">
          <span>今日金额</span>
          <strong>¥{Number(shiftStats?.totalAmount ?? 0).toFixed(2)}</strong>
        </div>
        <p className="confirm-text">交班后将结束当前班次并开始新班次，是否继续？</p>
      </Dialog>
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
