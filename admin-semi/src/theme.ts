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
