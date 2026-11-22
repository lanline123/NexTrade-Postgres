/**
 * /item/[id] 与 /product/[id] 是同一详情页（page.tsx 直接 re-export），
 * 骨架同样复用一份，不另造。
 */
export { default } from '../../product/[id]/loading';
