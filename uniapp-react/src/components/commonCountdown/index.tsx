/**
 * 通用倒计时（components 层入口，补齐旧 components/commonCountdown/index.vue 的位置）。
 * 实现本体已在装修模块层移植并被 countdown 模块与 groupon 列表复用
 * （pages/customPage/modules/commonCountdown），此处再导出为 components 级
 * 共享组件，避免其他页面反向引用 pages 路径。props 与旧组件一致：
 * countdownType/dailyTime/specifyTime/format/color/zh/valueStyle/timeStyle/unitStyle
 * + render-prop children({days,hours,minutes,seconds})。
 */
export { default } from "@/pages/customPage/modules/commonCountdown/index";
