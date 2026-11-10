import type { ConfigAction } from '../pages/AutoConfigPage';

/**
 * 旧版配置页里的「按钮型功能」清单。
 *
 * 背景：旧 Vue 配置页在表单里直接放按钮（生成密钥、批量更新、测试邮件等），
 * 迁移到 AutoConfigPage 通用表单引擎后这些按钮一度无处承载，造成「字段在、功能不在」。
 * 这里按旧版契约逐项补回；接口路径与 Java 控制器保持一致。
 *
 * 注意：部分能力在 Rust 后端仍是显式未迁移（返回业务码 1001）。
 * 前端不做「假成功」，由 AutoConfigPage 把后端返回的真实错误展示给用户。
 */

export const configActions: Record<string, ConfigAction[]> = {
  globalSetting: [
    {
      label: '同步到搜索引擎',
      section: '搜索设置',
      confirm: '将一次性把所有商品同步到 Elasticsearch，异步执行期间请勿重复点击，确认继续？',
      success: '同步任务已提交',
      visibleWhen: (values) => values.selectSearchType === 'es' || values.selectSearchType === 1,
      run: async ({ apiPost }) => {
        await apiPost('/product/product/initAllToEs');
      },
    },
    {
      label: '批量更新商品类目链接',
      section: '链接设置',
      confirm: '仅更新 SEO 链接为空的商品类目，请勿刷新页面，确认更新？',
      success: '批量更新任务已提交',
      run: async ({ apiPost }) => {
        await apiPost('/setting/seo/batchUpdateCategoryLink');
      },
    },
    {
      label: '批量更新商品链接',
      section: '链接设置',
      confirm: '仅更新 SEO 链接为空的商品，请勿刷新页面，确认更新？',
      success: '批量更新任务已提交',
      run: async ({ apiPost }) => {
        await apiPost('/setting/seo/batchUpdateProductLink');
      },
    },
  ],

};
