import { h, render } from "vue";
import SoldOut from "~/components/product/SoldOut.vue";
import { shouldShowProductStatus } from "~/utils/productSoldOut";

const getHostElement = (el: HTMLElement): HTMLElement | null => {
    if (el.tagName === "IMG") {
        return el.parentElement;
    }
    return el;
};

const mountSoldOut = (el: HTMLElement, binding: any) => {
    const value = binding.value || {};
    const { productStock, productStatus, productType, deliveryMode, deliveryType, stockField, statusField, uiVersion } = value;

    if (shouldShowProductStatus(value, { stockField, statusField })) {
        const host = getHostElement(el);
        if (!host) return;

        const computedStyle = window.getComputedStyle(host);
        if (computedStyle.position === "static") {
            host.style.position = "relative";
        }

        const container = document.createElement("div");
        container.style.position = "absolute";
        container.style.top = "0";
        container.style.left = "0";
        container.style.width = "100%";
        container.style.height = "100%";
        container.style.pointerEvents = "none";
        container.style.zIndex = "10";

        const vnode = h(SoldOut, {
            productStock,
            productStatus,
            productType,
            deliveryMode,
            deliveryType,
            stockField,
            statusField,
            uiVersion
        });

        host.appendChild(container);
        render(vnode, container);

        (el as any).__soldOutContainer__ = container;
        (el as any).__soldOutHost__ = host;
    }
};

const unmountSoldOut = (el: HTMLElement) => {
    const container = (el as any).__soldOutContainer__;
    if (container) {
        render(null, container);
        container.remove();
        delete (el as any).__soldOutContainer__;
        delete (el as any).__soldOutHost__;
    }
};

export default defineNuxtPlugin((nuxtApp) => {
    nuxtApp.vueApp.directive("sold-out", {
        mounted(el: HTMLElement, binding) {
            mountSoldOut(el, binding);
        },
        updated(el: HTMLElement, binding) {
            const oldValue = binding.oldValue || {};
            const newValue = binding.value || {};

            if (
                oldValue.productStock !== newValue.productStock ||
                oldValue.productStatus !== newValue.productStatus ||
                oldValue.productType !== newValue.productType ||
                oldValue.deliveryMode !== newValue.deliveryMode ||
                oldValue.deliveryType !== newValue.deliveryType ||
                oldValue.stockField !== newValue.stockField ||
                oldValue.statusField !== newValue.statusField ||
                oldValue.uiVersion !== newValue.uiVersion
            ) {
                unmountSoldOut(el);
                mountSoldOut(el, binding);
            }
        },
        unmounted(el: HTMLElement) {
            unmountSoldOut(el);
        }
    });
});

