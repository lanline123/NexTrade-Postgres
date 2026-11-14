/**
 * @tarojs/taro 测试桩：仅实现被测模块加载/执行期用到的 API。
 * request 由各测试通过 __setRequestImpl 注入行为（信封/错误/401 场景）。
 */
const storage = new Map<string, any>();

let requestImpl: ((options: any) => any) | null = null;

export const __setRequestImpl = (fn: ((options: any) => any) | null) => {
    requestImpl = fn;
};
export const __clearStorage = () => storage.clear();

const Taro = {
    getStorageSync: (key: string) => (storage.has(key) ? storage.get(key) : ""),
    setStorageSync: (key: string, value: any) => storage.set(key, value),
    removeStorageSync: (key: string) => storage.delete(key),
    getSystemInfoSync: () => ({
        windowWidth: 375,
        windowHeight: 667,
        statusBarHeight: 20,
        safeAreaInsets: { top: 20, bottom: 0, left: 0, right: 0 },
        safeArea: { top: 20, bottom: 667 }
    }),
    getMenuButtonBoundingClientRect: () => ({ top: 24, height: 32, width: 87, left: 278, right: 365, bottom: 56 }),
    request: (options: any) => {
        if (requestImpl) return requestImpl(options);
        options.success?.({ data: { code: 0, data: null }, statusCode: 200 });
        return { abort: () => {} };
    },
    uploadFile: () => ({ abort: () => {}, onProgressUpdate: () => {} }),
    showLoading: () => {},
    hideLoading: () => {},
    showToast: () => {},
    navigateTo: () => {},
    redirectTo: () => {},
    reLaunch: () => {},
    switchTab: () => {},
    navigateBack: () => {},
    onWindowResize: () => {},
    offWindowResize: () => {}
};

export default Taro;

// 页面级 hooks 桩（useList 等模块从 @tarojs/taro 具名导入）
export const useReachBottom = (_cb: () => void) => {};
export const usePageScroll = (_cb: (e: any) => void) => {};
export const useDidShow = (_cb: () => void) => {};
export const useDidHide = (_cb: () => void) => {};
export const getCurrentInstance = () => ({ router: { params: {} } });
