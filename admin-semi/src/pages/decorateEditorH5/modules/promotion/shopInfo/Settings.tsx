/**
 * 店铺信息 设置（对齐 view/admin shopInfo/Settings.vue；原版无页签、无 frame 编辑组）
 *
 * 指示器样式 style（样式一..四）→ 背景图：10 张预设缩略图网格（5 列 × 2 行，35px 高，
 * 选中边框 #155bd4，点击写 backgroundDefault=序号）+「自定义」单选（backgroundDefault=0）→
 * 自定义时出现「自定义背景图片」组：背景图片 PicSelect（customPic）。
 */
import { Radio, RadioGroup } from "@douyinfe/semi-ui";
import { SettingGroup, SettingRow, SegmentedRadio, PicSelectField, RadioRowField } from "../../../shared/settingWidgets";
import { SHOP_INFO_BACKGROUNDS } from "./backgrounds";

export default function ShopInfoSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const backgroundDefault = data?.backgroundDefault ?? 1;
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup>
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="style"
                    label="指示器样式"
                    options={[
                        { value: 1, label: "样式一" },
                        { value: 2, label: "样式二" },
                        { value: 3, label: "样式三" },
                        { value: 4, label: "样式四" }
                    ]}
                />
                <SettingRow label="背景图" block>
                    <div
                        style={{
                            boxSizing: "border-box",
                            width: "100%",
                            display: "grid",
                            gridTemplateColumns: "repeat(5, 1fr)",
                            gridTemplateRows: "repeat(2, 1fr)",
                            gap: "5px 10px"
                        }}
                    >
                        {SHOP_INFO_BACKGROUNDS.map((img, index) => (
                            <div
                                key={index}
                                onClick={() => onChange({ backgroundDefault: index + 1 })}
                                style={{
                                    height: 35,
                                    border: `1px solid ${backgroundDefault === index + 1 ? "#155bd4" : "#ebedf0"}`,
                                    background: backgroundDefault === index + 1 ? "#e0edff" : undefined,
                                    padding: 1,
                                    cursor: "pointer",
                                    boxSizing: "border-box"
                                }}
                            >
                                <img src={img} alt="" style={{ width: "100%", height: "100%", objectFit: "cover", display: "block" }} />
                            </div>
                        ))}
                    </div>
                    <div style={{ marginTop: 10 }}>
                        <RadioGroup type="button" value={backgroundDefault === 0 ? 0 : undefined} onChange={(e: any) => e?.target?.value === 0 && onChange({ backgroundDefault: 0 })}>
                            <Radio value={0}>自定义</Radio>
                        </RadioGroup>
                    </div>
                </SettingRow>
            </SettingGroup>
            {backgroundDefault === 0 ? (
                <>
                    <div style={{ padding: "8px 0 0", fontSize: 14, color: "#969799", whiteSpace: "nowrap" }}>自定义背景图片</div>
                    <SettingGroup>
                        <PicSelectField data={data} onChange={onChange} field="customPic" label="背景图片" />
                    </SettingGroup>
                </>
            ) : null}
        </div>
    );
}
