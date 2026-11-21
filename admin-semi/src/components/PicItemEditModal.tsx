/**
 * 图片编辑弹窗（PicItemEditModal）——装修编辑器通用的单张图片条目编辑弹窗，
 * 供任意需要「编辑一个 PicItem」的场景独立调用（不依赖图片列表编辑器）。
 *
 * 2026-11-21 自共享组件「图片列表编辑器」PicListEditor 内嵌的「编辑图片」
 * 弹窗抽取升格为独立组件（用户在分类推荐 A1 模块「基本设置」提出：点
 * 「添加图片」弹出的弹窗要做成独立组件供其他调用、再做现代化设计、高度
 * 要居中）。PicListEditor 本体改为调用本组件，既有全部调用点（PC 编辑器
 * + H5 22 个图片列表 + imageSquareAd）行为与数据契约逐字不变。
 *
 * 形态（与 PicListEditor 抽取前逐项一致）：选图框在左（素材中心选图走
 * ImageUploader，内含 GalleryPickerModal；有图时工具栏可清除图片）、
 * 标题 / 描述 / 链接（LinkField）纵列在右，底部左「删除」右「完成」。
 * 字段与图片均为即时回写（onPatch 展开合并），关闭不回滚——与抽取前
 * updateItem 口径一致。showImage=false 为「无图条目」模式：弹窗只剩
 * 标题 / 描述 / 链接字段，无选图区与商品选图入口，标题改「编辑条目」。
 *
 * 「从商品选择图片」入口：本组件只负责发出请求（onRequestProductPick），
 * 商品选图弹窗 ProductImagePickerModal（登记册第 11 条，本身即独立共享
 * 件）由宿主托管——批量带入的落位口径各宿主不同（PicListEditor 的当前卡
 * 填充 / 顺延插入、H5 的 onPickManyProducts 委托），留在宿主可保证抽取
 * 前后落位逻辑逐字不变；不传 onRequestProductPick 时不渲染该入口。
 *
 * 现代化与居中（2026-11-21）：Semi Modal `centered` 垂直居中（抽取前无
 * centered、弹窗贴顶呈现，是本次用户点名问题）；弹窗壳沿用图片列表编辑
 * 弹窗的现代化口径（.dec-pic-modal__dialog 作用域：直角扁平描边 + 柔和
 * 投影、头/脚描边分隔、浅底描边操作条、描边预览面板、字段标签加重），
 * 本批续补：内容超高时弹窗定高、body 内部滚动不溢出屏幕；工具栏/底部
 * 按钮与字段输入框在弹窗作用域内改直角，与后台扁平直角语言一致。
 * 样式：pages/decorate-editor.css 的 .dec-pic-modal*（PC 与 H5 编辑器入口
 * 均已引入该样式表，独立调用方需保证该样式表在场）。
 * 通用组件登记：~/workspace/shared-components.md 第 16 条。
 */
import { Button, Input, Modal } from '@douyinfe/semi-ui';
import { IconClose, IconPlus } from '@douyinfe/semi-icons';
import { useI18n } from '../i18n';
import ImageUploader from './ImageUploader';
import { LinkField } from './LinkPicker';
import type { PicItem } from './PicListEditor';

export interface PicItemEditModalProps {
  visible: boolean;
  /** 当前编辑条目；visible 为 true 且 item 为空时弹窗只渲染空壳（宿主状态未就绪的兜底） */
  item?: PicItem | null;
  /** false = 无图条目模式（纯文字 + 链接，无选图区）；缺省 true */
  showImage?: boolean;
  /** 字段显隐（与 PicListEditor 同口径）：缺省 showText/showLink=true、showDesc=false */
  showText?: boolean;
  showDesc?: boolean;
  showLink?: boolean;
  /** 字段 / 图片即时回写：宿主对当前条目做展开合并 patch（与 PicItem 契约一致） */
  onPatch: (patch: Partial<PicItem>) => void;
  /** 删除当前条目；不传则不渲染「删除」按钮 */
  onRemove?: () => void;
  /** 关闭弹窗（完成 / 关闭钮 / 遮罩同口径：仅关闭，不回滚已回写的 patch） */
  onClose: () => void;
  /**
   * 工具栏「从商品选择图片」点击回调（宿主据此打开 ProductImagePickerModal
   * 并自行处理单选回填 / 批量落位）；不传则不渲染该入口。
   */
  onRequestProductPick?: () => void;
}

export function PicItemEditModal({
  visible,
  item,
  showImage = true,
  showText = true,
  showDesc = false,
  showLink = true,
  onPatch,
  onRemove,
  onClose,
  onRequestProductPick,
}: PicItemEditModalProps) {
  const { t } = useI18n();
  const currentUrl = item ? String(item.picUrl || item.picThumb || '') : '';

  const renderFields = () => (
    <div className="dec-pic-modal__fields">
      {showText && (
        <label className="dec-pic-modal__field">
          <span>{t('标题')}</span>
          <Input
            value={item?.picTitle || ''}
            placeholder={t('建议10个字以内，可不填')}
            onChange={(v) => onPatch({ picTitle: v })}
          />
        </label>
      )}
      {showDesc && (
        <label className="dec-pic-modal__field">
          <span>{t('描述')}</span>
          <Input
            value={item?.picDesc || ''}
            placeholder={t('可不填')}
            onChange={(v) => onPatch({ picDesc: v })}
          />
        </label>
      )}
      {showLink && (
        <div className="dec-pic-modal__field">
          <span>{t('链接')}</span>
          <LinkField
            value={item?.picLink}
            onChange={(link) => onPatch({ picLink: link })}
          />
        </div>
      )}
    </div>
  );

  return (
    <Modal
      className="dec-pic-modal__dialog"
      title={showImage ? t('编辑图片') : t('编辑条目')}
      visible={visible}
      onCancel={onClose}
      width={560}
      centered
      footer={
        <div className="dec-pic-modal__footer">
          {onRemove ? (
            <Button
              type="danger"
              theme="borderless"
              icon={<IconClose />}
              onClick={onRemove}
            >
              {t('删除')}
            </Button>
          ) : (
            <span />
          )}
          <Button theme="solid" type="primary" onClick={onClose}>
            {t('完成')}
          </Button>
        </div>
      }
    >
      {item ? (
        showImage ? (
          <div className="dec-pic-modal">
            <div className="dec-pic-modal__toolbar">
              {onRequestProductPick ? (
                <>
                  <Button size="small" theme="light" type="primary" icon={<IconPlus />} onClick={onRequestProductPick}>
                    {t('从商品选择图片')}
                  </Button>
                  <span className="dec-pic-modal__toolbar-hint">{t('可用商品头图或商品相册图片')}</span>
                </>
              ) : null}
              {currentUrl ? (
                <Button
                  size="small"
                  theme="borderless"
                  type="danger"
                  onClick={() => onPatch({ picUrl: '', picThumb: '' })}
                >
                  {t('清除图片')}
                </Button>
              ) : null}
            </div>
            <div className="dec-pic-modal__body">
              <div className="dec-pic-modal__preview">
                <ImageUploader
                  single
                  value={currentUrl ? [currentUrl] : []}
                  onChange={(urls) => onPatch({ picUrl: urls[0] || '', picThumb: urls[0] || '' })}
                />
                <div className="dec-pic-modal__preview-hint">{t('点击图片从素材中心更换')}</div>
              </div>
              {renderFields()}
            </div>
          </div>
        ) : (
          // 无图模式：弹窗只有字段列，无选图区与商品选图入口
          <div className="dec-pic-modal">{renderFields()}</div>
        )
      ) : null}
    </Modal>
  );
}

export default PicItemEditModal;
