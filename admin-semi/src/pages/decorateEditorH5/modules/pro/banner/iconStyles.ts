/**
* 首焦轮播搜索行默认图标 → IconPark 映射（全站唯一一份，Settings 选择器与 Preview 共用）。
*
* 原版口径：menuIcon 1–7 / signIcon 1–4 为编号样式，'custom' 为自定义图、
* 0 为不显示，默认图走原版 defaultIconImage（oss.tigshop.com 的 11 张图）。
* 我方改口径（2026-11-15 全站图标换 IconPark 的既定口径，
* 用户 2026-11-19 拍板沿用）：默认样式不再使用 tigshop 默认图，改为 IconPark 字形
* （经 components/FloorIcon 内嵌 SVG 渲染），保证「面板里选的 = 预览看到的 =
* 前台渲染的」三处同一套字形；字段值域与数据结构不变，'custom' 自定义图
* （menuPic / signPic）与 0 不显示的口径不变。
* 前台（uniapp）渲染按同一映射表对齐，随前端批次落地。
*/

/** 菜单图标 menuIcon 1–7 → IconPark 名 */
export const MENU_ICON_PARK = ["hamburger-button", "application-menu", "list", "list-one", "list-two", "grid-nine", "more-app"];

/** 签到图标 signIcon 1–4 → IconPark 名 */
export const SIGN_ICON_PARK = ["calendar", "plan", "schedule", "calendar-dot"];

/** 编号值（1 起）→ `iconpark:<name>` 值；非编号（custom / 0 / 越界）返回 "" */
export function menuIconParkValue(value: unknown): string {
const name = MENU_ICON_PARK[Number(value) - 1];
return name? `iconpark:${name}`: "";
}

export function signIconParkValue(value: unknown): string {
const name = SIGN_ICON_PARK[Number(value) - 1];
return name? `iconpark:${name}`: "";
}
