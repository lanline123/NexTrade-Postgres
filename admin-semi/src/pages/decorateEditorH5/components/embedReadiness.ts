/**
* 嵌入页就绪状态机（2026-11-19 加载门批次）
*
* 编辑器入口加载门的眼目：把「iframe 里到底渲染出来没有」从 IframeCanvas 的
* 内部状态提升为编辑器入口可订阅的共享状态。信号全部来自现有 postMessage
* 协议，不新增消息：
* - iframeLoaded：iframe 元素 load 事件（嵌入文档与入口资源到位）；
* - embedReady：嵌入页回 decorate-ready（桥接已装好、能收 decorate-data）；
* - firstFrameRendered：当前「数据纪元」内真实数据已下发、且其后的首个
* decorate-layout（嵌入页渲染完成后回传的布局测量）已到达 —— 等价于
* 「首帧内容已渲染」。
*
* 数据纪元（epoch）：编辑器每把一份完整页面数据写入 store（挂载初始化、
* 详情加载成功）就 beginEmbedDataEpoch() 推进一格。firstFrameRendered 只认
* 当前纪元内「下发之后」到达的 layout：挂载初期编辑器会先以空结构挂载画布
* 与嵌入页并行预加载，那一轮 layout 不能误开加载门。
*
* 将来 uniapp 侧若新增「渲染完成」上行消息
* （decorate-rendered，最好回带 decorate-data 的 seq），只需把 IframeCanvas
* 里调用 markEmbedLayout 的位置换成新消息的处理，本文件与加载门判定不动。
*/
import { useSyncExternalStore} from "react";

export interface EmbedReadiness {
/** iframe 文档 load 事件已触发 */
iframeLoaded: boolean;
/** 嵌入页已回 decorate-ready */
embedReady: boolean;
/** 当前数据纪元的首帧已渲染（layout 已回传） */
firstFrameRendered: boolean;
}

const INITIAL: EmbedReadiness = { iframeLoaded: false, embedReady: false, firstFrameRendered: false};

let readiness: EmbedReadiness = {...INITIAL};
const listeners = new Set<() => void>();

function setReadiness(patch: Partial<EmbedReadiness>) {
const next = {...readiness,...patch};
if (
next.iframeLoaded === readiness.iframeLoaded &&
next.embedReady === readiness.embedReady &&
next.firstFrameRendered === readiness.firstFrameRendered
) {
return;
}
readiness = next;
listeners.forEach((l) => l());
}

/** 当前数据纪元序号（编辑器每写入一份完整页面数据 +1） */
let epoch = 0;
/** 已确认首帧渲染的纪元序号 */
let framedEpoch = 0;
/** 推进纪元时的下发序号基准：layout 必须对应基准之后的下发才算本纪元首帧 */
let epochBaseSeq = 0;
/** decorate-data 累计下发序号（每次下发 +1，并随消息体 seq 字段下发供回带） */
let dataSeq = 0;
/** 最近一次 decorate-data 的下发时刻（无 seq 回带时的顺序判定兜底） */
let lastDataSentAt = 0;

/** 编辑器挂载 / 画布卸载时复位：新会话从零开始判定 */
export function resetEmbedReadiness() {
epoch = 0;
framedEpoch = 0;
epochBaseSeq = 0;
dataSeq = 0;
lastDataSentAt = 0;
setReadiness({...INITIAL});
}

/** 一份完整页面数据已写入 store：推进纪元，本纪元首帧需重新确认 */
export function beginEmbedDataEpoch() {
epoch += 1;
epochBaseSeq = dataSeq;
if (framedEpoch < epoch) setReadiness({ firstFrameRendered: false});
}

export function markEmbedIframeLoaded() {
setReadiness({ iframeLoaded: true});
}

export function markEmbedReady() {
setReadiness({ embedReady: true});
}

/** 每次 decorate-data 下发时调用，返回本次序号（写入消息体 seq 供嵌入页回带） */
export function markEmbedDataSent(): number {
dataSeq += 1;
lastDataSentAt = Date.now();
return dataSeq;
}

/**
* 嵌入页回传 decorate-layout 时调用。echoSeq 为嵌入页回带的下发序号（现行
* 嵌入页不回带则为 undefined）：有回带时要求其已越过本纪元基准；无回带时以
* 「本纪元内已有下发、且 layout 在该下发之后到达」的事件顺序兜底（JS 单
* 线程，消息处理必然在 sendData 执行之后被调用，顺序成立）。
*/
export function markEmbedLayout(echoSeq?: number) {
if (epoch === 0 || framedEpoch >= epoch) return;
if (dataSeq <= epochBaseSeq) return; // 本纪元还没下发过数据，这份 layout 对应旧数据
if (typeof echoSeq === "number") {
if (echoSeq <= epochBaseSeq) return;
} else if (lastDataSentAt === 0) {
return;
}
framedEpoch = epoch;
setReadiness({ firstFrameRendered: true});
}

export function useEmbedReadiness(): EmbedReadiness {
return useSyncExternalStore(
(cb) => {
listeners.add(cb);
return () => {
listeners.delete(cb);
};
},
() => readiness,
() => readiness
);
}
