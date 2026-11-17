/** pc-home-hero 模块定义 —— 逐字摘自 PcDecorateEditor.tsx 的 NEW_MODULE_DEFS（PcDecorateEditor.tsx 第 1010–1080 行），Step 3 模块化拆分（P4 营销组）。 */
import type { ModuleDef, PicItem } from '../../contract/pcModuleContract';
import { registerPcModule, HERO_CATEGORIES, DEMO_PIC_1, DEMO_PIC_2, demoPic, heroDemoModules, containerFields } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-home-hero', label: '营销通栏', group: '营销活动',
  desc: '左侧分类导航 / 中间焦点区 / 右侧用户区',
  defaults: () => ({
    frame: { boxPadding: 0, boxPaddingTop: 0, boxPaddingBottom: 0, backgroundColor: '' },
    left: {
      title: '全部商品分类',
      showCount: 11,
      categories: HERO_CATEGORIES,
      backgroundColor: '#ffffff',
      textColor: '#333333',
    },
    center: {
      banners: [demoPic('1', DEMO_PIC_1, '演示 Banner 1', '限时直降'), demoPic('2', DEMO_PIC_2, '演示 Banner 2', '新品上新')],
      modules: heroDemoModules(),
    },
    right: {
      user: { avatar: { picUrl: 'https://oss.vortmall.com/upload/c4ca4238/2026/02/09/9be6e26cd04b449a8e843df6f790d601.png', picThumb: 'https://oss.vortmall.com/upload/c4ca4238/2026/02/09/9be6e26cd04b449a8e843df6f790d601.png', picName: 'avatar' }, nickname: '欢迎来商城', welcomeText: '登录后更多精彩' },
      userCardBackground: '#ffffff',
      userCardGradientColorA: '#FFEDE1',
      userCardGradientColorB: '#FCF9F8',
      userCardGradientType: 'upDown',
      premiumMember: { show: false, title: '会员', description: '', link: { path: 'base', label: '权益卡中心', name: '权益卡中心', link: 'member/premiumMember/index' } },
      stats: [
        { label: '优惠券', value: '85' },
        { label: '余额', value: '257.58' },
        { label: '积分', value: '544' },
      ],
      shortcuts: [1, 2, 3, 4].map((n) => ({ picId: `s${n}`, picThumb: '', picUrl: '', picName: `shop_${n}`, picTitle: '进入店铺', picLink: { link: '', name: '' } })),
      shopsTitle: '店铺街',
      shopsSubTitle: 'HOT',
      shopsSubTitleBg: '#FF4E4E',
      shopsMoreText: '查看全部',
      shopsMoreLink: { link: '', name: '' },
      shopsTextColor: '#a06a38',
    },
    style: {
      height: 336, gap: 16, leftWidth: 220, rightWidth: 260,
      panelBackgroundColor: '#ffffff', panelRadius: 8, panelShadow: 1,
      titleFontSize: 14, titleColor: '#333333', mutedColor: '#999999',
      priceColor: '#e1251b', bannerRadius: 12, cardRadius: 12,
      innerPadding: 0,
    },
  }),
  settings: [
    { kind: 'color', key: 'left.backgroundColor', label: '背景颜色', resetValue: '#ffffff', section: '左侧分类导航' },
    { kind: 'color', key: 'left.textColor', label: '文字颜色', resetValue: '#333333' },
    { kind: 'picList', key: 'center.banners', label: '中间轮播图', sections: ['中间焦点区', '轮播广告'], picAddText: '添加轮播' },
    { kind: 'heroCards', key: 'center.modules', label: '频道卡位(6个)', section: '推荐卡位' },
    { kind: 'switch', key: 'right.premiumMember.show', label: '付费会员', section: '右侧会员卡' },
    { kind: 'input', key: 'right.premiumMember.title', label: '标题', placeholder: '会员', showIf: (m) => m.right?.premiumMember?.show === true },
    { kind: 'input', key: 'right.premiumMember.description', label: '描述', placeholder: '请输入描述', showIf: (m) => m.right?.premiumMember?.show === true },
    { kind: 'link', key: 'right.premiumMember.link', label: '跳转链接', showIf: (m) => m.right?.premiumMember?.show === true },
    { kind: 'input', key: 'right.user.nickname', label: '未登录标题文字', placeholder: '如：欢迎来商城' },
    { kind: 'colorPair', key: 'right.userCardGradientColorA', secondKey: 'right.userCardGradientColorB', label: '背景渐变色', resetValue: '#FFEDE1', resetValueSecond: '#FCF9F8' },
    { kind: 'radio', key: 'right.userCardGradientType', label: '渐变方向', options: [{ label: '上下', value: 'upDown' }, { label: '左右', value: 'leftRight' }, { label: '斜向', value: 'diagonal' }] },
    { kind: 'input', key: 'right.shopsTitle', label: '区块标题', placeholder: '请输入标题', section: '快捷入口区' },
    { kind: 'input', key: 'right.shopsSubTitle', label: '角标文字', placeholder: '如：HOT、推荐' },
    { kind: 'color', key: 'right.shopsSubTitleBg', label: '角标背景色', resetValue: '#FF4E4E' },
    { kind: 'input', key: 'right.shopsMoreText', label: '查看更多文字', placeholder: '如：查看全部、更多' },
    { kind: 'link', key: 'right.shopsMoreLink', label: '查看更多链接' },
    { kind: 'color', key: 'right.shopsTextColor', label: '入口文字颜色', resetValue: '#a06a38' },
    { kind: 'picList', key: 'right.shortcuts', label: '入口列表', section: '入口列表', picMax: 4, picAddText: '添加入口' },
    { kind: 'slider', key: 'style.panelRadius', label: '圆角', max: 30, section: '整体样式' },
    { kind: 'color', key: 'style.panelBackgroundColor', label: '背景色', resetValue: '#ffffff' },
    { kind: 'slider', key: 'style.gap', label: '内间距', max: 30 },
    // 原版「外边距」= ModuleContainerSettings（frame.* 自动归入扩展设置）
    ...containerFields(),
  ],
  summary: (m) => `三列通栏 · ${((m.center?.banners || []) as PicItem[]).length} 轮播`,
};

registerPcModule(def);

export default def;
