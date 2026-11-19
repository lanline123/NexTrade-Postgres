/**
 * 系统模式（我方新增，原版无此配置；见 giftcard-welfare-design/system-mode-design.md）。
 *
 * 后台「系统模式」设置存 config 表 biz_code=systemMode，值 `{ mode }`：
 *   - `all`（默认，全功能）：商城 + 礼品卡全开，即现状；
 *   - `giftcard`（礼品卡系统）：普通商城营销玩法入口收起（秒杀/拼团/优惠券等）；
 *   - `mall`（商城系统）：礼品卡整体收起（无购卡/兑换入口、无结算抵扣区）。
 *
 * 前台经 GET /api/common/config/systemMode 读取，由 layout 服务端预取 →
 * header-bootstrap 内联 → setSystemModeConfig 注入（与 buyStyle 同一条链路）。
 * 缺失/非法值一律归一化为 `all`，保证存量部署行为零变化。
 * 门控只做入口级渲染隐藏，不拦直达链接、不改任何业务数据。
 */

export type SystemMode = 'all' | 'giftcard' | 'mall';

export interface SystemModeConfig {
  mode: SystemMode;
}

/** 内置默认：全功能（与后端 system_mode.rs 默认一致）。 */
export const DEFAULT_SYSTEM_MODE: SystemModeConfig = { mode: 'all' };

let systemModeState: SystemModeConfig = DEFAULT_SYSTEM_MODE;
let systemModeHydrated = false;

function sanitizeSystemMode(input: unknown): SystemModeConfig | null {
  if (!input || typeof input !== 'object') return null;
  const raw = input as Record<string, unknown>;
  const mode = String(raw.mode ?? '').trim();
  return { mode: mode === 'giftcard' || mode === 'mall' ? mode : 'all' };
}

/** layout 服务端 bootstrap 注入系统模式配置（SSR 与客户端各调一次，内容一致）。 */
export function setSystemModeConfig(input: unknown): void {
  const clean = sanitizeSystemMode(input);
  if (clean) {
    systemModeState = clean;
    systemModeHydrated = true;
  }
}

/** 当前生效的系统模式（客户端首次使用时从 layout 内联 bootstrap 懒加载）。 */
export function getSystemMode(): SystemModeConfig {
  if (!systemModeHydrated && typeof document !== 'undefined') {
    systemModeHydrated = true;
    try {
      const raw = document.getElementById('pc-header-bootstrap')?.textContent;
      const parsed = raw ? (JSON.parse(raw) as any) : null;
      const clean = sanitizeSystemMode(parsed?.systemMode);
      if (clean) systemModeState = clean;
    } catch {
      /* bootstrap 缺失时用内置默认 all */
    }
  }
  return systemModeState;
}

/** 客户端兜底拉取（bootstrap 缺失时用；结果缓存进模块状态，失败保持默认 all）。 */
let systemModeFetchPromise: Promise<SystemModeConfig> | null = null;
export function fetchSystemMode(): Promise<SystemModeConfig> {
  if (!systemModeFetchPromise) {
    systemModeFetchPromise = fetch('/api/common/config/systemMode')
      .then((response) => (response.ok ? response.json() : null))
      .then((json) => {
        const clean = sanitizeSystemMode(json?.data ?? json);
        if (clean) {
          systemModeState = clean;
          systemModeHydrated = true;
        }
        return systemModeState;
      })
      .catch(() => systemModeState);
  }
  return systemModeFetchPromise;
}
