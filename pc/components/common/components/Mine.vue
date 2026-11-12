<template>
    <div class="category-wrap" :class="{ white: isWhite }">
        <el-popover
            v-if="userStore.isAuthenticated"
            v-model:visible="visible"
            trigger="hover"
            placement="bottom"
            :show-arrow="false"
            :teleported="false"
            :hide-after="300"
            :width="180"
            popper-class="popover-mine"
        >
            <template #reference>
                <div class="bar-wrap">
                    <div class="content">
                        <NuxtLink to="/member" class="content" @click="handleLink('/member')">
                            <div class="icon">
                                <MineIcon width="18" height="18"></MineIcon>
                            </div>
                            <div class="txt">{{ $t("我的") }}</div>
                            <div class="icon ml">
                                <ArrowIcon class="arrow" :class="{ rotate: visible }" width="14" height="14"></ArrowIcon>
                            </div>
                        </NuxtLink>
                    </div>
                </div>
            </template>
            <div class="popover-mine-content">
                <div>
                    <NuxtLink to="/member" class="member-info" @click="handleLink('/member')">
                        <div class="avatar">
                            <img
                                v-if="avatarSrc"
                                :key="avatarSrc"
                                :src="avatarSrc"
                                alt=""
                                referrerpolicy="no-referrer"
                                @error="onAvatarError"
                            />
                        </div>
                        <div class="name">
                            {{ userStore.userInfo.nickname ? userStore.userInfo.nickname : userStore.userInfo.dimUsername }}
                        </div>
                    </NuxtLink>
                </div>
                <div class="menu-wrap">
                    <NuxtLink to="/member/order/list" class="item-wrap">
                        {{ $t("我的订单") }}
                    </NuxtLink>
                    <!-- D1146：评价功能关闭时不展示「我的评价」入口 -->
                    <NuxtLink to="/member/comment/list" class="item-wrap" v-if="commonStore.productCommentEnabled === 1">
                        {{ $t("我的评价") }}
                    </NuxtLink>
                    <NuxtLink to="/member/collectProduct/list" class="item-wrap">
                        {{ $t("心愿单") }}
                    </NuxtLink>
                    <NuxtLink to="/member/address/list" class="item-wrap">
                        {{ $t("收货地址") }}
                    </NuxtLink>
                    <NuxtLink to="/member/browsingHistory/list" class="item-wrap">
                        {{ $t("浏览记录") }}
                    </NuxtLink>
                    <NuxtLink class="item-wrap logout" @click="onLogout">
                        {{ $t("退出登录") }}
                    </NuxtLink>
                </div>
            </div>
        </el-popover>
        <div v-else class="bar-wrap">
            <div class="content">
                <NuxtLink to="/member/login" class="content">
                    <div class="icon">
                        <MineIcon width="18" height="18"></MineIcon>
                    </div>
                    <div class="txt">{{ $t("登录") }}</div>
                </NuxtLink>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
// ===================== Vue 核心 =====================
import { ref, computed, watch } from "vue";

// ===================== 图标（内联 svg，SSR 友好；等价 vortmall 的 svg?component） =====================
import ArrowIcon from "~/components/icons/ArrowIcon.vue";
import MineIcon from "~/components/icons/MineIcon.vue";

// ===================== 工具函数 / Store =====================
import { imageFormat } from "~/utils/format";
import { useUserStore } from "~/store/user";
import { useCommonStore } from "~/store/common";

const commonStore = useCommonStore();

// ===================== Props & Emits =====================
defineProps({
    isWhite: {
        type: Boolean,
        default: false
    }
});

// ===================== 响应式状态 =====================
const router = useRouter();
const userStore = useUserStore();
const visible = ref<boolean>(false);

/** 头像地址：显式依赖 storageUrl，避免登录后先渲染空 src 裂图；无头像时回退默认头像 */
const avatarSrc = computed(() => {
    const storageUrl = commonStore.storageUrl;
    const raw = userStore.userInfo?.avatar || commonStore.defaultAvatar || "";
    if (!raw) return "";
    const url = imageFormat(raw);
    // storageUrl 未就绪时相对路径暂不展示，等配置到位后再拼完整地址
    if (url && !url.includes("http") && !storageUrl) return "";
    return url;
});

const onAvatarError = (e: Event) => {
    const img = e.target as HTMLImageElement | null;
    if (!img) return;
    const fallback = imageFormat(commonStore.defaultAvatar);
    if (!fallback || img.src === fallback) {
        img.onerror = null;
        img.removeAttribute("src");
        return;
    }
    img.onerror = null;
    img.src = fallback;
};

// ===================== 事件处理方法 =====================
const handleLink = (url: string): void => {
    if (url === router.currentRoute.value.path) {
        location.reload();
    }
};

const onLogout = (): void => {
    userStore.loginOut();
};

// ===================== 侦听器 =====================
watch(
    () => userStore.isAuthenticated,
    (authed) => {
        if (authed) {
            userStore.validateUserInfoOnce?.();
        }
    },
    { immediate: true }
);
</script>

<style lang="less" scoped>
.category-wrap {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    &.white {
        .bar-wrap {
            .content {
                &:hover {
                    .icon {
                        color: var(--primary-dark-2);
                    }
                    .txt {
                        color: var(--primary-dark-2);
                    }
                }
                .icon {
                    color: #1a1a1a;
                }
                .txt {
                    color: #1a1a1a;
                }
            }
        }
    }
    .bar-wrap {
        cursor: pointer;
        display: flex;
        align-items: center;
        height: 100%;
        .content {
            display: flex;
            height: 28px;
            &:hover {
                .icon {
                    color: #fffc;
                }
                .txt {
                    color: #fffc;
                }
            }
            .icon {
                display: flex;
                align-items: center;
                align-self: center;
                color: #fff;
                &.ml {
                    margin-left: 4px;
                }
                .arrow {
                    transition: all 0.2s;
                    &.rotate {
                        transform: rotate(180deg);
                    }
                }
            }
            .txt {
                align-self: center;
                margin-left: 6px;
                font-size: 14px;
                color: #fff;
                font-weight: 500;
            }
        }
    }
}
.popover-mine-content {
    z-index: 9999;
    .member-info {
        display: flex;
        align-items: center;
        .avatar {
            width: 28px;
            height: 28px;
            img {
                width: 100%;
                height: 100%;
                object-fit: contain;
                border-radius: 50%;
            }
        }
        .name {
            margin-left: 6px;
            width: 100px;
            overflow: hidden;
            white-space: nowrap;
            text-overflow: ellipsis;
            text-transform: none;
        }
    }
    .menu-wrap {
        margin-top: 10px;
        .item-wrap {
            display: flex;
            align-items: center;
            height: 40px;
            color: #1a1a1a;
            text-transform: capitalize;
            &.logout {
                color: #888b94;
            }
        }
    }
}
</style>
<style lang="less">
.popover-mine.el-popover.el-popper {
    width: 180px !important;
    min-width: 180px !important;
    padding: 12px 16px;
}
</style>
