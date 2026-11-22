/**
 * /list 首屏骨架屏。
 *
 * /list 与 /search 共用同一套取数与版式（见 app/list/page.tsx：直接以
 * pageType='/list' 复用 SearchPage），但 App Router 的 loading.tsx 只对
 * 所在路由段生效——此前只有 /search 有骨架，/list 跳转时整页无过渡反馈。
 * 这里直接复用 /search 的骨架组件（同一份版式壳），不另造第二份。
 */
export { default } from '../search/loading';
