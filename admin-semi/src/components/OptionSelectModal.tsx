import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { Button, Input, Modal } from '@douyinfe/semi-ui';
import { IconClose, IconInbox, IconLayers, IconSearch, IconTick } from '@douyinfe/semi-icons';
import { useI18n } from '../i18n';
import './option-select-modal.css';

/**
 * 通用「卡片式单选」弹窗，任何需要从列表中选其一的场景都可直接复用。
 *
 * 与旧版 RadioGroup 弹窗相比：单选卡、搜索过滤、状态标签、
 * 骨架屏/空态、选中摘要、双击直达确认，均为受控用法。
 *
 * @example
 * <OptionSelectModal
 *   visible={visible}
 *   title={t('更换首页')}
 *   options={rows.map((r) => ({ value: r.id, label: r.name, description: r.time }))}
 *   value={choice}
 *   onChange={setChoice}
 *   onConfirm={(id) => save(id)}
 *   onCancel={() => setVisible(false)}
 * />
 */

export type OptionSelectValue = string | number;

export interface OptionSelectItem {
  value: OptionSelectValue;
  label: ReactNode;
  /** 搜索匹配用的纯文本；label 为字符串时可省略 */
  searchText?: string;
  /** 次要说明，如更新时间、路径 */
  description?: ReactNode;
  /** 左侧标识块，缺省用 label 首字 */
  icon?: ReactNode;
  /** 右侧状态标签 */
  tag?: ReactNode;
  disabled?: boolean;
}

export interface OptionSelectModalProps {
  visible: boolean;
  title: ReactNode;
  subtitle?: ReactNode;
  /** 头部下方提示条，如「当前首页：xxx」 */
  notice?: ReactNode;
  options: OptionSelectItem[];
  value: OptionSelectValue | null;
  onChange: (value: OptionSelectValue) => void;
  loading?: boolean;
  /** 默认在选项超过 6 个时显示搜索框 */
  searchable?: boolean;
  searchPlaceholder?: string;
  emptyText?: ReactNode;
  /** 默认单列 */
  columns?: 1 | 2;
  confirmText?: string;
  cancelText?: string;
  width?: number;
  onConfirm: (value: OptionSelectValue) => void | Promise<void>;
  onCancel: () => void;
}

const searchableText = (item: OptionSelectItem) =>
  `${item.searchText ?? ''} ${
    typeof item.label === 'string' ? item.label : ''
  } ${typeof item.description === 'string' ? item.description : ''}`.toLowerCase();

export default function OptionSelectModal({
  visible,
  title,
  subtitle,
  notice,
  options,
  value,
  onChange,
  loading = false,
  searchable,
  searchPlaceholder,
  emptyText,
  columns = 1,
  confirmText,
  cancelText,
  width = 620,
  onConfirm,
  onCancel,
}: OptionSelectModalProps) {
  const { t } = useI18n();
  const [keyword, setKeyword] = useState('');
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (visible) {
      setKeyword('');
      setSubmitting(false);
    }
  }, [visible]);

  const showSearch = searchable ?? options.length > 6;
  const searchText = keyword.trim();
  const filtered = useMemo(() => {
    const text = searchText.toLowerCase();
    if (!text) return options;
    return options.filter((item) => searchableText(item).includes(text));
  }, [options, searchText]);

  const selectedItem = useMemo(
    () => options.find((item) => item.value === value) ?? null,
    [options, value],
  );

  const submit = async (override?: OptionSelectValue) => {
    const target = override ?? value;
    if (target === null || target === undefined || submitting) return;
    setSubmitting(true);
    try {
      await onConfirm(target);
    } catch {
      // 失败提示由调用方负责
    } finally {
      setSubmitting(false);
    }
  };

  const choose = (next: OptionSelectValue, confirmNow = false) => {
    onChange(next);
    if (confirmNow) void submit(next);
  };

  return (
    <Modal
      className="option-select-modal"
      visible={visible}
      width={width}
      header={null}
      footer={null}
      bodyStyle={{ padding: 0 }}
      maskClosable={false}
      closeOnEsc
      onCancel={onCancel}
    >
      <div className="osm">
        <header className="osm-head">
          <span className="osm-head-icon" aria-hidden>
            <IconLayers />
          </span>
          <div className="osm-head-main">
            <div className="osm-head-title">{title}</div>
            {subtitle && <div className="osm-head-sub">{subtitle}</div>}
          </div>
          <button type="button" className="osm-head-close" aria-label={t('关闭')} onClick={onCancel}>
            <IconClose />
          </button>
        </header>

        {notice && <div className="osm-notice">{notice}</div>}

        {(showSearch || filtered.length > 0) && !loading && (
          <div className="osm-toolbar">
            {showSearch && (
              <Input
                prefix={<IconSearch />}
                value={keyword}
                onChange={setKeyword}
                showClear
                placeholder={searchPlaceholder ?? t('搜索')}
                style={{ width: 280 }}
              />
            )}
            <span className="osm-count">
              {t('共 {count} 项', { count: filtered.length })}
            </span>
          </div>
        )}

        <div className={`osm-list${columns === 2 ? ' is-cols-2' : ''}`}>
          {loading ? (
            [0, 1, 2].map((row) => (
              <div className="osm-skeleton" key={row}>
                <span className="osm-sk-radio" />
                <span className="osm-sk-lines">
                  <i />
                  <i />
                </span>
              </div>
            ))
          ) : filtered.length === 0 ? (
            <div className="osm-empty">
              <span className="osm-empty-icon">
                <IconInbox size="extra-large" />
              </span>
              <span className="osm-empty-text">
                {searchText ? t('没有匹配的选项') : emptyText ?? t('暂无数据')}
              </span>
            </div>
          ) : (
            filtered.map((item) => {
              const selected = item.value === value;
              const initial =
                item.icon ??
                (typeof item.label === 'string' && item.label.trim() ? (
                  <span className="osm-option-initial">{item.label.trim().slice(0, 1).toUpperCase()}</span>
                ) : null);
              return (
                <button
                  key={String(item.value)}
                  type="button"
                  role="radio"
                  aria-checked={selected}
                  disabled={item.disabled}
                  className={`osm-option${selected ? ' is-selected' : ''}`}
                  onClick={() => choose(item.value)}
                  onDoubleClick={() => !item.disabled && choose(item.value, true)}
                >
                  <span className="osm-option-radio" aria-hidden>
                    {selected && <IconTick size="extra-small" />}
                  </span>
                  {initial}
                  <span className="osm-option-main">
                    <span className="osm-option-title">{item.label}</span>
                    {item.description && <span className="osm-option-desc">{item.description}</span>}
                  </span>
                  {item.tag && <span className="osm-option-tag">{item.tag}</span>}
                </button>
              );
            })
          )}
        </div>

        <footer className="osm-foot">
          <span className="osm-foot-info">
            {selectedItem ? (
              <>
                {t('已选择：')}
                <b>{selectedItem.label}</b>
              </>
            ) : (
              t('请选择一个选项')
            )}
          </span>
          <span className="osm-foot-actions">
            <Button onClick={onCancel}>{cancelText ?? t('取消')}</Button>
            <Button
              theme="solid"
              type="primary"
              loading={submitting}
              disabled={value === null || value === undefined}
              onClick={() => void submit()}
            >
              {confirmText ?? t('确定')}
            </Button>
          </span>
        </footer>
      </div>
    </Modal>
  );
}
