// 蓝牙秤连接管理（单例）：封装 Web Bluetooth 的选设备、GATT 连接、
// 通知订阅与断线感知，对上层只暴露状态快照与读数订阅。
//
// 注意 Web Bluetooth 的平台约束（均为浏览器规则，非本层缺陷）：
// - 选设备必须由用户手势触发（按钮点击），页面刷新后需重新连接，
//   无法静默自动重连已配对秤；
// - 仅 Chrome / Edge 且 HTTPS（或 localhost）下可用，不支持时
//   status = 'unsupported'，称重弹窗自动落到手工录入（功能完整）。

import { useEffect, useState } from 'react';
import {
  allScaleServiceUuids,
  scaleProtocolDef,
  type ScaleProtocol,
  type ScaleReading,
} from './protocols';

export type ScaleStatus = 'unsupported' | 'disconnected' | 'connecting' | 'connected';

export interface ScaleState {
  status: ScaleStatus;
  protocolId: string;
  deviceName: string;
  reading: ScaleReading | null;
  error: string;
  /** 当前协议是否提供软件去皮命令（内置协议均未提供，见 protocols.ts 说明） */
  canTare: boolean;
}

const PROTOCOL_STORAGE_KEY = 'posScaleProtocolId';

function storedProtocolId(): string {
  try {
    return localStorage.getItem(PROTOCOL_STORAGE_KEY) || 'ble-uart-text';
  } catch {
    return 'ble-uart-text';
  }
}

class ScaleManager {
  private state: ScaleState = {
    status: typeof navigator !== 'undefined' && !!navigator.bluetooth ? 'disconnected' : 'unsupported',
    protocolId: storedProtocolId(),
    deviceName: '',
    reading: null,
    error: '',
    canTare: false,
  };

  private listeners = new Set<() => void>();
  private device: BluetoothDevice | null = null;
  private characteristic: BluetoothRemoteGATTCharacteristic | null = null;
  private protocol: ScaleProtocol | null = null;

  private onValueChanged = (event: Event) => {
    const target = event.target as unknown as BluetoothRemoteGATTCharacteristic;
    if (!target.value || !this.protocol) return;
    const readings = this.protocol.feed(target.value);
    if (readings.length) {
      this.setState({ reading: readings[readings.length - 1] });
    }
  };

  private onDisconnected = () => {
    this.characteristic = null;
    this.protocol = null;
    this.setState({ status: 'disconnected', deviceName: '', reading: null });
  };

  private setState(patch: Partial<ScaleState>) {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }

  getState(): ScaleState {
    return this.state;
  }

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  setProtocol(protocolId: string) {
    try {
      localStorage.setItem(PROTOCOL_STORAGE_KEY, protocolId);
    } catch {
      // 存储不可用时仅影响默认协议记忆，不阻断连接
    }
    this.setState({ protocolId });
  }

  async connect(protocolId?: string): Promise<void> {
    if (!navigator.bluetooth) {
      this.setState({ status: 'unsupported', error: '当前浏览器不支持 Web Bluetooth，请使用 Chrome / Edge，或手工录入重量' });
      return;
    }
    const def = scaleProtocolDef(protocolId ?? this.state.protocolId);
    this.setProtocol(def.id);
    this.setState({ status: 'connecting', error: '' });
    try {
      // acceptAllDevices：秤的 GATT 服务多不在广播包内，按服务过滤会漏检，
      // 故全量列设备由收银员选，连接后再按候选端点逐个尝试。
      const device = await navigator.bluetooth.requestDevice({
        acceptAllDevices: true,
        optionalServices: allScaleServiceUuids(),
      });
      this.device = device;
      device.addEventListener('gattserverdisconnected', this.onDisconnected);
      const server = await device.gatt?.connect();
      if (!server) throw new Error('设备不支持 GATT 连接');
      let lastError: unknown = null;
      for (const endpoint of def.endpoints) {
        try {
          const service = await server.getPrimaryService(endpoint.serviceUuid);
          const characteristic = await service.getCharacteristic(endpoint.characteristicUuid);
          await characteristic.startNotifications();
          characteristic.addEventListener('characteristicvaluechanged', this.onValueChanged);
          this.characteristic = characteristic;
          this.protocol = def.create();
          this.setState({
            status: 'connected',
            deviceName: device.name || '蓝牙秤',
            reading: null,
            error: '',
            canTare: !!this.protocol.tareCommand,
          });
          return;
        } catch (error) {
          lastError = error;
        }
      }
      throw lastError instanceof Error ? lastError : new Error('未找到匹配的秤服务，请确认协议选择与秤型一致');
    } catch (error) {
      const messageText = error instanceof Error ? error.message : '连接蓝牙秤失败';
      // 用户在选设备弹窗点取消（NotFoundError）不算错误状态
      const cancelled = error instanceof Error && error.name === 'NotFoundError';
      this.setState({
        status: 'disconnected',
        error: cancelled ? '' : messageText,
      });
    }
  }

  async disconnect(): Promise<void> {
    try {
      this.characteristic?.removeEventListener('characteristicvaluechanged', this.onValueChanged);
      if (this.characteristic) await this.characteristic.stopNotifications().catch(() => undefined);
      this.device?.gatt?.disconnect();
    } finally {
      this.characteristic = null;
      this.protocol = null;
      this.setState({ status: 'disconnected', deviceName: '', reading: null });
    }
  }

  /** 软件去皮：仅当协议定义了命令字时可用（内置协议暂未提供）。 */
  async tare(): Promise<boolean> {
    if (!this.characteristic || !this.protocol?.tareCommand) return false;
    await this.characteristic.writeValue(this.protocol.tareCommand);
    return true;
  }
}

export const scaleManager = new ScaleManager();

/** React 订阅钩子：返回秤状态快照（状态变化时组件重渲染）。 */
export function useScale(): ScaleState {
  const [state, setState] = useState<ScaleState>(() => scaleManager.getState());
  useEffect(() => scaleManager.subscribe(() => setState(scaleManager.getState())), []);
  return state;
}
