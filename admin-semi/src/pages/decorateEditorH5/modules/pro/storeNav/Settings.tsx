/**
 * 门店导航 设置（对齐 view/admin mobilePro/storeNav/Settings.vue）
 *
 * 页签：内容设置 / 扩展设置（原版扩展页仅 ModuleStyleEdit type="noticeA2"；
 * ContentStyleEdit type="noticeA2" 在原版位于内容页签内，照此保留）。
 * 指定门店的「选择门店」为 SelectStorePop 弹窗（门店数据选择组件），本编辑器无门店
 * 选择 API：已选门店以列表承接（可删除/上下移排序，替代原版拖拽），选择动作保留
 * 文案提示（任务书口径，小结注明）。CommonSubTitleEdit 以 SubTitleSettingsField
 * type="categoryRec" 承接（原版两处均为该 type）。
 */
import { Button } from "@douyinfe/semi-ui";
import {
    SettingGroup,
    SettingRow,
    RadioRowField,
    RangeField,
    NumberField,
    ColorField
} from "../../../shared/settingWidgets";
import { SettingsTabs, SubTitleSettingsField, ContentStyleSettingsField, ModuleStyleSettingsField } from "../../../shared/styleEdits";
import { useI18n } from "../../../../../i18n";

type Patch = Record<string, any>;
interface Props {
    data: Record<string, any>;
    onChange: (patch: Patch) => void;
}

const SHOW_OPTIONS = [
    { value: 1, label: "显示" },
    { value: 0, label: "不显示" }
];

export default function StoreNavSettings({ data, onChange }: Props) {
    const { t } = useI18n();
    const isTwo = Number(data?.storeStyle) === 2;
    const storeList: Array<Record<string, any>> = Array.isArray(data?.storeList) ? data.storeList : [];
    const moveStore = (index: number, delta: number) => {
        const target = index + delta;
        if (target < 0 || target >= storeList.length) return;
        const next = [...storeList];
        [next[index], next[target]] = [next[target], next[index]];
        onChange({ storeList: next });
    };

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <>
                    <SettingGroup title="样式设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="storeStyle"
                            label="门店样式"
                            options={[
                                { value: 1, label: "样式一" },
                                { value: 2, label: "样式二" }
                            ]}
                        />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="storeSource"
                            label="展示门店"
                            options={[
                                { value: 1, label: "自动获取" },
                                { value: 2, label: "指定门店" }
                            ]}
                        />
                        {Number(data?.storeSource) === 1 ? (
                            <>
                                <NumberField data={data} onChange={onChange} field="storeNum" label="门店数量" min={1} max={20} />
                                <div style={{ padding: "0 16px 8px", color: "#999", fontSize: 12 }}>{t("根据定位数据展示由近到远门店择优展示")}</div>
                            </>
                        ) : (
                            <>
                                <SettingRow label="选择门店" block>
                                    <div style={{ color: "#646a73", fontSize: 12, lineHeight: 1.6 }}>
                                        门店选择器（SelectStore 弹窗）尚未接入本编辑器，已选门店保持不变，可在下方调整顺序或删除。最多支持20个门店。
                                    </div>
                                </SettingRow>
                                {storeList.length > 0 ? (
                                    <div style={{ padding: "0 16px 10px" }}>
                                        {storeList.map((store, index) => (
                                            <div
                                                key={store.shopId ?? index}
                                                style={{ display: "flex", alignItems: "center", gap: 8, backgroundColor: "#fff", borderRadius: 2, boxShadow: "0 0 4px 0 rgba(10, 42, 97, 0.2)", padding: "8px 10px", marginTop: 8 }}
                                            >
                                                <span style={{ flex: 1, fontSize: 13 }}>{store.shopTitle || `门店${store.shopId ?? ""}`}</span>
                                                <Button size="small" theme="borderless" disabled={index === 0} onClick={() => moveStore(index, -1)}>
                                                    上移
                                                </Button>
                                                <Button size="small" theme="borderless" disabled={index === storeList.length - 1} onClick={() => moveStore(index, 1)}>
                                                    下移
                                                </Button>
                                                <Button size="small" theme="borderless" type="danger" onClick={() => onChange({ storeList: storeList.filter((_, i) => i !== index) })}>
                                                    删除
                                                </Button>
                                            </div>
                                        ))}
                                    </div>
                                ) : null}
                            </>
                        )}
                    </SettingGroup>
                    {!isTwo ? (
                        <>
                            <SubTitleSettingsField data={data} onChange={onChange} field="subTitle" title="标题" type="categoryRec" />
                            <RangeField data={data} onChange={onChange} field="titlePaddingY" label="标题上下间距" max={30} />
                            <RangeField data={data} onChange={onChange} field="titlePaddingX" label="标题左右间距" max={30} />
                        </>
                    ) : null}
                    <SubTitleSettingsField data={data} onChange={onChange} field="storeTitleStyle" title="门店标题" type="categoryRec" />
                    <RadioRowField data={data} onChange={onChange} field="showRating" label="门店评分" options={SHOW_OPTIONS} />
                    {isTwo ? (
                        <>
                            <RadioRowField data={data} onChange={onChange} field="showFollow" label="关注按钮" options={SHOW_OPTIONS} />
                            <RadioRowField data={data} onChange={onChange} field="showMinOrder" label="起送价" options={SHOW_OPTIONS} />
                            <RadioRowField data={data} onChange={onChange} field="showDeliveryFee" label="配送费" options={SHOW_OPTIONS} />
                            <RadioRowField data={data} onChange={onChange} field="showDistance" label="距离" options={SHOW_OPTIONS} />
                        </>
                    ) : null}
                    <ContentStyleSettingsField data={data} onChange={onChange} type="noticeA2" />
                    <RangeField data={data} onChange={onChange} field="storeGap" label="门店间距" max={30} />
                    <ColorField data={data} onChange={onChange} field="storeBackgroundColor" label="门店背景颜色" resetValue="#ffffff" />
                    <RangeField data={data} onChange={onChange} field="storePaddingY" label="门店上下间距" max={30} />
                    <RangeField data={data} onChange={onChange} field="storePaddingX" label="门店左右间距" max={30} />
                    <RangeField data={data} onChange={onChange} field="storeRadius" label="门店圆角半径" max={30} />
                    {isTwo ? (
                        <>
                            <RangeField data={data} onChange={onChange} field="productGap" label="商品项间距" max={30} />
                            <RadioRowField data={data} onChange={onChange} field="showProductName" label="商品名称" options={SHOW_OPTIONS} />
                            {Number(data?.showProductName) === 1 ? (
                                <RadioRowField
                                    data={data}
                                    onChange={onChange}
                                    field="productNameRow"
                                    label="商品名称行数"
                                    options={[
                                        { value: 1, label: "一行" },
                                        { value: 2, label: "两行" }
                                    ]}
                                />
                            ) : null}
                            <RadioRowField data={data} onChange={onChange} field="showProductPrice" label="商品价格" options={SHOW_OPTIONS} />
                        </>
                    ) : null}
                </>
                <>
                    <ModuleStyleSettingsField data={data} onChange={onChange} type="noticeA2" />
                </>
            </SettingsTabs>
        </div>
    );
}
