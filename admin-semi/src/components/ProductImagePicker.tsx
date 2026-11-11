import { useEffect, useState } from 'react';
import { Button, Checkbox, Input, Modal, Spin, Toast } from '@douyinfe/semi-ui';
import { IconSearch } from '@douyinfe/semi-icons';
import { apiGet } from '../api';
import { useI18n } from '../i18n';

interface ProductLite {
  productId: number;
  productName: string;
  productSn?: string;
  picThumb?: string;
  picUrl?: string;
}

const PAGE_SIZE = 8;

/** 批量选中的商品：一张卡片用它带出头图 + 标题 + 商品详情链接 */
export interface PickedImage {
  url: string;
  productId: number;
  productName: string;
  productSn?: string;
}

/** 头图优先取原图，避免带 OSS 缩略参数（?x-oss-process=...）。 */
const mainImage = (detail: Record<string, any>, fallback: ProductLite): string =>
  detail.picOriginal || detail.picUrl || detail.picThumb || fallback.picUrl || fallback.picThumb || '';

export default function ProductImagePickerModal({
  visible,
  onCancel,
  onPick,
  onPickMany,
}: {
  visible: boolean;
  onCancel: () => void;
  /** 点右侧单张图片：替换当前编辑的卡片 */
  onPick: (url: string) => void;
  /** 勾选多个商品：批量生成多张图片卡片（含标题与商品详情链接） */
  onPickMany: (items: PickedImage[]) => void;
}) {
  const { t } = useI18n();
  const [keyword, setKeyword] = useState('');
  const [rows, setRows] = useState<ProductLite[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [active, setActive] = useState<ProductLite | null>(null);
  const [images, setImages] = useState<string[]>([]);
  const [imageLoading, setImageLoading] = useState(false);
  const [checked, setChecked] = useState<ProductLite[]>([]);
  const [submitting, setSubmitting] = useState(false);

  const loadProducts = async (nextPage = 1, search = keyword) => {
    setLoading(true);
    try {
      const result = await apiGet<{ records?: ProductLite[]; total?: number }>('/product/product/list', {
        keyword: search,
        page: nextPage,
        pageSize: PAGE_SIZE,
      });
      setRows(result.records || []);
      setTotal(result.total || 0);
      setPage(nextPage);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!visible) return;
    setKeyword('');
    setActive(null);
    setImages([]);
    setChecked([]);
    void loadProducts(1, '');
  }, [visible]);

  const openProduct = async (product: ProductLite) => {
    setActive(product);
    setImageLoading(true);
    try {
      const detail = await apiGet<Record<string, any>>('/product/product/detail', {
        id: product.productId,
      });
      const gallery: string[] = (detail.imgList || [])
        .map((item: any) => item.picOriginal || item.picUrl || item.picThumb)
        .filter(Boolean);
      setImages(Array.from(new Set([mainImage(detail, product), ...gallery].filter(Boolean) as string[])));
    } catch {
      setImages(Array.from(new Set([product.picUrl, product.picThumb].filter(Boolean) as string[])));
    } finally {
      setImageLoading(false);
    }
  };

  const toggle = (product: ProductLite, on: boolean) => {
    setChecked((prev) => {
      const rest = prev.filter((item) => item.productId !== product.productId);
      return on ? [...rest, product] : rest;
    });
  };

  const selectPage = () => {
    setChecked((prev) => {
      const ids = new Set(prev.map((item) => item.productId));
      return [...prev, ...rows.filter((item) => !ids.has(item.productId))];
    });
  };

  /** 逐个取详情拿原图头图，勾选 N 个商品就生成 N 张卡片 */
  const submitMany = async () => {
    if (!checked.length) return;
    setSubmitting(true);
    try {
      const picked: PickedImage[] = [];
      for (const product of checked) {
        let url = '';
        try {
          const detail = await apiGet<Record<string, any>>('/product/product/detail', {
            id: product.productId,
          });
          url = mainImage(detail, product);
        } catch {
          url = product.picUrl || product.picThumb || '';
        }
        if (url) {
          picked.push({
            url,
            productId: product.productId,
            productName: product.productName,
            productSn: product.productSn,
          });
        }
      }
      if (!picked.length) {
        Toast.warning(t('所选商品均没有可用图片'));
        return;
      }
      onPickMany(picked);
    } finally {
      setSubmitting(false);
    }
  };

  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  return (
    <Modal
      title={t('从商品选择图片')}
      visible={visible}
      onCancel={onCancel}
      width={980}
      footer={
        <div className="prod-img-picker__footer">
          <span>{t('已选 {count} 个商品', { count: checked.length })}</span>
          <div>
            <Button theme="light" disabled={!checked.length} onClick={() => setChecked([])}>
              {t('清空选择')}
            </Button>
            <Button
              theme="solid"
              type="primary"
              loading={submitting}
              disabled={!checked.length}
              onClick={() => void submitMany()}
            >
              {t('添加为 {count} 张图片卡片', { count: checked.length })}
            </Button>
          </div>
        </div>
      }
    >
      <div className="prod-img-picker">
        <div className="prod-img-picker__left">
          <div className="prod-img-picker__search">
            <Input
              value={keyword}
              prefix={<IconSearch />}
              placeholder={t('搜索商品名称 / 货号')}
              onChange={setKeyword}
              onEnterPress={() => void loadProducts(1)}
            />
            <Button onClick={() => void loadProducts(1)}>{t('搜索')}</Button>
          </div>
          <div className="prod-img-picker__rows">
            {loading ? (
              <div className="prod-img-picker__empty"><Spin /></div>
            ) : (
              rows.map((product) => {
                const isChecked = checked.some((item) => item.productId === product.productId);
                return (
                  <div
                    key={product.productId}
                    className={`prod-img-picker__row${active?.productId === product.productId ? ' is-active' : ''}${isChecked ? ' is-checked' : ''}`}
                    role="button"
                    tabIndex={0}
                    onClick={() => void openProduct(product)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') void openProduct(product);
                    }}
                  >
                    <span className="prod-img-picker__check" onClick={(event) => event.stopPropagation()}>
                      <Checkbox checked={isChecked} onChange={(event: any) => toggle(product, !!event.target.checked)} />
                    </span>
                    {product.picThumb || product.picUrl ? (
                      <img src={product.picThumb || product.picUrl} alt="" />
                    ) : (
                      <span className="prod-img-picker__no-pic" />
                    )}
                    <span className="name">{product.productName}</span>
                  </div>
                );
              })
            )}
            {!loading && rows.length === 0 ? (
              <div className="prod-img-picker__empty">{t('没有找到商品')}</div>
            ) : null}
          </div>
          <div className="prod-img-picker__pager">
            <Button size="small" theme="borderless" disabled={!rows.length} onClick={selectPage}>
              {t('全选本页')}
            </Button>
            <span>{page} / {totalPages}</span>
            <div>
              <Button size="small" disabled={page <= 1} onClick={() => void loadProducts(page - 1)}>
                {t('上一页')}
              </Button>
              <Button size="small" disabled={page >= totalPages} onClick={() => void loadProducts(page + 1)}>
                {t('下一页')}
              </Button>
            </div>
          </div>
        </div>
        <div className="prod-img-picker__right">
          <div className="prod-img-picker__tip">
            {t('点单张图片替换当前卡片；勾选左侧商品可一次添加多张卡片')}
          </div>
          {!active ? (
            <div className="prod-img-picker__empty">{t('选择左侧商品查看图片')}</div>
          ) : imageLoading ? (
            <div className="prod-img-picker__empty"><Spin /></div>
          ) : (
            <div className="prod-img-picker__grid">
              {images.map((url, index) => (
                <button
                  type="button"
                  key={`${url}-${index}`}
                  className="prod-img-picker__thumb"
                  title={t('使用这张图片替换当前卡片')}
                  onClick={() => onPick(url)}
                >
                  <img src={url} alt="" />
                  {index === 0 ? <span className="badge">{t('主图')}</span> : null}
                </button>
              ))}
              {images.length === 0 ? (
                <div className="prod-img-picker__empty">{t('该商品暂无图片')}</div>
              ) : null}
            </div>
          )}
        </div>
      </div>
    </Modal>
  );
}
