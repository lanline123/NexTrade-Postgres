import { Button, Empty, Input, Pagination, Spin, Toast } from '@douyinfe/semi-ui';
import { useCallback, useEffect, useState } from 'react';
import { api } from '../api';

interface Category {
  categoryId: number;
  categoryName: string;
}

interface Product {
  productId: number;
  productName: string;
  productPrice?: string | number;
  productSn?: string;
  productStock?: number;
  picThumb?: string;
}

interface ProductPage {
  records: Product[];
  total: number;
  page: number;
  size: number;
}

const PAGE_SIZE = 24;

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : '请求失败';
}

function formatPrice(value: string | number | undefined): string {
  const amount = Number(value ?? 0);
  return Number.isFinite(amount) ? amount.toFixed(2) : '0.00';
}

export default function Cashier() {
  const [categories, setCategories] = useState<Category[]>([]);
  const [products, setProducts] = useState<Product[]>([]);
  const [keyword, setKeyword] = useState('');
  const [categoryId, setCategoryId] = useState<number>();
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);

  const loadProducts = useCallback(
    async (nextPage: number, nextKeyword: string, nextCategory: number | undefined) => {
      setLoading(true);
      try {
        const result = await api.get<ProductPage>('/pos/product/index', {
          keyword: nextKeyword.trim() || undefined,
          categoryId: nextCategory,
          page: nextPage,
          size: PAGE_SIZE,
        });
        setProducts(result.records || []);
        setTotal(result.total || 0);
        setPage(result.page || nextPage);
      } catch (error) {
        Toast.error({ content: toMessage(error) });
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  useEffect(() => {
    api
      .get<{ records: Category[] }>('/pos/product/getCategoryInfo')
      .then((result) => setCategories(result.records || []))
      .catch((error) => Toast.error({ content: toMessage(error) }));
    void loadProducts(1, '', undefined);
  }, [loadProducts]);

  return (
    <div className="cashier">
      <section className="cashier-products">
        <div className="cashier-toolbar">
          <Input
            placeholder="扫码或输入商品名称"
            value={keyword}
            showClear
            onChange={setKeyword}
            onEnterPress={() => void loadProducts(1, keyword, categoryId)}
          />
          <Button theme="solid" type="primary" onClick={() => void loadProducts(1, keyword, categoryId)}>
            搜索
          </Button>
        </div>

        <div className="cashier-categories">
          <button
            type="button"
            className={categoryId === undefined ? 'cashier-chip is-active' : 'cashier-chip'}
            onClick={() => {
              setCategoryId(undefined);
              void loadProducts(1, keyword, undefined);
            }}
          >
            全部
          </button>
          {categories.map((item) => (
            <button
              key={item.categoryId}
              type="button"
              className={categoryId === item.categoryId ? 'cashier-chip is-active' : 'cashier-chip'}
              onClick={() => {
                setCategoryId(item.categoryId);
                void loadProducts(1, keyword, item.categoryId);
              }}
            >
              {item.categoryName}
            </button>
          ))}
        </div>

        <Spin spinning={loading}>
          {products.length === 0 ? (
            <Empty description="暂无商品" />
          ) : (
            <div className="cashier-grid">
              {products.map((item) => (
                <article key={item.productId} className="cashier-card">
                  <div className="cashier-card-media">
                    {item.picThumb ? (
                      <img src={item.picThumb} alt={item.productName} loading="lazy" />
                    ) : (
                      <span>{item.productName.slice(0, 1)}</span>
                    )}
                  </div>
                  <h3>{item.productName}</h3>
                  <p className="cashier-card-sn">{item.productSn || '-'}</p>
                  <div className="cashier-card-foot">
                    <strong>¥{formatPrice(item.productPrice)}</strong>
                    <span>库存 {item.productStock ?? 0}</span>
                  </div>
                </article>
              ))}
            </div>
          )}
        </Spin>

        <div className="cashier-pager">
          <Pagination
            total={total}
            pageSize={PAGE_SIZE}
            currentPage={page}
            showSizeChanger={false}
            onPageChange={(next) => void loadProducts(next, keyword, categoryId)}
          />
        </div>
      </section>

      <aside className="cashier-cart">
        <h2>购物车</h2>
        <p>下一阶段接入 /pos/cart/* 与 /pos/order/check/*，完成加购、改价、挂单与结算。</p>
      </aside>
    </div>
  );
}
