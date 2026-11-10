export interface AppearanceSettings {
  navTheme?: string;
  primaryColor?: string;
  layout?: string;
  adminLightLogo?: string;
}

export const primaryColors: Record<string, { primary: string; soft: string }> = {
  blue: { primary: '#165dff', soft: '#e8f2ff' },
  red: { primary: '#f5222d', soft: '#fff1f0' },
  orange: { primary: '#faad14', soft: '#fff7e6' },
  green: { primary: '#52c41a', soft: '#f6ffed' },
  purple: { primary: '#722ed1', soft: '#f9f0ff' },
};

const APPEARANCE_KEY = 'admin_appearance_v1';

/** 登录页在拿到登录态之前无法请求 globalSettings，用上次保存的外观做主题色兜底。 */
export function saveAppearance(settings: AppearanceSettings) {
  try {
    localStorage.setItem(
      APPEARANCE_KEY,
      JSON.stringify({
        navTheme: settings.navTheme || 'light',
        primaryColor: settings.primaryColor || 'blue',
        layout: settings.layout || 'default',
      }),
    );
  } catch {
    // 忽略隐私模式等存储异常
  }
}

export function getStoredAppearance(): AppearanceSettings | null {
  try {
    const raw = localStorage.getItem(APPEARANCE_KEY);
    return raw ? (JSON.parse(raw) as AppearanceSettings) : null;
  } catch {
    return null;
  }
}

export function applyAppearance(settings: AppearanceSettings) {
  const theme = settings.navTheme === 'dark' ? 'dark' : 'light';
  if (theme === 'dark') {
    document.body.setAttribute('theme-mode', 'dark');
  } else {
    document.body.removeAttribute('theme-mode');
  }

  const color = primaryColors[settings.primaryColor || 'blue'] || primaryColors.blue;
  document.documentElement.style.setProperty('--primary', color.primary);
  document.documentElement.style.setProperty('--primary-soft', color.soft);

  document.documentElement.dataset.layout = settings.layout || 'default';
}
