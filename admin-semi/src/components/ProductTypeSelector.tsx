import { Tooltip} from '@douyinfe/semi-ui';
import { IconHelpCircle} from '@douyinfe/semi-icons';
import { useI18n} from '../i18n';

interface ProductTypeSelectorProps {
value: number;
disabled?: boolean;
onChange: (value: number) => void;
}

interface ProductTypeOption {
value: number;
label: string;
desc: string;
tips?: string;
}

/**
* 商品类型卡片式单选（对齐原版 tigshop RadioType 形态的现代化改版）：
* 选项/取值与后端枚举严格一致（1 普通 2 虚拟 3 卡密 4 付费内容 5 预约到店
* 6 上门服务 7 实体礼品卡 8 电子礼品卡），每项带原版 desc 与 tips 说明；
* 视觉为现代扁平直角：1px 边框、主题色选中态、右下角直角三角对勾。
* 5/6 的 desc 为我方新增文案（原版该两项未提供 desc）。
*/
export default function ProductTypeSelector({ value, disabled, onChange}: ProductTypeSelectorProps) {
const { t} = useI18n();
const options: ProductTypeOption[] = [
{ value: 1, label: t('普通商品'), desc: t('物流配送')},
{
value: 2,
label: t('虚拟商品'),
desc: t('无需物流'),
tips: t('网盘链接、视频链接，在添加商品添加虚拟商品。用户购买后将在订单详情直接展示。'),
},
{
value: 3,
label: t('卡密商品'),
desc: t('无需物流'),
tips: t(
'可以在菜单中创建卡券组，然后在添加单个卡密或通过下载填好卡密后在通过批量导入电子卡券按钮上传该文件批量导入。',
),
},
{
value: 4,
label: t('付费内容'),
desc: t('无需物流'),
tips: t(
'软件授权、在线课程等。通过简单的操作，即可将这些商品添加到您的商店中，为客户提供更丰富的选择。立即体验，享受便捷的付费内容管理服务。',
),
},
{ value: 5, label: t('预约到店'), desc: t('预约后到店消费')},
{ value: 6, label: t('上门服务'), desc: t('预约后上门服务')},
{
value: 7,
label: t('实体礼品卡'),
desc: t('快递发货实物卡片'),
tips: t('实体礼品卡：购买后在收到货后刮开图层查看卡密进行兑换，根据礼品卡的类型可用于兑换商品{value}', {
value: t('或充值余额'),
}),
},
{
value: 8,
label: t('电子礼品卡'),
desc: t('虚拟卡号，无实体卡片'),
tips: t('电子礼品卡：购买后可在订单详情查看卡号及卡密，也可转增；根据礼品卡的类型可用于兑换商品{value}', {
value: t('或充值余额'),
}),
},
];
const current = Number(value || 1);

return (
<div className="product-type-selector-wrap">
<div className="product-type-hint">
{t('不同商品类型可编辑的字段内容不同，商品类型一旦发布后将不可更改')}
</div>
<div
className={`product-type-selector${disabled? ' is-disabled': ''}`}
role="radiogroup"
aria-label={t('商品类型')}
>
{options.map((option) => {
const selected = current === option.value;
return (
<button
type="button"
key={option.value}
role="radio"
aria-checked={selected}
className={`product-type-card${selected? ' is-selected': ''}`}
onClick={() => {
if (!disabled) onChange(option.value);
}}
>
<span className="product-type-card-head">
<span className="product-type-card-title">{option.label}</span>
{option.tips && (
<span
className="product-type-card-help"
onClick={(event) => event.stopPropagation()}
>
<Tooltip
position="bottom"
content={<span className="product-type-tips">{option.tips}</span>}
>
<IconHelpCircle />
</Tooltip>
</span>
)}
</span>
<span className="product-type-card-desc">{option.desc}</span>
{selected && (
<span className="product-type-card-corner" aria-hidden="true">
<svg viewBox="0 0 10 10" width="9" height="9">
<path
d="M1.5 5.5 4 8 8.5 2.5"
fill="none"
stroke="currentColor"
strokeWidth="1.8"
strokeLinecap="round"
strokeLinejoin="round"
/>
</svg>
</span>
)}
</button>
);
})}
</div>
{![1, 5, 6].includes(current) && (
<div className="product-type-hint">{t('该类型商品同样需要客户填写收货地址')}</div>
)}
</div>
);
}
