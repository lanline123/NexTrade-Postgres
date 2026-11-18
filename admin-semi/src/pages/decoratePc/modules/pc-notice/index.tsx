import { containerFields, NOTICE_ICON, registerPcModule, type ModuleDef } from '../../contract/pcModuleContract';

const def: ModuleDef = {
    type: 'pc-notice', label: '公告', group: '营销活动',
    defaults: () => ({
      text: '公告内容',
      icoPic: { picUrl: NOTICE_ICON, picThumb: NOTICE_ICON },
      frame: { textColor: '#344054', fontSize: 14, iconSize: 20, itemBackgroundColor: '#f3f7ff', backgroundColor: '#ffffff', innerPadding: 0, itemHeight: 46, itemRadius: 12, boxRadius: 12, boxPadding: 0, boxPaddingTop: 0, boxPaddingBottom: 0 },
    }),
    settings: [
      { kind: 'textarea', key: 'text', label: '公告内容', placeholder: '请输入公告内容', rows: 3 },
      { kind: 'pic', key: 'icoPic', label: '公告图标', resetValue: { picUrl: NOTICE_ICON, picThumb: NOTICE_ICON } },
      { kind: 'slider', key: 'frame.iconSize', label: '公告图片大小', min: 12, max: 48, defaultValue: 16 },
      { kind: 'color', key: 'frame.textColor', label: '文字颜色', resetValue: '#555555', extend: false },
      { kind: 'slider', key: 'frame.fontSize', label: '文字大小', min: 12, max: 24, defaultValue: 14 },
      { kind: 'color', key: 'frame.itemBackgroundColor', label: '组件背景', resetValue: '#f3f7ff', extend: false },
      { kind: 'slider', key: 'frame.itemRadius', label: '组件圆角', max: 50, extend: false },
      { kind: 'slider', key: 'frame.itemHeight', label: '组件高度', min: 20, max: 80, extend: false },
      ...containerFields(),
    ],
    summary: (m) => String(m.text || '').slice(0, 20),
};

registerPcModule(def);
export default def;
