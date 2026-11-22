import { useEffect, useMemo, useState } from 'react';
import { Button, Col, Form, Row, SideSheet, Toast } from '@douyinfe/semi-ui';
import type { FormDrawerConfig } from './types';
import FieldRenderer from './FieldRenderer';
import { useI18n } from '../i18n';

interface FormDrawerProps {
  visible: boolean;
  mode: 'create' | 'edit';
  initialValues?: Record<string, any>;
  config: FormDrawerConfig;
  saving?: boolean;
  onCancel: () => void;
  onSubmit: (payload: Record<string, any>) => void | Promise<void>;
}

export default function FormDrawer({
  visible,
  mode,
  initialValues,
  config,
  saving,
  onCancel,
  onSubmit,
}: FormDrawerProps) {
  const { t } = useI18n();
  const [formApi, setFormApi] = useState<any>(null);
  const [submitting, setSubmitting] = useState(false);
  const [activeSection, setActiveSection] = useState(config.sections[0]?.key || '');
  const [values, setValues] = useState<Record<string, any>>(initialValues || {});

  useEffect(() => {
    if (!visible || !formApi) return;
    formApi.reset();
    formApi.setValues(initialValues || {}, { isOverride: true });
    setValues(initialValues || {});
    setActiveSection(config.sections[0]?.key || '');
  }, [visible, initialValues, formApi, config.sections]);

  const scrollTo = (key: string) => {
    setActiveSection(key);
    document
      .getElementById(`framework-section-${key}`)
      ?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  };

  const submit = async () => {
    if (!formApi || submitting) return;
    setSubmitting(true);
    try {
      const formValues = await formApi.validate();
      const payload = config.toPayload
        ? config.toPayload(formValues, mode)
        : formValues;
      await onSubmit(payload);
    } catch (error) {
      if (error instanceof Error) {
        Toast.error({ content: error.message });
      } else {
        // 校验失败（validate 抛出的是字段错误对象而非 Error）：字段已有行内红字，
        // 但出错字段可能在抽屉折叠线以下、用户点了保存却毫无反馈——滚动定位到第一处错误
        requestAnimationFrame(() => {
          document
            .querySelector(
              '.semi-sidesheet .semi-form-field-error-message, .semi-sidesheet [class*="error-message"]',
            )
            ?.scrollIntoView({ behavior: 'smooth', block: 'center' });
        });
      }
    } finally {
      setSubmitting(false);
    }
  };

  const anchorList = useMemo(() => config.sections, [config.sections]);

  return (
    <SideSheet
      title={
        mode === 'create'
          ? t(config.title || config.addText || '新增')
          : `${t('编辑')}${t(config.title || '')}`
      }
      visible={visible}
      onCancel={onCancel}
      width={config.width || 560}
      placement="right"
      bodyStyle={{ padding: 0, overflowY: 'auto' }}      footer={
        <div className="modal-footer">
          <Button onClick={onCancel}>{t('取消')}</Button>
          <Button
            theme="solid"
            type="primary"
            loading={saving || submitting}
            disabled={submitting && !saving}
            onClick={submit}
          >
            {t('保存')}
          </Button>
        </div>
      }
    >
      <div className="product-form-nav">
        {anchorList.map((section) => (
          <button
            type="button"
            key={section.key}
            className={activeSection === section.key ? 'is-active' : ''}
            onClick={() => scrollTo(section.key)}
          >
            {t(section.title)}
          </button>
        ))}
      </div>

      <Form
        getFormApi={setFormApi}
        initValues={initialValues}
        labelPosition={config.labelPosition || 'left'}
        labelWidth={config.labelWidth === 'auto' ? 120 : config.labelWidth || 110}
        labelAlign="right"
        onChange={(state) => setValues({ ...state.values })}
        className="product-form"
      >
        {config.sections.map((section) => {
          const fields = section.fields.filter(
            (field) => !field.visibleWhen || field.visibleWhen(mode),
          );
          return (
            <section
              id={`framework-section-${section.key}`}
              className="product-section"
              key={section.key}
            >
              <div className="product-section-title">{t(section.title)}</div>
              {section.description && (
                <div className="framework-section-desc">{t(section.description)}</div>
              )}
              {fields.map((field) =>
                field.span && field.span < 24 ? (
                  <Row gutter={16} key={field.field}>
                    <Col span={field.span}>
                      <FieldRenderer
                        config={field}
                        formApi={formApi}
                        values={values}
                        mode={mode}
                        inputWidth={config.inputWidth}
                      />
                    </Col>
                  </Row>
                ) : (
                  <FieldRenderer
                    key={field.field}
                    config={field}
                    formApi={formApi}
                    values={values}
                    mode={mode}
                    inputWidth={config.inputWidth}
                  />
                ),
              )}
            </section>
          );
        })}
      </Form>
    </SideSheet>
  );
}
