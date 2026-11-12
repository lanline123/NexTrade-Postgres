import type { PcSeckillFrameDecor } from "./pcSeckillModule";

/** 装修拖拽模块通用 payload（含可选 frame，其余字段由各业务组件解析） */
export type DecorateModulePayload = Record<string, unknown> & {
    frame?: PcSeckillFrameDecor;
};
