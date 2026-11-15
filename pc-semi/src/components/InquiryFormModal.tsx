'use client';

/**
 * 询价留资表单（商品详情页）。
 *
 * 提交接口：POST /api/product/product/priceInquiry（匿名可提交，登录时后端自动关联 user_id）。
 * DTO（固定契约）：mobile（必填）、content（需求，必填）、email、companyName、
 * jobTitle、productId、skuId、skuValue。
 *
 * 防刷门闸：提交前先过 AJ-Captcha 滑块（复用登录页 VerifySlide 组件与校验流程）。
 * 注意：滑块产出的 captchaVerification 只被登录 / 发短信接口校验，询价 DTO
 * 没有该字段，故此处滑块仅作前端交互门闸，token 不随询价提交发送。
 *
 * 已登录用户打开时，用 user/user/memberCenter 的已知手机 / 邮箱预填（取不到不阻塞）。
 */
import { useEffect, useRef, useState } from 'react';
import { Button, Input, Modal, TextArea, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '@/lib/client-api';
import VerifySlide from '@/components/VerifySlide';
import './inquiry-form.scss';

import { useTranslation } from 'react-i18next';

export interface InquiryFormModalProps {
  open: boolean;
  onClose: () => void;
  productId: number;
  productName: string;
  /** 当前型号（可空：未选规格或单规格商品） */
  skuId?: number;
  skuValue?: string;
}

const MOBILE_RE = /^1[3-9]\d{9}$/;
const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export default function InquiryFormModal({ open, onClose, productId, productName, skuId, skuValue }: InquiryFormModalProps) {
  const { t } = useTranslation();
  const [mobile, setMobile] = useState('');
  const [companyName, setCompanyName] = useState('');
  const [jobTitle, setJobTitle] = useState('');
  const [email, setEmail] = useState('');
  const [content, setContent] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [success, setSuccess] = useState(false);
  const [verifyOpen, setVerifyOpen] = useState(false);
  const prefilledRef = useRef(false);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  /* 打开时：已登录则拉 memberCenter 预填手机/邮箱（仅填空位，失败静默） */
  useEffect(() => {
    if (!open) return undefined;
    setSuccess(false);
    if (prefilledRef.current) return undefined;
    if (typeof window === 'undefined' || !localStorage.getItem('accessToken')) return undefined;
    let alive = true;
    apiGet<Record<string, any>>('user/user/memberCenter')
      .then((data) => {
        if (!alive || !data) return;
        const info = (data as any).item ?? data;
        const knownMobile = String(info?.mobile ?? '');
        const knownEmail = String(info?.email ?? '');
        if (knownMobile) setMobile((current) => current || knownMobile);
        if (knownEmail) setEmail((current) => current || knownEmail);
        prefilledRef.current = true;
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [open ]);

  useEffect(
    () => () => {
      if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    },
    [],
  );

  const validate = (): string => {
    if (!MOBILE_RE.test(mobile.trim())) return t('请输入正确的手机号');
    if (email.trim() && !EMAIL_RE.test(email.trim())) return t('请输入正确的邮箱');
    if (!content.trim()) return t('请填写您的需求');
    return '';
  };

  const doSubmit = async () => {
    setSubmitting(true);
    try {
      const payload: Record<string, unknown> = {
        mobile: mobile.trim(),
        content: content.trim(),
        productId,
      };
      if (email.trim()) payload.email = email.trim();
      if (companyName.trim()) payload.companyName = companyName.trim();
      if (jobTitle.trim()) payload.jobTitle = jobTitle.trim();
      if (skuId) payload.skuId = skuId;
      if (skuValue) payload.skuValue = skuValue;
      await apiPost('product/product/priceInquiry', payload);
      setSuccess(true);
      closeTimerRef.current = setTimeout(() => onClose(), 1800);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('提交失败，请稍后重试') });
    } finally {
      setSubmitting(false);
    }
  };

  /** 点「提交询价」：先本地校验，再过滑块门闸，滑块成功才真正提交 */
  const handleSubmitClick = () => {
    const error = validate();
    if (error) {
      Toast.warning({ content: error });
      return;
    }
    setVerifyOpen(true);
  };

  return (
    <>
      <Modal
        title={t('提交询价')}
        visible={open}
        onCancel={onClose}
        footer={null}
        width={480}
        closeOnEsc={!submitting}
        maskClosable={!submitting}
      >
        {success ? (
          <div className="inquiry-form__success">
            <div className="inquiry-form__success-icon">✓</div>
            <p className="inquiry-form__success-text">{t('已提交，我们会尽快与您联系')}</p>
            <Button theme="solid" type="primary" onClick={onClose}>
              {t('完成')}
            </Button>
          </div>
        ) : (
          <div className="inquiry-form">
            <div className="inquiry-form__product">
              <div className="inquiry-form__product-name">{productName}</div>
              {skuValue ? <div className="inquiry-form__product-sku">{t('型号')}：{skuValue}</div> : null}
            </div>

            <label className="inquiry-form__field">
              <span className="inquiry-form__label">
                {t('手机号')}<i className="inquiry-form__required">*</i>
              </span>
              <Input
                value={mobile}
                onChange={(value) => setMobile(value)}
                placeholder={t('请输入手机号，方便我们回拨联系')}
                maxLength={11}
                showClear
              />
            </label>

            <label className="inquiry-form__field">
              <span className="inquiry-form__label">{t('公司名称')}</span>
              <Input value={companyName} onChange={(value) => setCompanyName(value)} placeholder={t('请输入公司名称')} showClear />
            </label>

            <div className="inquiry-form__row">
              <label className="inquiry-form__field">
                <span className="inquiry-form__label">{t('职务')}</span>
                <Input value={jobTitle} onChange={(value) => setJobTitle(value)} placeholder={t('如：采购经理')} showClear />
              </label>
              <label className="inquiry-form__field">
                <span className="inquiry-form__label">{t('邮箱')}</span>
                <Input value={email} onChange={(value) => setEmail(value)} placeholder={t('请输入邮箱')} showClear />
              </label>
            </div>

            <label className="inquiry-form__field">
              <span className="inquiry-form__label">
                {t('需求描述')}<i className="inquiry-form__required">*</i>
              </span>
              <TextArea
                value={content}
                onChange={(value) => setContent(value)}
                placeholder={t('请描述采购数量、交付周期、定制要求等')}
                rows={4}
                maxCount={500}
                showClear
              />
            </label>

            <div className="inquiry-form__actions">
              <Button onClick={onClose} disabled={submitting}>
                {t('取消')}
              </Button>
              <Button theme="solid" type="primary" loading={submitting} onClick={handleSubmitClick}>
                {t('提交询价')}
              </Button>
            </div>
          </div>
        )}
      </Modal>

      {/* AJ-Captcha 滑块门闸：与登录页同一组件/校验流程；成功才放行提交（token 不随询价 DTO 发送，见文件头） */}
      <VerifySlide
        open={verifyOpen}
        onClose={() => setVerifyOpen(false)}
        onSuccess={() => {
          setVerifyOpen(false);
          void doSubmit();
        }}
      />
    </>
  );
}
