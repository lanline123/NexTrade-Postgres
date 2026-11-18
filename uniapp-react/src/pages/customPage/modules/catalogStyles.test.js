import { expect, it } from "vitest";

import userInfo from "../../member/components/decorate/userInfo/index.scss?inline";
import userInfo3 from "../../member/components/decorate/userInfo3/index.scss?inline";

it.each([false, true])("preserves each member header's nickname size (reverse=%s)", (reverse) => {
    // jsdom ignores rpx. Check the matching CSS rules directly, using the classes from the components.
    const style = document.createElement("style");
    style.textContent = reverse ? userInfo3 + userInfo : userInfo + userInfo3;
    document.head.appendChild(style);
    try {
        for (const [name, size] of [["dec-user-info", "28rpx"], ["dec-user-info-v3", "40rpx"]]) {
            const nicknameSelector = "." + name + "__name";
            const sizes = Array.from(style.sheet.cssRules)
                .filter(rule => rule.selectorText === nicknameSelector)
                .map(rule => rule.style.getPropertyValue("font-size"))
                .filter(Boolean);
            expect(sizes).toEqual([size]);
        }
    } finally {
        style.remove();
    }
});
