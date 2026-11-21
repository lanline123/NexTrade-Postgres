// 蓝牙秤协议注册表（可插拔）：新增秤协议只需在此登记一个
// ScaleProtocolDef 并实现其解析器，ScaleManager 与称重弹窗零改动。
//
// 已内置两种协议：
// 1. ble-uart-text —— 通用 BLE 串口文本协议。大量国产蓝牙秤/秤透传模块
//    （HM-10 类 FFE0/FFE1、FFF0/FFF1）以 ASCII 行流持续推送重量，
//    典型帧形如 `ST,GS,  0.750kg`、`+  0.750 kg`、`US,  0.755kg`。
//    解析：按行切分 → 提取「数字 + 单位（kg/g/lb/oz）」换算为克；
//    行内含 US/UNSTABLE 记未稳，其余记稳定；无单位的纯数字行按 kg 解
//    （多数台秤默认输出 kg，此点为不确定项，见线报告）。
// 2. weight-scale-service —— Bluetooth SIG 标准称重服务（0x181D）的
//    Weight Measurement 特征（0x2A9D），健康秤类设备常用。二进制格式
//    按 SIG 规范解析：flags bit0 = 英制（lb，刻度 0.01 lb），
//    否则 SI（kg，刻度 0.005 kg = 5g）；该特征为事件式最终值，恒记稳定。
//
// 去皮/置零命令未内置：串口秤命令字各厂商不一（T / TARE / Z 等均有），
// 标准称重服务无此命令。协议定义预留 tareCommand/zeroCommand 字段，
// 待真机确认具体秤型后按厂商补入即可（秤体按键去皮不依赖软件）。

export interface ScaleReading {
  /** 重量（克，整数） */
  weightGrams: number;
  /** 读数是否稳定（未稳读数不应直接确认入账，弹窗会提示） */
  stable: boolean;
}

export interface ScaleEndpoint {
  serviceUuid: string;
  characteristicUuid: string;
}

export interface ScaleProtocol {
  /** 协议内有状态解析（一连接一实例）：喂入通知数据块，产出完整读数 */
  feed(chunk: DataView): ScaleReading[];
  tareCommand?: Uint8Array;
  zeroCommand?: Uint8Array;
}

export interface ScaleProtocolDef {
  id: string;
  name: string;
  description: string;
  /** 候选 GATT 端点（按序尝试，首个可用的生效） */
  endpoints: ScaleEndpoint[];
  create(): ScaleProtocol;
}

const FFE0 = '0000ffe0-0000-1000-8000-00805f9b34fb';
const FFE1 = '0000ffe1-0000-1000-8000-00805f9b34fb';
const FFF0 = '0000fff0-0000-1000-8000-00805f9b34fb';
const FFF1 = '0000fff1-0000-1000-8000-00805f9b34fb';
const WEIGHT_SCALE_SERVICE = '0000181d-0000-1000-8000-00805f9b34fb';
const WEIGHT_MEASUREMENT = '00002a9d-0000-1000-8000-00805f9b34fb';

const UNIT_TO_GRAMS: Record<string, number> = {
  kg: 1000,
  g: 1,
  lb: 453.59237,
  oz: 28.349523125,
};

/** 解析一行串口文本为读数；无法解析返回 null。导出供单测/排障复用。 */
export function parseScaleTextLine(line: string): ScaleReading | null {
  const text = line.trim();
  if (!text) return null;
  const matched = text.match(/([+-]?\d+(?:\.\d+)?)\s*(kg|g|lb|oz)?/i);
  if (!matched) return null;
  const value = Number(matched[1]);
  if (!Number.isFinite(value)) return null;
  const unit = (matched[2] || 'kg').toLowerCase();
  const grams = value * (UNIT_TO_GRAMS[unit] ?? 1000);
  const stable = !/\bUS\b|UNSTABLE/i.test(text);
  return { weightGrams: Math.round(grams), stable };
}

function createUartTextProtocol(): ScaleProtocol {
  const decoder = new TextDecoder();
  let buffer = '';
  return {
    feed(chunk: DataView): ScaleReading[] {
      buffer += decoder.decode(new Uint8Array(chunk.buffer, chunk.byteOffset, chunk.byteLength), {
        stream: true,
      });
      const lines = buffer.split(/[\r\n]+/);
      buffer = lines.pop() ?? '';
      const readings: ScaleReading[] = [];
      for (const line of lines) {
        const reading = parseScaleTextLine(line);
        if (reading) readings.push(reading);
      }
      return readings;
    },
  };
}

function createWeightScaleProtocol(): ScaleProtocol {
  return {
    feed(chunk: DataView): ScaleReading[] {
      if (chunk.byteLength < 3) return [];
      const flags = chunk.getUint8(0);
      const imperial = (flags & 0x01) !== 0;
      const raw = chunk.getUint16(1, true);
      const grams = imperial ? raw * 0.01 * UNIT_TO_GRAMS.lb : raw * 5;
      return [{ weightGrams: Math.round(grams), stable: true }];
    },
  };
}

export const SCALE_PROTOCOLS: ScaleProtocolDef[] = [
  {
    id: 'ble-uart-text',
    name: '通用串口文本秤（FFE0/FFF0）',
    description: 'ASCII 行流推送重量，适配多数国产蓝牙台秤/透传模块',
    endpoints: [
      { serviceUuid: FFE0, characteristicUuid: FFE1 },
      { serviceUuid: FFF0, characteristicUuid: FFF1 },
    ],
    create: createUartTextProtocol,
  },
  {
    id: 'weight-scale-service',
    name: '标准称重服务（Bluetooth SIG 0x181D）',
    description: 'Weight Measurement 特征，适配健康秤类标准 BLE 设备',
    endpoints: [{ serviceUuid: WEIGHT_SCALE_SERVICE, characteristicUuid: WEIGHT_MEASUREMENT }],
    create: createWeightScaleProtocol,
  },
];

export function scaleProtocolDef(id: string): ScaleProtocolDef {
  return SCALE_PROTOCOLS.find((def) => def.id === id) ?? SCALE_PROTOCOLS[0];
}

/** 全部候选服务 UUID（连接时声明 optionalServices 用） */
export function allScaleServiceUuids(): string[] {
  const uuids = new Set<string>();
  for (const def of SCALE_PROTOCOLS) {
    for (const endpoint of def.endpoints) uuids.add(endpoint.serviceUuid);
  }
  return [...uuids];
}
