'use client';

import { useCallback, useEffect, useState } from 'react';
import { Button, Card, Modal, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '@/lib/client-api';
import { useTranslation } from 'react-i18next';

export interface InvoiceTitle {
  id: number;
  titleType: number;
  titleName: string;
  taxNo: string;
  companyAddress: string;
  companyPhone: string;
  bankName: string;
  bankAccount: string;
  isDefault: number;
}

interface TitleForm {
  id?: number;
  titleType: number;
  titleName: string;
  taxNo: string;
  companyAddress: string;
  companyPhone: string;
  bankName: string;
  bankAccount: string;
  isDefault: boolean;
}

const EMPTY_FORM: TitleForm = {
  titleType: 1, titleName: '', taxNo: '', companyAddress: '', companyPhone: '', bankName: '', bankAccount: '', isDefault: false,
};

/** 常用发票抬头管理（v5.8.65 跟进）：列表 + 新建/编辑 + 删除 + 设默认。 */
export default function InvoiceTitleClient() {
  const { t } = useTranslation();
  const [titles, setTitles] = useState<InvoiceTitle[]>([]);
  const [loading, setLoading] = useState(false);
  const [form, setForm] = useState<TitleForm | null>(null);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const result = await apiGet<any>('user/invoiceTitle/list');
      const list = Array.isArray(result) ? result : (result?.item ?? result?.data ?? []);
      setTitles(Array.isArray(list) ? list : []);
    } catch {
      setTitles([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  const act = async (fn: () => Promise<unknown>, success: string) => {
    try {
      await fn();
      Toast.success({ content: success });
      await load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    }
  };

  const save = async () => {
    if (!form) return;
    if (!form.titleName.trim()) return Toast.error({ content: t('请输入抬头名称') });
    if (form.titleType === 2 && !form.taxNo.trim()) return Toast.error({ content: t('企业抬头必须填写纳税人识别号') });
    // 与后端 validate_tax_no 同规：18 位统一社会信用代码或 15 位老税号，字母数字。
    if (form.taxNo.trim() && !/^([0-9A-Za-z]{18}|[0-9A-Za-z]{15})$/.test(form.taxNo.trim())) return Toast.error({ content: t('纳税人识别号格式不正确') });
    setSaving(true);
    try {
      const payload = {
        titleType: form.titleType,
        titleName: form.titleName.trim(),
        taxNo: form.taxNo.trim(),
        companyAddress: form.companyAddress.trim(),
        companyPhone: form.companyPhone.trim(),
        bankName: form.bankName.trim(),
        bankAccount: form.bankAccount.trim(),
        isDefault: form.isDefault ? 1 : 0,
      };
      if (form.id) {
        await apiPost('user/invoiceTitle/update', { id: form.id, ...payload });
      } else {
        await apiPost('user/invoiceTitle/create', payload);
      }
      Toast.success({ content: t('保存成功') });
      setForm(null);
      await load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  const updateForm = (key: keyof TitleForm, value: string | number | boolean) =>
    setForm((current) => (current ? { ...current, [key]: value } : current));

  return (
    <Card className="member-card invoice-card">
      <div className="invoice-form">
        <div className="invoice-actions" style={{ justifyContent: 'space-between' }}>
          <span>{t('常用发票抬头最多保存10条，申请发票时自动填充默认抬头')}</span>
          <Button theme="solid" type="primary" disabled={titles.length >= 10} onClick={() => setForm({ ...EMPTY_FORM })}>{t('新建抬头')}</Button>
        </div>
        {loading ? <p className="invoice-hint">{t('加载中…')}</p> : null}
        {!loading && titles.length === 0 ? <p className="invoice-hint">{t('暂无常用抬头')}</p> : null}
        {titles.map((title) => (
          <div key={title.id} className="invoice-options" style={{ alignItems: 'center', gap: 8 }}>
            <strong>{title.titleName}</strong>
            <span>{title.titleType === 2 ? t('企业') : t('个人')}</span>
            {title.taxNo ? <span>{title.taxNo}</span> : null}
            {title.isDefault ? <span className="main-color">{t('默认')}</span> : null}
            <span style={{ flex: 1 }} />
            {!title.isDefault ? (
              <Button size="small" onClick={() => void act(() => apiPost('user/invoiceTitle/setDefault', { id: title.id }), t('已设为默认'))}>{t('设为默认')}</Button>
            ) : null}
            <Button size="small" onClick={() => setForm({
              id: title.id, titleType: title.titleType, titleName: title.titleName, taxNo: title.taxNo,
              companyAddress: title.companyAddress, companyPhone: title.companyPhone,
              bankName: title.bankName, bankAccount: title.bankAccount, isDefault: Boolean(title.isDefault),
            })}>{t('编辑')}</Button>
            <Button size="small" type="danger" onClick={() => {
              Modal.confirm({
                title: t('删除该抬头？'),
                content: title.isDefault ? t('删除默认抬头后，系统会自动将最近更新的抬头设为默认') : undefined,
                onOk: () => act(() => apiPost('user/invoiceTitle/del', { id: title.id }), t('已删除')),
              });
            }}>{t('删除')}</Button>
          </div>
        ))}
      </div>
      <Modal
        title={form?.id ? t('编辑抬头') : t('新建抬头')}
        visible={Boolean(form)}
        onCancel={() => setForm(null)}
        onOk={() => void save()}
        confirmLoading={saving}
        okText={t('保存')}
        cancelText={t('取消')}
      >
        {form ? (
          <div className="invoice-form">
            <label>{t('抬头类型')}</label>
            <div className="invoice-options">
              <button type="button" className={form.titleType === 1 ? 'is-active' : ''} onClick={() => updateForm('titleType', 1)}>{t('个人')}</button>
              <button type="button" className={form.titleType === 2 ? 'is-active' : ''} onClick={() => updateForm('titleType', 2)}>{t('企业')}</button>
            </div>
            <label>{form.titleType === 2 ? t('企业名称') : t('个人名称')}</label>
            <input value={form.titleName} onChange={(e) => updateForm('titleName', e.target.value)} placeholder={t('请输入抬头名称')} />
            {form.titleType === 2 ? (
              <>
                <label>{t('纳税人识别号')}</label>
                <input value={form.taxNo} onChange={(e) => updateForm('taxNo', e.target.value)} placeholder={t('请输入纳税人识别号')} />
                <label>{t('单位地址')}</label>
                <input value={form.companyAddress} onChange={(e) => updateForm('companyAddress', e.target.value)} placeholder={t('请输入单位地址')} />
                <label>{t('单位电话')}</label>
                <input value={form.companyPhone} onChange={(e) => updateForm('companyPhone', e.target.value)} placeholder={t('请输入单位电话')} />
                <label>{t('开户银行')}</label>
                <input value={form.bankName} onChange={(e) => updateForm('bankName', e.target.value)} placeholder={t('请输入开户银行')} />
                <label>{t('银行账户')}</label>
                <input value={form.bankAccount} onChange={(e) => updateForm('bankAccount', e.target.value)} placeholder={t('请输入银行账户')} />
              </>
            ) : null}
            <label>
              <input type="checkbox" checked={form.isDefault} onChange={(e) => updateForm('isDefault', e.target.checked)} /> {t('设为默认抬头')}
            </label>
          </div>
        ) : null}
      </Modal>
    </Card>
  );
}
