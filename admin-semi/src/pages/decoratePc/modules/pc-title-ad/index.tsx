import { containerFields, getPath, registerPcModule, standardFrame, standardTitle, type ModuleDef } from '../../contract/pcModuleContract';

const def: ModuleDef = {
    type: 'pc-title-ad', label: '文本标题', group: '辅助元素',
    defaults: () => ({
      // 标题即本体：默认走共享标题契约（standardTitle），与其它模块同一套默认值
      title: standardTitle({ showTitle: 1, titleText: '标题内容', titleColor: '#2a3145' }),
      frame: standardFrame(),
    }),
    settings: [
      // 与其它模块一致的通用「标题设置」组（kind:'title'）：样式1-3/对齐/字号/描述
      // 及描述排版/背景/圆角/四边距/更多链接。此前这里是手写子集（只有风格一二、
      // 无边距与描述排版、字号上限不同），与通用标题设置脱节。
      { kind: 'title', key: 'title', label: '标题设置', titleHideMoreType: true },
      ...containerFields({ showInnerPadding: true }),
    ],
    summary: (m) => String(getPath(m, 'title.titleText') || ''),
};

registerPcModule(def);
export default def;
