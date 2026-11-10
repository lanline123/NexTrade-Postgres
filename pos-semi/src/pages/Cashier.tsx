import type { CSSProperties } from 'react';
import { useCallback, useEffect, useState } from 'react';
import { api } from '../api';
import Dialog from '../components/Dialog';
import Price from '../components/Price';
import productEmpty from '../legacy/productEmpty-P1dhOWH0.png';
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
  productId: number;
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

interface PendingOrder {
  pendingId: number;
  remark: string;
  userId: number;
  tableId: number;
  addTime: string;
  count: number;
  amount: string | number;
}

interface ConfirmState {
  message: string;
  action: () => void | Promise<void>;
}

interface Member {
  userId: number;
  nickname: string;
  mobile: string;
  balance: string | number;
  points: string | number;
}

const PAGE_SIZE = 30;

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
  const [loadingMore, setLoadingMore] = useState(false);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [displayMode, setDisplayMode] = useState<'imageText' | 'text'>('imageText');
  const [remark, setRemark] = useState('');
  const [remarkOpen, setRemarkOpen] = useState(false);
  const [remarkDraft, setRemarkDraft] = useState('');
  const [pendingOpen, setPendingOpen] = useState(false);
  const [pendingList, setPendingList] = useState<PendingOrder[]>([]);
  const [pendingLoading, setPendingLoading] = useState(false);
  const [confirmState, setConfirmState] = useState<ConfirmState | null>(null);
  const [member, setMember] = useState<Member | null>(null);
  const [memberOpen, setMemberOpen] = useState(false);
  const [memberMobile, setMemberMobile] = useState('');
  const [memberResult, setMemberResult] = useState<Member | null>(null);
  const [memberSearching, setMemberSearching] = useState(false);
  const [priceOpen, setPriceOpen] = useState(false);
  const [priceType, setPriceType] = useState<1 | 2>(1);
  const [priceValue, setPriceValue] = useState('');
  const [pricePreview, setPricePreview] = useState<string | number | null>(null);
  const [rechargeOpen, setRechargeOpen] = useState(false);
  const [rechargeValue, setRechargeValue] = useState('');

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
      setTotal(result.total || 0);
      setPage(1);
    } catch (error) {
      toast(message(error), 'error');
    } finally {
      setLoading(false);
    }
  }, []);

  const loadMore = async () => {
    if (loading || loadingMore || products.length >= total) return;
    setLoadingMore(true);
    try {
      const result = await api.get<ProductPage>('/pos/product/index', {
        keyword: keyword.trim() || undefined,
        categoryId,
        shopId: shopIdOf(),
        page: page + 1,
        size: PAGE_SIZE,
      });
      setProducts((prev) => [...prev, ...(result.records || [])]);
      setTotal(result.total || 0);
      setPage(page + 1);
    } catch (error) {
      toast(message(error), 'error');
    } finally {
      setLoadingMore(false);
    }
  };

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
  const quantityByProduct = new Map<number, number>();
  for (const item of items) {
    quantityByProduct.set(item.productId, (quantityByProduct.get(item.productId) ?? 0) + item.quantity);
  }

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

  const removeItem = (item: CartItem) => {
    setConfirmState({
      message: `确定要删除该购物车商品吗？\n${item.productName}`,
      action: async () => {
        await api.post('/pos/cart/removeItem', { cartIds: [item.cartId] });
        await loadCart();
      },
    });
  };

  const clearCart = () => {
    if (!items.length) return;
    setConfirmState({
      message: '清空该购物车后将无法恢复，确定要清空该购物车吗？',
      action: async () => {
        await api.post('/pos/cart/clearCart');
        await loadCart();
      },
    });
  };

  const runConfirm = async () => {
    const current = confirmState;
    setConfirmState(null);
    if (!current) return;
    try {
      await current.action();
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  const editRemark = () => {
    setRemarkDraft(remark);
    setRemarkOpen(true);
  };

  const saveRemark = () => {
    setRemark(remarkDraft.trim());
    setRemarkOpen(false);
    toast('整单备注已保存');
  };

  const loadPending = async () => {
    setPendingLoading(true);
    try {
      const result = await api.post<{ records: PendingOrder[] }>('/pos/cart/pendingOrderList', {
        shopId: shopIdOf(),
      });
      setPendingList(result.records || []);
    } catch (error) {
      toast(message(error), 'error');
    } finally {
      setPendingLoading(false);
    }
  };

  const openPending = () => {
    setPendingOpen(true);
    void loadPending();
  };

  const savePending = async () => {
    if (!items.length) {
      toast('购物车为空', 'error');
      return;
    }
    try {
      await api.post('/pos/cart/pendingOrder', { remark });
      await loadCart();
      await loadPending();
      toast('挂单成功');
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  const takePending = async (pending: PendingOrder) => {
    try {
      const result = await api.post<CartVO>('/pos/cart/takePendingOrder', {
        pendingId: pending.pendingId,
      });
      setCart(result);
      setMember(null);
      setPendingOpen(false);
      toast('取单成功');
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  const openRecharge = () => {
    if (!isMember) {
      toast('请先绑定会员', 'error');
      return;
    }
    setRechargeValue('');
    setRechargeOpen(true);
  };

  const submitRecharge = async () => {
    const value = Number(rechargeValue);
    if (!Number.isFinite(value) || value <= 0) {
      toast('请输入正确的充值金额', 'error');
      return;
    }
    try {
      await api.post('/pos/user/rechargeOrder/create', { userId: member?.userId, money: value });
      if (member?.mobile) {
        const refreshed = await api.get<Member | null>('/pos/user/getUserByMobile', { mobile: member.mobile });
        if (refreshed) setMember(refreshed);
      }
      setRechargeOpen(false);
      toast('充值成功');
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  const openPrice = () => {
    if (!items.length) {
      toast('购物车为空', 'error');
      return;
    }
    setPriceType(1);
    setPriceValue('');
    setPricePreview(null);
    setPriceOpen(true);
  };

  const previewPrice = async () => {
    const value = Number(priceValue);
    if (!Number.isFinite(value) || value <= 0) {
      toast('请输入正确的改价数值', 'error');
      return;
    }
    try {
      const result = await api.post<CartVO>('/pos/cart/changePriceShow', {
        totalChangePriceType: priceType,
        changePriceValue: value,
        userId: member?.userId ?? 0,
        tableId: 0,
      });
      setPricePreview(result?.total?.discountAfter ?? null);
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  const submitPrice = async () => {
    const value = Number(priceValue);
    if (!Number.isFinite(value) || value <= 0) {
      toast('请输入正确的改价数值', 'error');
      return;
    }
    try {
      await api.post('/pos/cart/changePrice', {
        totalChangePriceType: priceType,
        changePriceValue: value,
      });
      await loadCart();
      setPriceOpen(false);
      toast('改价成功');
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  const checkout = () => {
    if (!items.length) {
      toast('购物车为空', 'error');
      return;
    }
    setConfirmState({
      message: `确认收款 ¥${Number(totalAmount).toFixed(2)} 并完成结账？`,
      action: async () => {
        const result = await api.post<{ orderId: number; orderSn: string; totalAmount: string | number }>(
          '/pos/order/check/submit',
          { remark: remark || undefined, markPaid: true },
        );
        await loadCart();
        setMember(null);
        setRemark('');
        toast(`结账成功：${result.orderSn}`);
      },
    });
  };

  const openMember = () => {
    setMemberOpen(true);
    setMemberResult(null);
    setMemberMobile('');
  };

  const searchMember = async () => {
    const mobile = memberMobile.trim();
    if (!mobile) {
      toast('请输入手机号', 'error');
      return;
    }
    setMemberSearching(true);
    try {
      const result = await api.get<Member | null>('/pos/user/getUserByMobile', { mobile });
      if (!result) {
        setMemberResult(null);
        toast('未找到该会员', 'error');
        return;
      }
      setMemberResult(result);
    } catch (error) {
      toast(message(error), 'error');
    } finally {
      setMemberSearching(false);
    }
  };

  const bindMember = async (target: Member) => {
    try {
      const result = await api.post<CartVO>('/pos/cart/list', {
        userId: target.userId,
        tableId: 0,
      });
      setCart(result);
      setMember(target);
      setMemberOpen(false);
      toast(`已绑定会员：${target.nickname || target.mobile}`);
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  const bindTourist = async () => {
    if (member && member.nickname === '游客') {
      setMemberOpen(false);
      return;
    }
    try {
      const userId = await api.post<number>('/pos/posManager/createTourists');
      const result = await api.post<CartVO>('/pos/cart/list', { userId, tableId: 0 });
      setCart(result);
      setMember({ userId, nickname: '游客', mobile: '', balance: 0, points: 0 });
      setMemberOpen(false);
      toast('已切换游客模式');
    } catch (error) {
      toast(message(error), 'error');
    }
  };

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.key !== 'F1') return;
      event.preventDefault();
      setMemberOpen(true);
      setMemberResult(null);
      setMemberMobile('');
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  const isMember = !!member && member.nickname !== '游客';

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
              <button type="button" className="el-button el-button--primary btn large" style={primaryStyle} onClick={openMember}>
                <span>会员(F1)</span>
              </button>
              <button type="button" className="el-button btn" onClick={() => void clearCart()}>
                <span>清空</span>
              </button>
              <div className="dialog-link" style={{ display: 'inline-flex' }}>
                <button
                  type="button"
                  className={isMember ? 'el-button btn' : 'el-button is-disabled btn'}
                  disabled={!isMember}
                  onClick={openRecharge}
                >
                  <span>充值</span>
                </button>
              </div>
              <div className="dialog-link" style={{ display: 'inline-flex' }}>
                <button
                  type="button"
                  className={isMember ? 'el-button btn' : 'el-button is-disabled btn'}
                  disabled={!isMember}
                  onClick={() => toast('优惠券功能接入中')}
                >
                  <span>优惠券</span>
                </button>
              </div>
              <div className="dialog-link" style={{ display: 'inline-flex' }}>
                <button type="button" className="el-button btn" onClick={openPrice}>
                  <span>改价</span>
                </button>
              </div>
              <div className="badge-box">
                <button type="button" className="el-button btn" onClick={openPending}>
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
                  <span className="pos-avatar">{member ? member.nickname.slice(0, 1) : '游'}</span>
                </div>
                <div className="info-left-content">
                  <div className="user-name">
                    <span className="name">{member?.nickname || '游客'}</span>
                  </div>
                </div>
              </div>
              <div className="info-right flex-cloumn">
                <div className="info-right-top">
                  {isMember ? `余额 ¥${Number(member.balance ?? 0).toFixed(2)}` : ''}
                </div>
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
          <div className="switch-view" onClick={() => setDisplayMode(displayMode === 'imageText' ? 'text' : 'imageText')}>
            <div className="icon">
              {displayMode === 'imageText' ? (
                <i className="iconfont-pos icon-sanhengxian" />
              ) : (
                <i className="iconfont-pos icon-tuwendaohang" />
              )}
            </div>
          </div>
        </div>

        <div
          className="product-content"
          onScroll={(event) => {
            const el = event.currentTarget;
            if (el.scrollHeight - el.scrollTop - el.clientHeight < 80) void loadMore();
          }}
        >
          <div className={products.length <= 4 && !loading ? 'product-list single-row' : 'product-list'}>
            {products.map((product) => {
              const quantity = quantityByProduct.get(product.productId) ?? 0;
              const soldOut = (product.productStock ?? 1) <= 0;
              const price = `¥${Number(product.productPrice ?? 0).toFixed(2)}`;
              return (
                <div
                  className="product-item-wrapper"
                  key={product.productId}
                  onClick={() => {
                    if (soldOut) {
                      toast('该商品已售罄', 'error');
                      return;
                    }
                    void addProduct(product);
                  }}
                >
                  <div className="badge-box">
                    {quantity > 0 ? (
                      <div className="badge" style={{ top: -10, right: -10 }}>
                        {quantity}
                      </div>
                    ) : null}
                    {displayMode === 'imageText' ? (
                      <div className={quantity > 0 ? 'product-item active' : 'product-item'}>
                        <div className="img-box">
                          {soldOut ? (
                            <div className="sold-out">
                              <div className="sold-out-img">已售罄</div>
                            </div>
                          ) : null}
                          <img src={product.picThumb || productEmpty} alt="" />
                        </div>
                        <div className="product-info">
                          <div className="name pos-ellipsis-1">{product.productName}</div>
                          <div className="price">{price}</div>
                        </div>
                      </div>
                    ) : (
                      <div className={`product-item${quantity > 0 ? ' active' : ''}${soldOut ? ' soldout' : ''}`}>
                        <div className="product-info2">
                          <div className="name">
                            {soldOut ? <span className="soldout">售罄</span> : null} {product.productName}
                          </div>
                          <div className="price">{price}</div>
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
          {products.length === 0 && !loading ? (
            <div className="empty">
              <img src={productEmpty} alt="" />
            </div>
          ) : null}
          {loadingMore ? (
            <div className="loading">
              <div>
                <span>加载中...</span>
              </div>
            </div>
          ) : null}
          {!loadingMore && !loading && products.length > 12 ? <div className="end-box">没有更多了~</div> : null}
        </div>
      </div>

      <div className="dialog-link" />

      <Dialog
        open={remarkOpen}
        title="整单备注"
        onClose={() => setRemarkOpen(false)}
        footer={
          <>
            <button type="button" className="el-button" onClick={() => setRemarkOpen(false)}>
              <span>取消</span>
            </button>
            <button type="button" className="el-button el-button--primary" style={primaryStyle} onClick={saveRemark}>
              <span>确定</span>
            </button>
          </>
        }
      >
        <textarea
          className="el-textarea__inner"
          rows={4}
          value={remarkDraft}
          placeholder="请输入整单备注"
          onChange={(event) => setRemarkDraft(event.target.value)}
        />
      </Dialog>

      <Dialog
        open={pendingOpen}
        title="挂/取单"
        width={560}
        onClose={() => setPendingOpen(false)}
        footer={
          <button type="button" className="el-button" onClick={() => setPendingOpen(false)}>
            <span>关闭</span>
          </button>
        }
      >
        <div className="pending-toolbar">
          <button type="button" className="el-button el-button--primary" style={primaryStyle} onClick={() => void savePending()}>
            <span>挂单</span>
          </button>
          <span className="pending-tip">保存当前购物车{remark ? `（备注：${remark}）` : ''}</span>
        </div>
        <div className="pending-list">
          {pendingLoading ? (
            <div className="pending-empty">加载中…</div>
          ) : pendingList.length === 0 ? (
            <div className="pending-empty">暂无挂单</div>
          ) : (
            pendingList.map((pending) => (
              <div className="pending-item" key={pending.pendingId}>
                <div className="pending-meta">
                  <strong>#{pending.pendingId}</strong>
                  <span>{pending.remark || '无备注'}</span>
                  <span>{pending.count} 件</span>
                  <strong>¥{Number(pending.amount ?? 0).toFixed(2)}</strong>
                </div>
                <button
                  type="button"
                  className="el-button el-button--primary"
                  style={primaryStyle}
                  onClick={() => void takePending(pending)}
                >
                  <span>取单</span>
                </button>
              </div>
            ))
          )}
        </div>
      </Dialog>

      <Dialog
        open={rechargeOpen}
        title="会员充值"
        width={460}
        onClose={() => setRechargeOpen(false)}
        footer={
          <>
            <button type="button" className="el-button" onClick={() => setRechargeOpen(false)}>
              <span>取消</span>
            </button>
            <button type="button" className="el-button el-button--primary" style={primaryStyle} onClick={() => void submitRecharge()}>
              <span>确认充值</span>
            </button>
          </>
        }
      >
        <div className="member-row">
          <span>会员</span>
          <strong>{member?.nickname || member?.mobile || '-'}</strong>
        </div>
        <div className="member-row">
          <span>当前余额</span>
          <strong>¥{Number(member?.balance ?? 0).toFixed(2)}</strong>
        </div>
        <div className="price-input">
          <div className="el-input">
            <div className="el-input__wrapper">
              <input
                className="el-input__inner"
                placeholder="请输入充值金额"
                value={rechargeValue}
                onChange={(event) => setRechargeValue(event.target.value)}
              />
            </div>
          </div>
          <span className="price-unit">元</span>
        </div>
      </Dialog>

      <Dialog
        open={priceOpen}
        title="整单改价"
        width={460}
        onClose={() => setPriceOpen(false)}
        footer={
          <>
            <button type="button" className="el-button" onClick={() => setPriceOpen(false)}>
              <span>取消</span>
            </button>
            <button type="button" className="el-button" onClick={() => void previewPrice()}>
              <span>预览</span>
            </button>
            <button type="button" className="el-button el-button--primary" style={primaryStyle} onClick={() => void submitPrice()}>
              <span>确认</span>
            </button>
          </>
        }
      >
        <div className="price-types">
          <button
            type="button"
            className={priceType === 1 ? 'price-type is-active' : 'price-type'}
            onClick={() => {
              setPriceType(1);
              setPricePreview(null);
            }}
          >
            一口价
          </button>
          <button
            type="button"
            className={priceType === 2 ? 'price-type is-active' : 'price-type'}
            onClick={() => {
              setPriceType(2);
              setPricePreview(null);
            }}
          >
            折扣
          </button>
        </div>
        <div className="price-input">
          <div className="el-input">
            <div className="el-input__wrapper">
              <input
                className="el-input__inner"
                placeholder={priceType === 1 ? '请输入改价金额' : '请输入折扣（如 90 表示九折）'}
                value={priceValue}
                onChange={(event) => setPriceValue(event.target.value)}
              />
            </div>
          </div>
          <span className="price-unit">{priceType === 1 ? '元' : '%'}</span>
        </div>
        <div className="member-row">
          <span>原金额</span>
          <strong>¥{Number(totalAmount).toFixed(2)}</strong>
        </div>
        <div className="member-row">
          <span>改价后金额</span>
          <strong>{pricePreview === null ? '—' : `¥${Number(pricePreview).toFixed(2)}`}</strong>
        </div>
      </Dialog>

      <Dialog
        open={memberOpen}
        title="会员"
        width={460}
        onClose={() => setMemberOpen(false)}
        footer={
          <>
            <button type="button" className="el-button" onClick={() => void bindTourist()}>
              <span>游客</span>
            </button>
            <button type="button" className="el-button" onClick={() => setMemberOpen(false)}>
              <span>关闭</span>
            </button>
          </>
        }
      >
        <div className="member-search">
          <div className="el-input">
            <div className="el-input__wrapper">
              <input
                className="el-input__inner"
                placeholder="请输入手机号"
                value={memberMobile}
                onChange={(event) => setMemberMobile(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') void searchMember();
                }}
              />
            </div>
          </div>
          <button
            type="button"
            className="el-button el-button--primary"
            style={primaryStyle}
            disabled={memberSearching}
            onClick={() => void searchMember()}
          >
            <span>查询</span>
          </button>
        </div>

        {memberResult ? (
          <div className="member-card">
            <div className="member-row">
              <strong>{memberResult.nickname || '-'}</strong>
              <span>{memberResult.mobile || '-'}</span>
            </div>
            <div className="member-row">
              <span>余额 ¥{Number(memberResult.balance ?? 0).toFixed(2)}</span>
              <span>积分 {memberResult.points ?? 0}</span>
            </div>
            <button
              type="button"
              className="el-button el-button--primary"
              style={primaryStyle}
              onClick={() => void bindMember(memberResult)}
            >
              <span>绑定会员</span>
            </button>
          </div>
        ) : (
          <div className="member-empty">输入手机号查询会员</div>
        )}
      </Dialog>

      <Dialog
        open={!!confirmState}
        title="提示"
        onClose={() => setConfirmState(null)}
        footer={
          <>
            <button type="button" className="el-button" onClick={() => setConfirmState(null)}>
              <span>取消</span>
            </button>
            <button type="button" className="el-button el-button--primary" style={primaryStyle} onClick={() => void runConfirm()}>
              <span>确定</span>
            </button>
          </>
        }
      >
        <p className="confirm-text">{confirmState?.message}</p>
      </Dialog>
    </div>
  );
}
