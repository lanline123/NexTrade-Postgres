/**
 * 文章列表 设置（字段对齐 view/admin article/Settings.vue）
 *
 * 两页签（原版 el-tabs：文章设置 / 扩展设置）：
 * - 文章设置：选择风格 / 文章数量（el-input 数字、无上限）/ 参数设置（三个显示开关均为
 *   el-radio 显示/不显示）/ 图片圆角半径（slider max 100）/ 标题粗细（400/700）。
 *   文章分类 SelectArticleCategory 属数据选择器、尚未移植，本批不渲染假控件（详见批次回报）。
 * - 扩展设置：颜色设置五色（含 moduleColor.statisticColor，默认 #999999，见 index.ts defaults）
 *   + ModuleStyleEdit/ContentStyleEdit（ExtensionStyleSettings type="article"）。
 */
import { Input } from "@douyinfe/semi-ui";
import { SettingGroup, SettingRow, RadioRowField, RangeField, ColorField } from "../../../shared/settingWidgets";
import { SettingsTabs, ExtensionStyleSettings } from "../../../shared/styleEdits";

export default function ArticleSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const moduleColor = data?.moduleColor ?? {};
    const setColor = (patch: Record<string, any>) => onChange({ moduleColor: { ...moduleColor, ...patch } });
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["文章设置", "扩展设置"]}>
                {[
                    <div key="article">
                        <SettingGroup>
                            <RadioRowField
                                data={data}
                                onChange={onChange}
                                field="style"
                                label="选择风格"
                                options={[
                                    { value: 1, label: "大图展示" },
                                    { value: 2, label: "两列展示(纵向)" },
                                    { value: 3, label: "两列展示(横向)" }
                                ]}
                            />
                            {/* 文章数量：原版为 el-input 数字输入、无上限（不用滑杆形态） */}
                            <SettingRow label="文章数量">
                                <Input
                                    type="number"
                                    value={data?.articleNum ?? ""}
                                    placeholder="请输入文章数量"
                                    style={{ width: 240, maxWidth: "100%" }}
                                    onChange={(v: string) => onChange({ articleNum: v === "" ? "" : Number(v) })}
                                />
                            </SettingRow>
                        </SettingGroup>
                        <SettingGroup title="参数设置">
                            <RadioRowField
                                data={data}
                                onChange={onChange}
                                field="showTime"
                                label="文章日期时间"
                                options={[
                                    { value: 1, label: "显示" },
                                    { value: 0, label: "不显示" }
                                ]}
                            />
                            <RadioRowField
                                data={data}
                                onChange={onChange}
                                field="showPageView"
                                label="文章浏览量"
                                options={[
                                    { value: 1, label: "显示" },
                                    { value: 0, label: "不显示" }
                                ]}
                            />
                            <RadioRowField
                                data={data}
                                onChange={onChange}
                                field="showLike"
                                label="文章点赞数量"
                                options={[
                                    { value: 1, label: "显示" },
                                    { value: 0, label: "不显示" }
                                ]}
                            />
                        </SettingGroup>
                        <SettingGroup>
                            <RangeField data={data} onChange={onChange} field="picRadius" label="图片圆角半径" max={100} />
                            <RadioRowField
                                data={data}
                                onChange={onChange}
                                field="articleNameWeight"
                                label="标题粗细"
                                options={[
                                    { value: 400, label: "常规体" },
                                    { value: 700, label: "加粗体" }
                                ]}
                            />
                        </SettingGroup>
                    </div>,
                    <div key="extend">
                        <SettingGroup title="颜色设置">
                            <ColorField data={moduleColor} onChange={setColor} field="articleNameColor" label="文章标题颜色" />
                            <ColorField data={moduleColor} onChange={setColor} field="articleTimeColor" label="时间日期颜色" />
                            <ColorField data={moduleColor} onChange={setColor} field="articleViewColor" label="浏览量颜色" />
                            <ColorField data={moduleColor} onChange={setColor} field="articleLikeColor" label="点赞颜色" />
                            <ColorField data={moduleColor} onChange={setColor} field="statisticColor" label="统计数量颜色" />
                        </SettingGroup>
                        <ExtensionStyleSettings data={data} onChange={onChange} type="article" />
                    </div>
                ]}
            </SettingsTabs>
        </div>
    );
}
