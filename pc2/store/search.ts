/**
 * 搜索状态管理Store
 * @description 管理搜索相关的全局状态
 */
export const useSearchStore = defineStore("search", () => {
    /** 搜索结果总数 */
    const total = ref(0);
    /** 分类条当前展示的层级列表，用于跨路由 query 变化保持新版顶部分类条层级 */
    const barCategoryList = ref<any[]>([]);
    /** 分类条层级对应的上下文标识（pageType + keyword），上下文变化（如换了关键词）时才重置层级 */
    const barCategoryKey = ref("");
    /** 新版「全部筛选」浮层是否展开，用于跨路由变化保持浮层开启态，避免点选筛选项后浮层被关闭 */
    const filterPanelOpen = ref(false);

    return {
        total,
        barCategoryList,
        barCategoryKey,
        filterPanelOpen
    };
});
