import fs from 'node:fs';
const file = 'src/decorate/components/modules.tsx';
let text = fs.readFileSync(file, 'utf8');

const anchor = `                    </div>
                  </div>
                ) : null}
              </div>
            ) : null}
            <div className={\`goods-ad-warp\${style === 6 ? ' has-nav' : ''}\`}`;

if (!text.includes(anchor)) {
  console.error('groupon-top 结尾锚点没找到');
  process.exit(1);
}

const replacement = `                    </div>
                  </div>
                ) : null}
                {/* 旧版 PcGrouponLegacy.vue:96-105：标题右侧“更多链接”，groupon-more-link */}
                {String(title.showMore) === '1' || title.showMore === true ? (
                  <a
                    className="groupon-more-link"
                    href={navHref(title.moreLink) || '/group/list'}
                    target="_blank"
                    rel="noreferrer"
                    style={{ color: title.moreColor || '#1a1a1a' }}
                  >
                    <span>{title.moreLink?.title || t('更多')}</span>
                    <i className="iconfont-pc icon-login-jt" />
                  </a>
                ) : null}
              </div>
            ) : null}
            <div className={\`goods-ad-warp\${style === 6 ? ' has-nav' : ''}\`}`;

text = text.replace(anchor, replacement);
fs.writeFileSync(file, text, 'utf8');
console.log('已插入 groupon-more-link');
