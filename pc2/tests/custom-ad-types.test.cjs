const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const ts = require('typescript');
const vue = require('vue');
const { renderToString } = require('@vue/server-renderer');
const { parse, compileScript } = require('@vue/compiler-sfc');

const filename = path.resolve(__dirname, '../components/home/CustomAd2.vue');
const { descriptor, errors } = parse(fs.readFileSync(filename, 'utf8'), { filename });
assert.deepEqual(errors, []);
const script = compileScript(descriptor, { id: 'fixture-ad', inlineTemplate: true });
const emitted = ts.transpileModule(script.content, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText;
const fixtureModule = { exports: {} };
vm.runInNewContext(emitted, { module: fixtureModule, exports: fixtureModule.exports, require(name) {
    assert.equal(name, 'vue');
    return vue;
} });

async function render(picList) {
    const app = vue.createSSRApp(fixtureModule.exports.default, { modelValue: { picList, picList2: [], picList3: [], linkList: [] } });
    app.component('NuxtLink', { props: ['to'], setup: (props, { slots }) => () => vue.h('a', { href: props.to }, slots.default?.()) });
    app.component('el-image', { props: ['src'], setup: (props) => () => vue.h('img', { src: props.src }) });
    Object.assign(app.config.globalProperties, { $t: value => value, imageFormat: value => value, urlFormat: value => value?.path || '' });
    return renderToString(app);
}

test('empty editor picture placeholder produces no broken main image', async () => {
    const html = await render([]);
    assert.doesNotMatch(html, /<img\b/);
});

test('an existing main picture keeps its original URL and link', async () => {
    const html = await render({ picUrl: '/fixture-main.png', picTitle: 'fixture', picLink: { data: { path: '/fixture-product' } } });
    assert.match(html, /src="\/fixture-main.png"/);
    assert.match(html, /href="\/fixture-product"/);
});
