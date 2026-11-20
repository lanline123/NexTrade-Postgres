/**
* 图片魔方 设置（字段对齐 view/admin imageSquareAd/Settings.vue）
*
* 布局选择：按 picType 渲染魔方缩略图宫格，点击选中图片位（picIndex），
* 选中后在下方面板替换该位置图片/编辑链接（对齐 Vue 版 PicList v-model:photo 行为）；
* 切换模板时按 Vue 版 onPicTypeChange 的 lengthMap 调整 picList 数量。
* 「选择布局」使用原版.dec-square-group 类名结构（dec-square-item / square-item-row /
* square-item-col / square-item-box.selected），样式由全局 styles/css/decorate.css 的
*.dec-square-group 规则提供（整宽 flex、盒:after border #ebedf0、选中 border #155bd4 + bg #eef2ff），
* 与原版 Settings.vue 的 DOM 逐 picType 对应；盒内显示 picList 实际缩略图（原版即如此，非示意图）。
* 「选择模板」原版为 ico-decorate 图标 radio，2026-11-20 起走共享视觉选择器
* TemplateIconRadioField。
* 「添加图片」原版为 PicList 单图模式（isMultiple=false），2026-11-20 起走共享
* PicItemEditor（与 PicListField 同源的条目编辑器：图盒+更换图片条/标题/链接选择器/
* × 清空槽位）；此前手写老式编辑块（h5-editor-pic-row + 红色删除按钮 + 裸 URL 输入、
* 且丢了原版「标题」字段）已废弃。
* 2026-11-21 晚起改调共享图片列表编辑器（PicListEditor，components/PicListEditor，
* 自 PcDecorateEditor 抽取升格，用户 2026-11-21 定版纠正落点）：以单槽位绑定
* （value=[当前图片位]、max=1）编辑当前图片位——点卡打开「编辑图片」弹窗，弹窗内
* 素材中心选图与「从商品选择图片」两入口并列、选图框在左标题/链接在右，与 PC
* 编辑器同形；晨版 ProductPicItemEditor（卡面外挂工具条形态）已废弃移除。
* 商品批量落位经 onPickManyProducts 委托回本文件 fillFromProducts：自当前槽位起
* 顺延填充后续槽位（productPicPatch 落图+标题+商品链接，H5 LinkType 口径），
* 槽位不足时提示；弹窗内「删除」= 清空当前槽位（clearCurrent 口径不变）。
* 字段口径不变（picList 条目同前）。
*/
import { useEffect, useState} from "react";
import { Toast} from "@douyinfe/semi-ui";
import { SettingGroup, NumberField, TitleSettingsField, FrameSettingsField, productPicPatch} from "../../../shared/settingWidgets";
import type { PickedImage} from "../../../../../components/ProductImagePicker";
import PicListEditor from "../../../../../components/PicListEditor";
import { TemplateIconRadioField} from "../../../shared/templateIcon";
import { imageFormat} from "../../../shared/format";

import { useI18n} from '../../../../../i18n';
const lengthMap: Record<number, number> = { 1: 2, 2: 3, 5: 3, 7: 3, 3: 4, 4: 4, 6: 4};
const emptyPic = { picId: 0, picThumb: "", picUrl: "", picName: ""};

/** 原版.square-item-box（选中加.selected；整高列位加.square-item-box-col） */
function SquareBox({ src, selected, onClick, col = false}: { src?: string; selected: boolean; onClick: () => void; col?: boolean}) {
return (
<div className={`square-item-box${col? " square-item-box-col": ""}${selected? " selected": ""}`} onClick={onClick}>
{src? <img src={imageFormat(src)} alt="" />: null}
</div>
);
}

export default function ImageSquareAdSettings({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
const { t} = useI18n();
const picList: any[] = Array.isArray(data?.picList)? data.picList: [];
const [picIndex, setPicIndex] = useState(0);

// 对齐 Vue 版 onPicTypeChange：切换模板时重置选中位并按 lengthMap 调整 picList 数量
useEffect(() => {
setPicIndex(0);
const length = lengthMap[Number(data?.picType)] || 2;
if (picList.length === length) return;
let next: any[];
if (length >= picList.length) {
next = [...picList,...Array.from({ length: length - picList.length}, () => ({...emptyPic}))];
} else {
next = picList.slice(0, length); // 与 Vue 版 splice(-(now - length)) 一致：从尾部裁剪
}
onChange({ picList: next});
// eslint-disable-next-line react-hooks/exhaustive-deps
}, [data?.picType]);

const setPic = (index: number, patch: Record<string, any>) => {
const next = [...picList];
next[index] = {...next[index],...patch};
onChange({ picList: next});
};

const currentIndex = Math.min(picIndex, picList.length - 1);
const currentItem = picList[currentIndex];
const showLayoutGrid = [1, 2, 3].includes(Number(data?.picType)) || ([4, 5, 6, 7].includes(Number(data?.picType)) && picList.length === lengthMap[Number(data?.picType)]);

// 原版 PicList 单图模式 removePhoto 语义：清空该槽位的图对象（槽位保留，长度由模板决定）
const clearCurrent = () =>
setPic(currentIndex, { picId: 0, picThumb: "", picUrl: "", picName: "", picTitle: "", picLink: { link: "", title: ""}});

// 商品直选批量落位（PC PicListEditor onPickMany 的魔方等价形）：槽位由模板定长、
// 不能追加，自当前槽位起顺延填充后续槽位；超出部分丢弃并提示。
const fillFromProducts = (items: PickedImage[]) => {
const next = [...picList];
let filled = 0;
items.forEach((picked, offset) => {
const slot = currentIndex + offset;
if (slot >= next.length) return;
next[slot] = {...next[slot],...productPicPatch(next[slot], picked, t('商品详情页'))};
filled += 1;
});
if (!filled) return;
onChange({ picList: next});
if (filled < items.length) {
Toast.warning(t('图片位不足，已填入 {count} 个商品', { count: filled}));
}
};

return (
<div className="h5-editor-settings-body">
<TemplateIconRadioField
data={data}
onChange={onChange}
field="picType"
label="选择模板"
options={[
{ value: 1, label: "一行两个", icon: "cuberow"},
{ value: 2, label: "一行三个", icon: "cuberow2"},
{ value: 3, label: "一行四个", icon: "cuberow1"},
{ value: 4, label: "左二右二", icon: "cube"},
{ value: 5, label: "左一右二", icon: "cubeto"},
{ value: 6, label: "左一右三", icon: "cubeto1"},
{ value: 7, label: "上一下二", icon: "cube-upto"}
]}
/>
<SettingGroup title="选择布局">
{showLayoutGrid && (
<div className="dec-square-group">
{[1, 2, 3].includes(Number(data?.picType)) && (
<div className="dec-square-item dec-square-item-empty">
<div className="square-item-row">
{picList.map((item, index) => (
<SquareBox key={index} src={item?.picThumb} selected={picIndex === index} onClick={() => setPicIndex(index)} />
))}
</div>
</div>
)}
{Number(data?.picType) === 4 && picList.length === 4 && (
<div className="dec-square-item dec-square-item-4 dec-square-item-empty">
<div className="square-item-row">
<div className="square-item-col">
<SquareBox src={picList[0]?.picThumb} selected={picIndex === 0} onClick={() => setPicIndex(0)} />
<SquareBox src={picList[1]?.picThumb} selected={picIndex === 1} onClick={() => setPicIndex(1)} />
</div>
<div className="square-item-col">
<SquareBox src={picList[2]?.picThumb} selected={picIndex === 2} onClick={() => setPicIndex(2)} />
<SquareBox src={picList[3]?.picThumb} selected={picIndex === 3} onClick={() => setPicIndex(3)} />
</div>
</div>
</div>
)}
{Number(data?.picType) === 5 && picList.length === 3 && (
<div className="dec-square-item dec-square-item-5 dec-square-item-empty">
<div className="square-item-row">
<div className="square-item-col">
<SquareBox col src={picList[0]?.picThumb} selected={picIndex === 0} onClick={() => setPicIndex(0)} />
</div>
<div className="square-item-col">
<SquareBox src={picList[1]?.picThumb} selected={picIndex === 1} onClick={() => setPicIndex(1)} />
<SquareBox src={picList[2]?.picThumb} selected={picIndex === 2} onClick={() => setPicIndex(2)} />
</div>
</div>
</div>
)}
{Number(data?.picType) === 7 && picList.length === 3 && (
<div className="dec-square-item dec-square-item-empty">
<div className="square-item-col">
<div className="square-item-row">
<SquareBox src={picList[0]?.picThumb} selected={picIndex === 0} onClick={() => setPicIndex(0)} />
</div>
<div className="square-item-row">
<SquareBox src={picList[1]?.picThumb} selected={picIndex === 1} onClick={() => setPicIndex(1)} />
<SquareBox src={picList[2]?.picThumb} selected={picIndex === 2} onClick={() => setPicIndex(2)} />
</div>
</div>
</div>
)}
{Number(data?.picType) === 6 && picList.length === 4 && (
<div className="dec-square-item dec-square-item-7 dec-square-item-empty">
<div className="square-item-row">
<div className="square-item-col">
<SquareBox col src={picList[0]?.picThumb} selected={picIndex === 0} onClick={() => setPicIndex(0)} />
</div>
<div className="square-item-col">
<div className="square-item-col">
<div className="square-item-row">
<SquareBox src={picList[1]?.picThumb} selected={picIndex === 1} onClick={() => setPicIndex(1)} />
</div>
<div className="square-item-row">
<SquareBox src={picList[2]?.picThumb} selected={picIndex === 2} onClick={() => setPicIndex(2)} />
<SquareBox src={picList[3]?.picThumb} selected={picIndex === 3} onClick={() => setPicIndex(3)} />
</div>
</div>
</div>
</div>
</div>
)}
</div>
)}
</SettingGroup>
<SettingGroup title="添加图片">
{currentItem? (
<PicListEditor
value={[currentItem]}
max={1}
onChange={(next) => {
if (!next.length) { clearCurrent(); return; }
setPic(currentIndex, next[0] as Record<string, any>);
}}
onPickManyProducts={fillFromProducts}
/>
): (
<div style={{ fontSize: 12, color: "#999"}}>{t('请先选择布局并选中图片位后编辑')}</div>
)}
</SettingGroup>
<SettingGroup title="图片设置">
<NumberField data={data} onChange={onChange} field="imgPadding" label="图片边距" suffix="px" max={30} />
</SettingGroup>
<TitleSettingsField data={data} onChange={onChange} />
<FrameSettingsField data={data} onChange={onChange} />
</div>
);
}
