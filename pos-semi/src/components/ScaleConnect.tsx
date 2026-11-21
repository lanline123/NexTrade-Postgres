import { useState } from 'react';
import Dialog from './Dialog';
import { scaleManager, useScale } from '../bluetooth/ScaleManager';
import { SCALE_PROTOCOLS } from '../bluetooth/protocols';

// 蓝牙秤连接入口（收银工具栏按钮 + 连接弹窗）：选协议 → 连接 → 状态/读数。
// 浏览器规则：每次打开页面都需点「连接」重新选设备，无法静默重连。

export default function ScaleConnect() {
  const scale = useScale();
  const [open, setOpen] = useState(false);
  const connected = scale.status === 'connected';
  const connecting = scale.status === 'connecting';

  return (
    <>
      <button type="button" className="el-button btn" onClick={() => setOpen(true)}>
        <span>
          <span className={connected ? 'scale-dot is-on' : 'scale-dot'} />
          蓝牙秤
        </span>
      </button>
      <Dialog
        open={open}
        title="蓝牙秤"
        width={460}
        onClose={() => setOpen(false)}
        footer={
          <button type="button" className="el-button" onClick={() => setOpen(false)}>
            <span>关闭</span>
          </button>
        }
      >
        <div className="member-row">
          <span>秤协议</span>
          <select
            className="scale-protocol-select"
            value={scale.protocolId}
            disabled={connected || connecting}
            onChange={(event) => scaleManager.setProtocol(event.target.value)}
          >
            {SCALE_PROTOCOLS.map((def) => (
              <option key={def.id} value={def.id}>
                {def.name}
              </option>
            ))}
          </select>
        </div>
        <div className="member-row">
          <span>状态</span>
          <strong>
            {scale.status === 'unsupported'
              ? '当前浏览器不支持（请用 Chrome / Edge）'
              : connected
                ? `已连接：${scale.deviceName}`
                : connecting
                  ? '连接中…'
                  : '未连接'}
          </strong>
        </div>
        {connected && scale.reading ? (
          <div className="member-row">
            <span>当前读数</span>
            <strong>
              {scale.reading.weightGrams} 克{scale.reading.stable ? '（已稳定）' : '（跳动中）'}
            </strong>
          </div>
        ) : null}
        {scale.error ? <div className="weigh-warn">{scale.error}</div> : null}
        <div className="scale-actions">
          {connected ? (
            <button type="button" className="el-button" onClick={() => void scaleManager.disconnect()}>
              <span>断开</span>
            </button>
          ) : (
            <button
              type="button"
              className="el-button el-button--primary"
              disabled={connecting || scale.status === 'unsupported'}
              onClick={() => void scaleManager.connect()}
            >
              <span>{connecting ? '连接中…' : '连接蓝牙秤'}</span>
            </button>
          )}
        </div>
        <div className="pending-tip">提示：浏览器限制，刷新页面后需重新连接；去皮请用秤体按键。</div>
      </Dialog>
    </>
  );
}
