import { useEffect, useState } from 'react';
import { Button, Radio, RadioGroup, Switch, Toast } from '@douyinfe/semi-ui';
import { IconSetting, IconUser } from '@douyinfe/semi-icons';
import SettingsCard from '../components/SettingsCard';
import { apiGet, apiPost, isDemo } from '../api';
import { useI18n } from '../i18n';
import { isPlatformAccount } from '../components/BlacklistAddModal';


/** 会员设置（客户能力门）：对应旧版 user/shopMember/Index.vue，接口 /user/shopMember/setting/*。
 * 旧版文案为 i18n 模板：允许shopTerm添加客户 / 允许shopTerm导入客户（模板变量 shopTerm 渲染为 店铺/门店），新版固定"店铺"。 */
export default function ShopMemberSettings() {
  const { t } = useI18n();
  const [formState, setFormState] = useState<Record<string, number>>({
    shopMemberEnabled: 0,
    shopMemberDesensitize: 1,
    shopMemberAllowAdd: 1,
    shopMemberAllowImport: 1,
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  /** 客户黑名单开关（shopBlacklistAllow）：独立端点 /user/blacklist/setting/*，仅平台可见。 */
  const platform = isPlatformAccount();
  const [blacklistAllow, setBlacklistAllow] = useState(0);
  const [blacklistSaving, setBlacklistSaving] = useState(false);

  useEffect(() => {
    if (!platform) return;
    apiGet<{ shopBlacklistAllow: number }>('/user/blacklist/setting/get')
      .then((result) => setBlacklistAllow(Number(result?.shopBlacklistAllow) === 1 ? 1 : 0))
      .catch(() => undefined);
  }, [platform]);

  const saveBlacklistAllow = async (next: boolean) => {
    if (isDemo()) {
      Toast.warning({ content: t('演示模式不支持操作') });
      return;
    }
    setBlacklistSaving(true);
    try {
      await apiPost('/user/blacklist/setting/save', { shopBlacklistAllow: next ? 1 : 0 });
      setBlacklistAllow(next ? 1 : 0);
      Toast.success({ content: t('保存成功') });
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setBlacklistSaving(false);
    }
  };

  useEffect(() => {
    setLoading(true);
    apiGet<Record<string, number>>('/user/shopMember/setting/get')
      .then((result) => setFormState((current) => ({ ...current, ...(result || {}) })))
      .catch((error) => Toast.error({ content: error.message }))
      .finally(() => setLoading(false));
  }, []);

  const save = async () => {
    setSaving(true);
    try {
      await apiPost('/user/shopMember/setting/save', formState);
      Toast.success({ content: t('保存成功') });
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  const settingsDisabled = Number(formState.shopMemberEnabled) !== 1;

  return (
    <div className="config-page">
      <SettingsCard
        icon={<IconUser />}
        title={t('店铺客户开关')}
        description={t('控制店铺是否展示本店客户。')}
        count={t('{count} 项', { count: 1 })}
      >
        <div className="config-field">
          <label>{t('店铺是否展示客户')}</label>
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <Switch
                checked={Number(formState.shopMemberEnabled) === 1}
                onChange={(next) => setFormState((c) => ({ ...c, shopMemberEnabled: next ? 1 : 0 }))}
              />
              <span>{Number(formState.shopMemberEnabled) === 1 ? t('已开启') : t('已关闭')}</span>
            </div>
            <p className="config-field__help">
              {t('开启后，顾客在店铺下单将自动成为该店铺客户，店铺可查看和管理本店客户；关闭后全平台冻结。')}
            </p>
          </div>
        </div>
      </SettingsCard>

      <SettingsCard
        icon={<IconSetting />}
        title={t('店铺客户设置')}
        description={t('客户脱敏、添加与导入权限。')}
        count={t('{count} 项', { count: 3 })}
      >
        <div className="config-field">
          <label>{t('敏感信息脱敏')}</label>
          <div>
            <RadioGroup
              type="button"
              value={formState.shopMemberDesensitize}
              disabled={settingsDisabled}
              onChange={(event) => setFormState((c) => ({ ...c, shopMemberDesensitize: Number(event.target.value) }))}
            >
              <Radio value={1}>{t('开启')}</Radio>
              <Radio value={0}>{t('关闭')}</Radio>
            </RadioGroup>
            <p className="config-field__help">
              {t('开启后，店铺端查看客户时对昵称/手机号脱敏展示。总开关关闭时默认关闭。')}
            </p>
          </div>
        </div>
        <div className="config-field">
          <label>{t('允许店铺添加客户')}</label>
          <div>
            <RadioGroup
              type="button"
              value={formState.shopMemberAllowAdd}
              disabled={settingsDisabled}
              onChange={(event) => setFormState((c) => ({ ...c, shopMemberAllowAdd: Number(event.target.value) }))}
            >
              <Radio value={1}>{t('开启')}</Radio>
              <Radio value={0}>{t('关闭')}</Radio>
            </RadioGroup>
            <p className="config-field__help">{t('开启后，店铺端可手动添加客户为本店客户。')}</p>
          </div>
        </div>
        <div className="config-field">
          <label>{t('允许店铺导入客户')}</label>
          <div>
            <RadioGroup
              type="button"
              value={formState.shopMemberAllowImport}
              disabled={settingsDisabled}
              onChange={(event) => setFormState((c) => ({ ...c, shopMemberAllowImport: Number(event.target.value) }))}
            >
              <Radio value={1}>{t('开启')}</Radio>
              <Radio value={0}>{t('关闭')}</Radio>
            </RadioGroup>
            <p className="config-field__help">{t('开启后，店铺端可批量导入客户为本店客户。')}</p>
          </div>
        </div>
      </SettingsCard>

      {platform && (
        <SettingsCard
          icon={<IconUser />}
          title={t('客户黑名单')}
          description={t('控制店铺是否可将客户加入店铺黑名单。')}
          count={t('{count} 项', { count: 1 })}
        >
          <div className="config-field">
            <label>{t('允许店铺将客户加入店铺黑名单')}</label>
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <Switch
                  checked={blacklistAllow === 1}
                  loading={blacklistSaving}
                  onChange={saveBlacklistAllow}
                />
                <span>{blacklistAllow === 1 ? t('已开启') : t('已关闭')}</span>
              </div>
              <p className="config-field__help">
                {t('关闭后店铺端黑名单入口不可用，平台黑名单不受影响，已生效的店铺条目暂停生效。')}
              </p>
            </div>
          </div>
        </SettingsCard>
      )}

      <div className="config-actions">
        <Button theme="solid" type="primary" loading={saving} onClick={save}>
          {t('保存')}
        </Button>
      </div>
    </div>
  );
}
