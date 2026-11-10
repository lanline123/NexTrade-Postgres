import type { CSSProperties } from 'react';
import { useCallback, useEffect, useState } from 'react';
import { api } from '../api';
import Price from '../components/Price';
import { toast } from '../toast';

interface Category {
  categoryId: number;
  categoryName: string;
}

interface Product {
  productId: number;
  productName: string;
  productPrice?: string | number;
  productStock?: number;
  picThumb?: string;
}

interface ProductPage {
  records: Product[];
  total: number;
  page: number;
  size: number;
}

interface CartItem {
  cartId: number;
  productName: string;
  picThumb?: string;
  price: string | number;
  originPrice?: string | number;
  quantity: number;
  subtotal: string | number;
  isChecked: boolean;
}

interface CartStore {
  carts: CartItem[];
  isChecked: boolean;
}

interface CartTotal {
  totalCount: number;
  checkedCount: number;
  productAmount: string | number;
  discountAfter: string | number;
}

interface CartVO {
  cartList: CartStore[];
  total: CartTotal;
  pendingId: number;
}

const PAGE_SIZE = 100;

function message(error: unknown): string {
  return error instanceof Error ? error.message : '请求失败';
}

function shopIdOf(): number | undefined {
  const raw = localStorage.getItem('shopId');
  const id = raw ? Number(raw) : NaN;
  return Number.isFinite(id) && id > 0 ? id : undefined;
}

export default function Cashier({ primaryStyle }: { primaryStyle: CSSProperties }) {
  const [categories, setCategories] = useState<Category[]>([]);
  const [products, setProducts] = useState<Product[]>([]);
  const [cart, setCart] = useState<CartVO | null>(null);
  const [keyword, setKeyword] = useState('');
  const [categoryId, setCategoryId] = useState<number>();
  const [loading, setLoading] = useState(false);

  const loadProducts = useCallback(async (nextKeyword: string, nextCategory: number | undefined) => {
    setLoading(true);
    try {
      const result = await api.get<ProductPage>('/pos/product/index', {
        keyword: nextKeyword.trim() || undefined,
        categoryId: nextCategory,
        shopId: shopIdOf(),
        page: 1,
        size: PAGE_SIZE,
      });
      setProducts(result.records || []);
    } catch (error) {
      toast(message(error), 'error');
    } finally {
      setLoading(false);
    }
  }, []);

  const loadCart = useCallback(async () => {
    try {
      const result = await api.post<CartVO>('/pos/cart/list', { shopId: shopIdOf() });
      setCart(result);
    } catch (error) {
      toast(message(error), 'error');
    }
  }, []);

  useEffect(() => {
    api
      .get<{ records: Category[] }>('/pos/product/getCategoryInfo')
      .then((result) => setCategories(result.records || []))
      .catch((error) => toast(message(error), 'error'));
    void loadProducts('', undefined);
    void loadCart();
  }, [loadCart, loadProducts]);

  const items = cart?.cartList.flatMap((store) => store.carts) ?? [];
  const totalCount = cart?.total.totalCount ?? 0;
  const totalAmount = cart?.total.discountAfter ?? 0;

  const addProduct = async (product: Product) => {
    try {
      await api.post('/pos/cart/addToCart', {
        productId: product.productId,
        quantity: 1,
        shopId: shopIdOf(),
      });
      await loadCart();
      toast(`${product.productName} 已加入购物车`);
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  const changeQuantity = async (item: CartItem, quantity: number) => {
    const next = Math.floor(quantity);
    if (!Number.isFinite(next) || next <= 0) return;
    try {
      await api.post('/pos/cart/updateItem', { cartId: item.cartId, quantity: next });
      await loadCart();
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  const removeItem = async (item: CartItem) => {
    if (!window.confirm(`确定要删除该购物车商品吗？\n${item.productName}`)) return;
    try {
      await api.post('/pos/cart/removeItem', { cartIds: [item.cartId] });
      await loadCart();
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  const clearCart = async () => {
    if (!items.length) return;
    if (!window.confirm('清空该购物车后将无法恢复，确定要清空该购物车吗？')) return;
    try {
      await api.post('/pos/cart/clearCart');
      await loadCart();
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  const editRemark = () => {
    const remark = window.prompt('整单备注', '');
    if (remark !== null) toast(remark ? '备注已记录（下单时提交）' : '已清空整单备注');
  };

  const checkout = () => {
    if (!items.length) {
      toast('购物车为空', 'error');
      return;
    }
    toast('结算流程接入中');
  };

  return (
    <div className="cashier-desk-box">
      <div className="pos-cart-box flex">
        <div className="cart-content flex">
          <div className="cart-info flex-column">
            <div className="catr-list">
              <div className="table-context-bar">
                <div className="table-meta">
                  <span className="table-meta-item">
                    <span className="table-meta-label">桌号：</span>
                    <span className="table-meta-value">--</span>
                  </span>
                  <span className="table-meta-item">
                    <span className="table-meta-label">就餐人数：</span>
                    <span className="table-meta-value">--</span>
                  </span>
                </div>
              </div>

              <div className="list-header flex">
                <div className="col-1 flex">序号</div>
                <div className="col-2 flex">商品/编码</div>
                <div className="col-3 flex">售价(￥)</div>
                <div className="col-4 flex">数量</div>
                <div className="col-5 flex">小计</div>
              </div>

              <div className="list">
                {items.length === 0 ? (
                  <div className="cart-empty-box">购物车为空</div>
                ) : (
                  items.map((item, index) => (
                    <div className="list-item flex" key={item.cartId}>
                      <div className="col-1 flex">{String(index + 1).padStart(2, '0')}</div>
                      <div className="col-2 flex">
                        <div className="product-box flex">
                          <div className="product-img">
                            {item.picThumb ? <img src={item.picThumb} alt="" /> : null}
                          </div>
                          <div className="product-info">
                            <div className="product-name line2">{item.productName}</div>
                            <div className="product-sn flex">
                              编码: <span />
                            </div>
                          </div>
                        </div>
                      </div>
                      <div className="col-3 flex">
                        <div className="special-price">
                          <Price value={item.originPrice ?? item.price} size={16} muted strike />
                        </div>
                        <Price value={item.price} size={16} />
                      </div>
                      <div className="col-4 flex">
                        <div className="cart-num-box flex">
                          <div className="delete btn flex algin-center" onClick={() => void removeItem(item)}>
                            <div className="iconfont-pos icon-shanchu" />
                          </div>
                          <div className="num-box">
                            <input
                              key={`${item.cartId}-${item.quantity}`}
                              type="text"
                              defaultValue={item.quantity}
                              onBlur={(event) => void changeQuantity(item, Number(event.target.value))}
                              onKeyDown={(event) => {
                                if (event.key === 'Enter') event.currentTarget.blur();
                              }}
                            />
                          </div>
                          <div
                            className="plus btn flex algin-center"
                            onClick={() => void changeQuantity(item, item.quantity + 1)}
                          >
                            <div className="iconfont-pos icon-zengjia" />
                          </div>
                        </div>
                      </div>
                      <div className="col-5 flex">
                        <Price value={item.subtotal} size={16} />
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>

            <div className="cart-btns flex-column">
              <button type="button" className="el-button el-button--primary btn large" style={primaryStyle}>
                <span>会员(F1)</span>
              </button>
              <button type="button" className="el-button btn" onClick={() => void clearCart()}>
                <span>清空</span>
              </button>
              <div className="dialog-link" style={{ display: 'inline-flex' }}>
                <button type="button" className="el-button is-disabled btn" disabled>
                  <span>充值</span>
                </button>
              </div>
              <div className="dialog-link" style={{ display: 'inline-flex' }}>
                <button type="button" className="el-button is-disabled btn" disabled>
                  <span>优惠券</span>
                </button>
              </div>
              <div className="dialog-link" style={{ display: 'inline-flex' }}>
                <button type="button" className="el-button btn" onClick={() => toast('改价功能接入中')}>
                  <span>改价</span>
                </button>
              </div>
              <div className="badge-box">
                <button type="button" className="el-button btn" onClick={() => toast('挂/取单接入中')}>
                  <span>挂/取单</span>
                </button>
              </div>
            </div>
          </div>
        </div>

        <div className="cart-footer flex">
            <div className="order-info flex">
              <div className="info-left">
                <div className="info-left-img">
                  <span className="pos-avatar">游</span>
                </div>
                <div className="info-left-content">
                  <div className="user-name">
                    <span className="name">游客</span>
                  </div>
                </div>
              </div>
              <div className="info-right flex-cloumn">
                <div className="info-right-top" />
                <div className="total">
                  <div className="sum">
                    共<span>{totalCount}</span>件
                  </div>
                  <Price value={totalAmount} size={28} decimalSize={22} />
                </div>
              </div>
            </div>
            <div className="footer-btns">
              <div className="dialog-link" style={{ display: 'inline-flex' }}>
                <div className="mark-btn" onClick={editRemark}>
                  整单备注
                </div>
              </div>
              <div className="dialog-link" style={{ display: 'inline-flex' }}>
                <button type="button" className="el-button el-button--primary btn large" style={primaryStyle} onClick={checkout}>
                  <span>结账</span>
                </button>
              </div>
            </div>
          </div>
        </div>

      <div className="pos-product-box flex">
        <div className="pos-search-box">
          <div className="pos-search">
            <div className="search-box flex">
              <div className="el-input search-input">
                <div className="el-input__wrapper">
                  <input
                    className="el-input__inner"
                    placeholder="扫码枪扫码"
                    value={keyword}
                    onChange={(event) => setKeyword(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter') void loadProducts(keyword, categoryId);
                    }}
                  />
                </div>
              </div>
              <button
                type="button"
                className="el-button el-button--primary search-btn"
                onClick={() => void loadProducts(keyword, categoryId)}
              >
                <span>搜索</span>
              </button>
            </div>
          </div>
        </div>

        <div className="category-box flex flex-align-center">
          <div className="category-content flex">
            <div className="category-list">
              <div
                className={categoryId === undefined ? 'item active' : 'item'}
                onClick={() => {
                  setCategoryId(undefined);
                  void loadProducts(keyword, undefined);
                }}
              >
                全部
              </div>
              {categories.map((item) => (
                <div
                  key={item.categoryId}
                  className={categoryId === item.categoryId ? 'item active' : 'item'}
                  onClick={() => {
                    setCategoryId(item.categoryId);
                    void loadProducts(keyword, item.categoryId);
                  }}
                >
                  {item.categoryName}
                </div>
              ))}
            </div>
          </div>
          <div className="switch-view">
            <div className="icon">
              <i className="iconfont-pos icon-tuwendaohang" />
            </div>
          </div>
        </div>

        <div className="product-content">
          {loading ? <div className="product-loading">加载中…</div> : null}
          <div className="product-list">
            {products.map((product) => (
              <div className="product-item-wrapper" key={product.productId}>
                <div className="badge-box">
                  <div className="product-item" onClick={() => void addProduct(product)}>
                    <div className="img-box">{product.picThumb ? <img src={product.picThumb} alt="" /> : null}</div>
                    <div className="product-info">
                      <div className="name pos-ellipsis-1">{product.productName}</div>
                      <div className="price">¥{Number(product.productPrice ?? 0).toFixed(2)}</div>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
          <div className="end-box">{products.length === 0 ? '暂无商品' : '没有更多了'}</div>
        </div>
      </div>

      <div className="dialog-link" />
    </div>
  );
}
