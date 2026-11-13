import React from 'react';
import ReactDOM from 'react-dom/client';
import '@semi-css';
import './styles.css';
import App from './App';
import { I18nProvider } from './i18n';
import { applyAppearance, getStoredAppearance } from './theme';

/**
 * 首帧就把外观（主题色 --primary / --primary-soft）落到 <html> 上。
 *
 * 装修编辑器的进入动画（DecorateLoading）用的是 var(--primary)，而 App 要等异步
 * /setting/config/globalSettings 返回后才 applyAppearance —— 硬刷新直达
 * ?page=pcDecorateEditor 时，动画会先用 styles.css 的默认色闪一下再跳成主题色。
 * 本地缓存是同步可读的，先应用一次即可消除这次跳变；随后服务端返回的真值会覆盖它。
 * 没有缓存时什么都不做，保持 styles.css 的既有默认色不变。
 */
const storedAppearance = getStoredAppearance();
if (storedAppearance) applyAppearance(storedAppearance);

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider>
      <App />
    </I18nProvider>
  </React.StrictMode>,
);
