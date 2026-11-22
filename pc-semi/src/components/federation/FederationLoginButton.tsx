'use client';

/**
 * 联邦登录入口按钮（登录页社交登录行）。纯文字按钮（联邦无品牌图标
 * 资产），样式与同排第三方图标按钮同口径（auth-modern.scss
 * .login-card__federation）；中枢名进 title 悬停提示。
 *
 * 自 useFederationSso.ts 拆出（2026-11-22 构建修复）：JSX 组件写在
 * .ts 文件里会被 SWC 按纯 TS 解析而构建失败（build-pc 实测），故组件
 * 独立成 .tsx，hook 与逻辑仍留在 useFederationSso.ts。组件本体逐字未动。
 */
import { useTranslation } from 'react-i18next';

export function FederationLoginButton({
  peerName,
  starting,
  onStart,
}: {
  peerName: string;
  starting: boolean;
  onStart: () => void;
}) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      className="login-card__federation"
      disabled={starting}
      onClick={onStart}
      title={peerName ? t('使用{peerName}账号登录', { peerName }) : t('联邦登录')}
    >
      {starting ? t('正在跳转...') : t('联邦登录')}
    </button>
  );
}
