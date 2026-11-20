/**
 * 使用静态资源域名补全相对图片地址。
 */
export const resolveImageUrl = (imagePath: string, storageUrl = "") => {
    // 非字符串入参（老数据 picUrl 被写成对象/数字等）直接归空：此前裸调
    // .includes 会抛 TypeError、被模块错误边界降级为「渲染失败」占位
    if (!imagePath || typeof imagePath !== "string") {
        return "";
    }
    return imagePath.includes("http") ? imagePath : storageUrl + imagePath;
};
