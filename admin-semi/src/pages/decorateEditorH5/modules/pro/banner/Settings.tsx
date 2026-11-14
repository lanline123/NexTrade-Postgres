/**
 * 轮播Banner 设置（字段对齐 uniapp-react banner 读取字段）
 */
import { SettingGroup, RadioRowField, NumberField, ColorField, PicListField, TextField, SwitchField, SinglePicField } from "../../../shared/settingWidgets";

export default function BannerSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const bc = data?.bannerContent ?? {}; const setBc = (p: Record<string, any>) => onChange({ bannerContent: { ...bc, ...p } });
    const bs = data?.bannerStyle ?? {}; const setBs = (p: Record<string, any>) => onChange({ bannerStyle: { ...bs, ...p } });
    const sc = data?.searchContent ?? {}; const setSc = (p: Record<string, any>) => onChange({ searchContent: { ...sc, ...p } });
    const nc = data?.navContent ?? {}; const setNc = (p: Record<string, any>) => onChange({ navContent: { ...nc, ...p } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="轮播内容">
                <PicListField data={bc} onChange={setBc} field="picList" label="轮播图片" max={10} />
                <NumberField data={bc} onChange={setBc} field="picRadius" label="图片圆角" max={40} suffix="px" />
                <NumberField data={bc} onChange={setBc} field="height" label="背景高度" max={400} suffix="px" />
                <RadioRowField data={bc} onChange={setBc} field="picPageType" label="指示器样式" options={[{ value: 1, label: "样式一" }, { value: 2, label: "样式二" }]} />
                <ColorField data={data} onChange={onChange} field="swiperPageColor" label="指示器选中颜色" />
            </SettingGroup>
            <SettingGroup title="搜索框">
                <SwitchField data={sc} onChange={setSc} field="showSearch" label="显示搜索框" />
                <TextField data={sc} onChange={setSc} field="searchText" label="搜索框文字" />
                <ColorField data={bs} onChange={setBs} field="searchBackgroundColor" label="搜索框背景色" />
                <ColorField data={bs} onChange={setBs} field="searchFontColor" label="搜索框文字色" />
                <NumberField data={bs} onChange={setBs} field="searchHeight" label="搜索框高度" min={24} max={60} suffix="px" />
                <NumberField data={bs} onChange={setBs} field="searchRadius" label="搜索框圆角" max={30} suffix="px" />
                <RadioRowField data={bs} onChange={setBs} field="effectType" label="滚动效果" options={[{ value: "default", label: "正常" }, { value: "ceiling", label: "吸顶" }]} />
            </SettingGroup>
            <SettingGroup title="热词导航">
                <SwitchField data={nc} onChange={setNc} field="showNav" label="显示热词" />
                <ColorField data={bs} onChange={setBs} field="navFontColor" label="热词文字色" />
                <ColorField data={bs} onChange={setBs} field="navBackgroundColor" label="热词背景色" />
                <SinglePicField data={data} onChange={onChange} field="logoPic" label="Logo 图片" />
                <NumberField data={data} onChange={onChange} field="logoHeight" label="Logo 高度" max={80} suffix="px" />
            </SettingGroup>
        </div>
    );
}
