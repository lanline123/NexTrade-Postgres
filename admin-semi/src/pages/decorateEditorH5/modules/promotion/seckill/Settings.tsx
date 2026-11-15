/**
 * 秒杀 设置（对齐 view/admin seckill/Settings.vue）
 *
 * 原版仅两组：CommonActivityFrameEdit（整模块）+ CommonTitleEdit（module.title）——
 * 分别以 shared/ActivityFrameFields 与 TitleSettingsField 承接，无页签。
 */
import { TitleSettingsField } from "../../../shared/settingWidgets";
import ActivityFrameFields from "../../../shared/ActivityFrameFields";

export default function SeckillSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <ActivityFrameFields data={data} onChange={onChange} />
            <TitleSettingsField data={data} onChange={onChange} field="title" />
        </div>
    );
}
