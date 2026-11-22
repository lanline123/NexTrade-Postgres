/**
 * 商品详情页首屏骨架（/product/[id] 与 /item/[id] 共用同一份）。
 *
 * 详情页是服务端取数（force-dynamic），此前没有 loading.tsx：从列表/搜索
 * 点进商品时，旧页面一直停留到新页数据齐了才整页切换，无任何过渡反馈。
 * 三套详情风格（productDetailStyleType）版式不同，骨架用中性近似：
 * 左图右文（标题/价格/规格行/购买按钮）+ 下方整宽详情块，样式类
 * .pc-pdp-skeleton* 定义在 globals.scss 的 UX 基础层。
 */
export default function ProductDetailLoading() {
  return (
    <main className="pc-pdp-skeleton" aria-hidden="true">
      <div className="container">
        <div className="pc-pdp-skeleton__main">
          <div className="pc-pdp-skeleton__block pc-pdp-skeleton__gallery" />
          <div className="pc-pdp-skeleton__info">
            <div className="pc-pdp-skeleton__block pc-pdp-skeleton__line pc-pdp-skeleton__line--title" />
            <div className="pc-pdp-skeleton__block pc-pdp-skeleton__line pc-pdp-skeleton__line--short" />
            <div className="pc-pdp-skeleton__block pc-pdp-skeleton__line pc-pdp-skeleton__line--price" />
            <div className="pc-pdp-skeleton__block pc-pdp-skeleton__line" />
            <div className="pc-pdp-skeleton__block pc-pdp-skeleton__line" />
            <div className="pc-pdp-skeleton__block pc-pdp-skeleton__line pc-pdp-skeleton__line--short" />
            <div className="pc-pdp-skeleton__block pc-pdp-skeleton__line pc-pdp-skeleton__line--btn" />
          </div>
        </div>
        <div className="pc-pdp-skeleton__block pc-pdp-skeleton__detail" />
      </div>
    </main>
  );
}
