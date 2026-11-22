export default definePageConfig({
    navigationBarTitleText: "",
    // 页面已注册 usePullDownRefresh 且回调内有 stopPullDownRefresh 配对，此前无 config 文件导致小程序端刷新永不触发
    enablePullDownRefresh: true
});
