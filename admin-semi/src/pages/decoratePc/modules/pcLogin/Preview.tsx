import { useEffect, useState } from 'react';
import { apiGet } from '../../../../api';
import { useI18n } from '../../../../i18n';

const PC_LOGIN_DEFAULTS: Record<string, any> = {
  loginMode: 2,
  headerHeight: 70,
  logoWidth: 140,
  logoHeight: 40,
  headerShadow: true,
  showWelcome: true,
  backgroundPic: { picUrl: '', picThumb: '' },
  backgroundColor: '',
  warpHeight: 620,
};

export default function PcLoginModulePreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const d = { ...PC_LOGIN_DEFAULTS, ...(data || {}) };
  const [shopLogo, setShopLogo] = useState('');
  const loginMode = Number(d.loginMode) === 1 ? 1 : 2;
  const headerHeight = Number(d.headerHeight) || 70;
  const warpHeight = Math.max(620, Number(d.warpHeight) || 620);
  const logoWidth = Number(d.logoWidth) || 140;
  const logoHeight = Number(d.logoHeight) || 40;

  useEffect(() => {
    let alive = true;
    apiGet<any>('/setting/config/basicSettings')
      .then((res) => alive && setShopLogo(String(res?.shopLogo || '')))
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, []);

  const wechatIcon = (
    <span className="dec-login-wechat" aria-label="wechat">
      <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
        <path d="M11.176 14.429c-2.665 0-4.826-1.8-4.826-4.018 0-2.22 2.159-4.02 4.824-4.02S16 8.191 16 10.411c0 1.21-.65 2.301-1.666 3.036a.324.324 0 0 0-.12.366l.218.81a.616.616 0 0 1 .029.117.166.166 0 0 1-.162.162.177.177 0 0 1-.092-.03l-1.057-.61a.519.519 0 0 0-.256-.074.509.509 0 0 0-.142.021 5.668 5.668 0 0 1-1.576.22ZM0 6.826c0 1.455.781 2.765 2.001 3.656a.385.385 0 0 1 .143.439l-.161.6-.1.373a.499.499 0 0 0-.032.14.192.192 0 0 0 .193.193.213.213 0 0 0 .111-.036l1.268-.733a.622.622 0 0 1 .308-.088.62.62 0 0 1 .17.025 6.83 6.83 0 0 0 1.625.26 4.45 4.45 0 0 1-.177-1.251c0-2.936 2.785-5.02 5.824-5.02l.15.002C10.587 3.429 8.392 2 5.796 2 2.596 2 0 4.16 0 6.826Zm4.632-1.555a.77.77 0 1 1-1.54 0 .77.77 0 0 1 1.54 0Zm3.875 0a.77.77 0 1 1-1.54 0 .77.77 0 0 1 1.54 0Z" />
      </svg>
    </span>
  );

  return (
    <div className="dec-pc-login">
      <div
        className="dec-pc-login__header"
        style={{ height: headerHeight, boxShadow: d.headerShadow ? '0 2px 12px 0 rgba(0,0,0,.04), 0 4px 6px 0 rgba(0,0,0,.06)' : 'none' }}
      >
        <div className="dec-pc-login__header-inner">
          {shopLogo ? (
            <img
              className="dec-pc-login__header-logo"
              src={shopLogo}
              alt="logo"
              style={{ maxWidth: logoWidth, maxHeight: logoHeight, objectFit: 'contain' }}
            />
          ) : (
            <span className="dec-pc-login__header-logo-placeholder" style={{ width: logoWidth, height: logoHeight }}>LOGO</span>
          )}
          {d.showWelcome ? (
            <div className="dec-pc-login__header-info">
              <span>{t('您好，欢迎光临商城！')}</span>
              <span className="dec-pc-login__header-link">{t('没有账号？注册新用户！')}</span>
            </div>
          ) : null}
        </div>
      </div>

      <div
        className="dec-pc-login__warp"
        style={{
          minHeight: warpHeight,
          backgroundColor: d.backgroundColor || '#fff',
          backgroundImage: d.backgroundPic?.picUrl ? `url(${d.backgroundPic.picUrl})` : 'none',
        }}
      >
        {loginMode === 1 ? (
          <div className="dec-login-classic">
            <div className="dec-login-classic__card">
              <div className="dec-login-classic__title-box">
                <div className="dec-login-classic__title">{t('欢迎登录')}</div>
                <div className="dec-login-classic__reg">
                  {t('没有账号？')}
                  <span>{t('去注册')}</span>
                </div>
              </div>
              <div className="dec-login-classic__tabs">
                <div className="dec-login-classic__tab is-active">{t('普通登录')}</div>
                <div className="dec-login-classic__tab">{t('手机登录')}</div>
              </div>
              <div className="dec-login-classic__body">
                <div className="dec-login-classic__input">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" />
                    <circle cx="12" cy="7" r="4" />
                  </svg>
                  <span>{t('用户名/手机')}</span>
                </div>
                <div className="dec-login-classic__input">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                    <rect x="3" y="11" width="18" height="11" rx="2" />
                    <path d="M7 11V7a5 5 0 0 1 10 0v4" />
                  </svg>
                  <span>{t('密码')}</span>
                </div>
                <div className="dec-login-classic__agreement">
                  <span className="dec-login-classic__checkbox" />
                  <span>{t('我已阅读并同意《服务协议》和《隐私协议》')}</span>
                </div>
                <div className="dec-login-classic__btn">{t('登录')}</div>
                <div className="dec-login-classic__forget">{t('忘记密码？')}</div>
                <div className="dec-login-classic__oauth">
                  <div className="dec-login-classic__oauth-tit">{t('合作网站账号登录')}</div>
                  {wechatIcon}
                </div>
              </div>
            </div>
          </div>
        ) : (
          <div className="dec-login-modern">
            <div className="dec-login-modern__content">
              <div className="dec-login-modern__menu">
                <div className="dec-login-modern__menu-item is-active">
                  {t('账号登录')}
                  <span className="dec-login-modern__menu-line" />
                </div>
                <div className="dec-login-modern__menu-item">{t('手机号登录')}</div>
              </div>
              <div className="dec-login-modern__form">
                <div className="dec-login-modern__form-item">
                  <div className="dec-login-modern__label">{t('用户名/手机')}</div>
                  <div className="dec-login-modern__input">{t('用户名/手机')}</div>
                </div>
                <div className="dec-login-modern__form-item">
                  <div className="dec-login-modern__label">
                    <span>{t('密码')}</span>
                    <span className="dec-login-modern__label-special">{t('忘记密码？')}</span>
                  </div>
                  <div className="dec-login-modern__input">{t('密码')}</div>
                </div>
              </div>
              <div className="dec-login-modern__btn-box">
                <div className="dec-login-modern__btn">{t('登录')}</div>
                <div className="dec-login-modern__divider">
                  <span className="dec-login-modern__divider-line" />
                  <span>{t('或通过以下方式加入')}</span>
                  <span className="dec-login-modern__divider-line" />
                </div>
                <div className="dec-login-modern__oauth">
                  <div className="dec-login-modern__oauth-item">
                    {wechatIcon}
                    <p>{t('微信')}</p>
                  </div>
                </div>
              </div>
              <div className="dec-login-modern__register">
                <span>{t('还没有账号？')}</span>
                <span className="dec-login-modern__register-btn">{t('去注册')}</span>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
