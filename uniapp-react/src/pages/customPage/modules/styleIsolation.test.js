import { afterEach, describe, expect, it } from "vitest";

import banner from "./banner/index.scss?inline";
import categoryNav from "./categoryNavA1/index.scss?inline";
import notice from "./noticeA2/index.scss?inline";
import largeImage from "./slideLargeImage/index.scss?inline";
import slidingImage from "./slidingImage/index.scss?inline";
import specialOffer from "./specialOffer/index.scss?inline";

const sheets = [banner, categoryNav, notice, largeImage, slidingImage, specialOffer];

afterEach(() => {
    document.getElementById("module-styles")?.remove();
    document.body.replaceChildren();
});

describe.each([{ reverse: false }, { reverse: true }])("mixed decoration modules (reverse=$reverse)", ({ reverse }) => {
    it("keeps layout and marquee rules within their owning modules", () => {
        const style = document.createElement("style");
        style.id = "module-styles";
        style.textContent = (reverse ? [...sheets].reverse() : sheets).join("\n");
        document.head.appendChild(style);
        document.body.innerHTML = `
            <div class="decorate-banner module-ad-con"><div class="fixed"></div></div>
            <div class="decorate-category-nav-a1 module-ad-con"><div class="list"></div></div>
            <div class="decorate-sliding-image module-ad-con"><div class="item-box"></div></div>
            <div class="decorate-special-offer module-ad-con"><div class="item-box"></div></div>
            <div class="decorate-notice-a2 module-ad-con"><div class="headlines-text"></div></div>
            <div class="decorate-slide-large-image module-ad-con"><div class="slidelarge-image-box"></div></div>
            <aside><span class="fixed list item-box"></span></aside>`;
        const css = (selector) => getComputedStyle(document.querySelector(selector));

        expect(css(".decorate-banner .fixed").position).toBe("fixed");
        expect(css(".decorate-category-nav-a1 .list").display).toBe("flex");
        expect(css(".decorate-sliding-image .item-box").flexShrink).toBe("0");
        expect(css(".decorate-sliding-image .item-box").display).not.toBe("flex");
        expect(css(".decorate-special-offer .item-box").display).toBe("flex");
        expect(css(".decorate-special-offer .item-box").flexShrink).not.toBe("0");
        expect(css("aside span").position).not.toBe("fixed");
        expect(css("aside span").display).not.toBe("flex");

        const rules = Array.from(style.sheet.cssRules);
        for (const [selector, name, start, end] of [
            [".headlines-text", "decorate-notice-a2-scroll", "-100%", "-200%"],
            [".slidelarge-image-box", "decorate-slide-large-image-scroll", "-200%", "-300%"]
        ]) {
            expect(css(selector).animation).toContain(name);
            const frames = rules.find((rule) => rule.name === name);
            expect(Array.from(frames.cssRules, (frame) => frame.style.left)).toEqual([start, end]);
        }
    });
});
