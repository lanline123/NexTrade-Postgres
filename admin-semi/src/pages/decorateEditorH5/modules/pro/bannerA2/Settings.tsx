/**
 * 首焦轮播A2 设置（对齐 view/admin mobilePro/bannerA2/Settings.vue）
 *
 * 页签：基础设置（BannerContent + BannerSearch）/ 样式设置（BannerStyle
 * type="bannerA2"）/ 扩展设置（ModuleStyleEdit type="banner"）。
 */
import { RadioRowField, RangeField, ColorField, PicListField, TextField, DecDivider } from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField } from "../../../shared/styleEdits";

export default function BannerA2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const bannerContent = data?.bannerContent ?? {};
    const setBanner = (patch: Record<string, any>) => onChange({ bannerContent: { ...bannerContent, ...patch } });
    const searchContent = data?.searchContent ?? {};
    const setSearch = (patch: Record<string, any>) => onChange({ searchContent: { ...searchContent, ...patch } });
    const bannerStyle = data?.bannerStyle ?? {};
    const setStyle = (patch: Record<string, any>) => onChange({ bannerStyle: { ...bannerStyle, ...patch } });

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["基础设置", "样式设置", "扩展设置"]}>
                <>
                    <PicListField data={bannerContent} onChange={setBanner} field="picList" label="轮播内容设置" max={10} showText={false} addText="添加一组" />
                    <RangeField data={bannerContent} onChange={setBanner} field="picRadius" label="图片圆角" max={20} />
                    <RadioRowField
                        data={bannerContent}
                        onChange={setBanner}
                        field="picPageType"
                        label="指示器样式"
                        options={[
                            { value: 1, label: "样式一" },
                            { value: 2, label: "样式二" },
                        ]}
                    />
                    <ColorField data={bannerContent} onChange={setBanner} field="swiperPageColor" label="指示器选中颜色" resetValue="" />
                    <ColorField data={bannerContent} onChange={setBanner} field="indicatorColor" label="指示器默认颜色" resetValue="" />
                    <DecDivider />
                    <RadioRowField
                        data={searchContent}
                        onChange={setSearch}
                        field="showSearch"
                        label="搜索框设置"
                        options={[
                            { value: 1, label: "显示" },
                            { value: 0, label: "不显示" },
                        ]}
                    />
                    {Number(searchContent?.showSearch) === 1 && (
                        <TextField data={searchContent} onChange={setSearch} field="searchText" label="默认搜索词" placeholder="请输入搜索框文字" />
                    )}
                </>
                <>
                    <RadioRowField
                        data={bannerStyle}
                        onChange={setStyle}
                        field="effectType"
                        label="搜索框/导航效果"
                        options={[
                            { value: "default", label: "正常模式" },
                            { value: "ceiling", label: "滚动至顶部固定" },
                        ]}
                    />
                    <DecDivider />
                    <ColorField data={bannerStyle} onChange={setStyle} field="searchBackgroundColor" label="背景颜色" resetValue="" />
                    <ColorField data={bannerStyle} onChange={setStyle} field="searchFontColor" label="文本颜色" resetValue="" />
                    <ColorField data={bannerStyle} onChange={setStyle} field="searchIconColor" label="搜索图标颜色" resetValue="" />
                    <ColorField data={bannerStyle} onChange={setStyle} field="areaTextColor" label="门店/定位文字颜色" resetValue="" />
                    <ColorField data={bannerStyle} onChange={setStyle} field="areaSwitchColor" label="切换文字颜色" resetValue="" />
                    <RangeField data={bannerStyle} onChange={setStyle} field="searchHeight" label="搜索框高度" max={40} />
                    <RangeField data={bannerStyle} onChange={setStyle} field="searchRadius" label="搜索框圆角" max={20} />
                </>
                <ModuleStyleSettingsField data={data} onChange={onChange} type="banner" />
            </SettingsTabs>
        </div>
    );
}
