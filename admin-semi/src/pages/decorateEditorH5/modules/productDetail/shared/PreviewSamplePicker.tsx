/**
 * 预览样例商品选择（2026-11-21 方案二，我方新增）
 *
 * 商品详情编辑器的真实渲染预览此前固定取第一条在售商品作样例，活动态
 * （秒杀/团购标题条）与价格门控口径无从验版。本组件让商家指定样例商品：
 * 选择结果只写编辑器会话状态（editorStore.previewProductId，随
 * decorate-data 下发给嵌入预览页），不写模块装修数据、不进保存。
 * 不选（0）时预览保持默认口径（第一条在售商品）。
 */
import { useState } from "react";
import { Button } from "@douyinfe/semi-ui";
import ProductPickerModal from "../../../../../components/ProductPickerModal";
import { editorStore, useEditorState } from "../../../store/editorStore";
import { useI18n } from "../../../../../i18n";

export default function PreviewSamplePicker() {
    const { t } = useI18n();
    const previewProductId = useEditorState().previewProductId;
    const [pickerVisible, setPickerVisible] = useState(false);
    /** 本会话内经弹窗选中的商品名（store 只存 id；未选过时以「商品 #id」兜底展示） */
    const [pickedName, setPickedName] = useState("");

    return (
        <div>
            <div style={{ fontSize: 13, color: "#1c1f23", marginBottom: 8 }}>
                {previewProductId > 0 ? pickedName || `${t("商品")} #${previewProductId}` : t("默认（第一条在售商品）")}
            </div>
            <div style={{ display: "flex", gap: 8 }}>
                <Button size="small" onClick={() => setPickerVisible(true)}>
                    {previewProductId > 0 ? t("更换样例商品") : t("选择样例商品")}
                </Button>
                {previewProductId > 0 ? (
                    <Button
                        size="small"
                        onClick={() => {
                            editorStore.setPreviewProductId(0);
                            setPickedName("");
                        }}
                    >
                        {t("清除")}
                    </Button>
                ) : null}
            </div>
            <div style={{ fontSize: 12, color: "#86909c", marginTop: 8 }}>{t("指定秒杀/团购等活动商品可预览活动态与价格门控效果，仅影响编辑预览")}</div>
            <ProductPickerModal
                visible={pickerVisible}
                selectedIds={previewProductId > 0 ? [previewProductId] : []}
                multiple={false}
                max={1}
                onOk={(ids, rows) => {
                    const id = ids[0];
                    if (id) {
                        editorStore.setPreviewProductId(id);
                        setPickedName(rows[0]?.productName ?? "");
                    }
                    setPickerVisible(false);
                }}
                onCancel={() => setPickerVisible(false)}
            />
        </div>
    );
}
