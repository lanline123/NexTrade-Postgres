import { addServerPlugin, addTemplate, defineNuxtModule } from "@nuxt/kit";
import type { NuxtPage } from "@nuxt/schema";

interface RuntimeRoute {
    path: string;
    name?: string;
    alias?: string | string[];
    children?: RuntimeRoute[];
    component: Record<string, never>;
}

function isNotFoundPage(page: NuxtPage): boolean {
    return page.file?.replaceAll("\\", "/").endsWith("/pages/[...slug].vue") === true;
}

function toRuntimeRoutes(pages: NuxtPage[]): RuntimeRoute[] {
    return pages.flatMap((page) => {
        if (isNotFoundPage(page)) {
            return [];
        }

        const route: RuntimeRoute = {
            path: page.path,
            component: {}
        };

        if (page.name) route.name = page.name;
        if (page.alias) route.alias = page.alias;
        if (page.children?.length) route.children = toRuntimeRoutes(page.children);

        return [route];
    });
}

export default defineNuxtModule({
    meta: {
        name: "not-found-status"
    },
    setup(_options, nuxt) {
        // 静态生成阶段需要以 200 预渲染 /200.html、/404.html 和 /index.html；
        // 部署后的静态站点也不会运行 Nitro 插件，404 状态应由 Web 服务器负责。
        if (nuxt.options.ssr || nuxt.options._generate) return;

        let runtimeRoutes: RuntimeRoute[] = [];

        nuxt.hook("pages:resolved", (pages) => {
            runtimeRoutes = toRuntimeRoutes(pages);
        });

        const baseURL = nuxt.options.app.baseURL || "/";
        const plugin = addTemplate({
            filename: "not-found-status.mjs",
            write: true,
            getContents: () => `
import { getRequestURL } from "h3";
import { withoutBase } from "ufo";
import { createMemoryHistory, createRouter } from "vue-router";

const notFoundRouteName = Symbol("not-found-status");
const baseURL = ${JSON.stringify(baseURL)};
const router = createRouter({
    history: createMemoryHistory(),
    routes: [
        ...${JSON.stringify(runtimeRoutes)},
        { path: "/:pathMatch(.*)*", name: notFoundRouteName, component: {} }
    ]
});

export default function notFoundStatusPlugin(nitroApp) {
    nitroApp.hooks.hook("render:response", (response, { event }) => {
        if (event.method !== "GET" && event.method !== "HEAD") return;
        if ((response.statusCode ?? 200) !== 200) return;

        const pathname = withoutBase(getRequestURL(event).pathname, baseURL) || "/";
        const matched = router.resolve(pathname).matched.some((route) => route.name !== notFoundRouteName);
        if (!matched) {
            response.statusCode = 404;
            response.statusMessage = "Not Found";
        }
    });
}
`
        });

        addServerPlugin(plugin.dst);
    }
});
