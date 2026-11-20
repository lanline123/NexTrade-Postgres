/**
* 可视化热区绘制器（自 PC 编辑器 PcDecorateEditor.tsx 的 HotareaPainter 逐字移植，
* 2026-11-20；用户指令：H5 热图绘制向 PC 看齐）。
*
* - 在图片上直接**拖拽框选**新建热区；拖动热区内部移动位置，拖动四角控制点缩放；
* - 点击热区选中，下方出现属性条（链接 / 尺寸 / 删除），也可右键打开操作菜单；
* - 内部坐标按 500px 基准换算（HOTAREA_BASE），与 H5/PC 前台渲染及存量数据契约一致；
* - 样式类 hotarea-painter__* 在 pages/decorate-editor.css（PC 与 H5 编辑器共用同一份，
* decorateEditorH5/index.tsx 已 import），本文件不带样式。
*
* 待抽共享：PC 侧仍用其本地实现（PcDecorateEditor.tsx 体量过大，本轮不做整文件
* 抽取以免回归面失控）；后续应把 PC 改为 import 本组件、删本地副本，只留一份。
*
*/
import { useEffect, useRef, useState} from "react";
import { Button, InputNumber, Typography} from "@douyinfe/semi-ui";
import { IconDelete, IconPlus} from "@douyinfe/semi-icons";
import LinkPickerModal, { LinkField} from "../../../components/LinkPicker";
import type { PicItem} from "./settingWidgets";
import { useI18n} from "../../../i18n";

const { Text} = Typography;

/** 热区基准：热区坐标一律按 500px 宽的虚拟画布存储，渲染时按实际宽度等比缩放。 */
export const HOTAREA_BASE = 500;
/** 新建热区的默认尺寸（基准坐标）。 */
const HOTAREA_DEFAULT_SIZE = 100;
/** 新建/拖拽时允许的最小尺寸（基准坐标）。 */
const HOTAREA_MIN_SIZE = 20;

export type HotareaRect = { width?: number; height?: number; top?: number; left?: number; link?: Record<string, any>};

const clamp = (v: number, min: number, max: number) => Math.min(Math.max(v, min), Math.max(min, max));

export default function HotareaPainter({
picList,
onChange,
}: {
picList: Array<PicItem & { hotarea?: HotareaRect[]}>;
onChange: (next: Array<PicItem & { hotarea?: HotareaRect[]}>) => void;
}) {
const { t} = useI18n();
const [activePic, setActivePic] = useState(0);
const [selected, setSelected] = useState<number | null>(null);
const [drag, setDrag] = useState<{ kind: 'create' | 'move' | 'resize'; corner?: string; startX: number; startY: number; origin: HotareaRect} | null>(null);
const [menu, setMenu] = useState<{ index: number; x: number; y: number} | null>(null);
const [linkTarget, setLinkTarget] = useState<{ index: number; link: Record<string, any>} | null>(null);
const boardRef = useRef<HTMLDivElement | null>(null);

const pic = picList[activePic];
const hotareas: HotareaRect[] = (pic?.hotarea || []) as HotareaRect[];
const picUrl = pic? String(pic.picUrl || pic.picThumb || ''): '';
/**
* 画板实测尺寸。热区契约是「500px 宽基准画布」：纵轴与横轴共用同一刻度
* （前台 PC / 编辑器画布预览 / 模块预览三处都按 容器宽度/500 缩放）。
* 因此绘制器纵轴的换算与渲染都必须以画板**宽度**为参照，不能按画板高度归一化——
* 否则非正方形图片上画出的热区与存储参数、前台渲染全部错位。
*/
const [boardSize, setBoardSize] = useState({ w: 0, h: 0});
useEffect(() => {
const el = boardRef.current;
if (!el || typeof ResizeObserver === 'undefined') return undefined;
const observer = new ResizeObserver((entries) => {
const entry = entries[0];
if (entry) setBoardSize({ w: entry.contentRect.width, h: entry.contentRect.height});
});
observer.observe(el);
return () => observer.disconnect();
// eslint-disable-next-line react-hooks/exhaustive-deps
}, [activePic, picUrl, picList.length]);
/** 纵轴在基准坐标下的上限 = 500 × 画板高宽比（宽图 < 500，竖图 > 500）。 */
const baseHeight = boardSize.w? (boardSize.h / boardSize.w) * HOTAREA_BASE: HOTAREA_BASE;

const updatePic = (patch: Partial<{ hotarea: HotareaRect[]}>) => {
onChange(picList.map((item, i) => (i === activePic? {...item,...patch}: item)));
};
/**
* 热区写入统一走「按最新 picList 计算」的函数式更新：
* pointermove 高频触发时若直接闭包 `hotareas`，会读写到过期数组，导致拖拽位移被覆盖（表现为"拖不动"）。
*/
const mutatePic = (fn: (list: Array<PicItem & { hotarea?: HotareaRect[]}>, index: number) => Array<PicItem & { hotarea?: HotareaRect[]}>) => {
onChange(fn(picList, activePic));
};
const updateHotarea = (index: number, patch: Partial<HotareaRect>) => {
mutatePic((list, pi) =>
list.map((item, i) => {
if (i!== pi) return item;
const hs = (item.hotarea || []) as HotareaRect[];
return {...item, hotarea: hs.map((h, j) => (j === index? {...h,...patch}: h))};
}),
);
};
const removeHotarea = (index: number) => {
updatePic({ hotarea: hotareas.filter((_, i) => i!== index)});
setSelected(null);
setMenu(null);
};

  // 拖拽会话：统一在 window 上监听，鼠标移出画板也不断开
  // 所有坐标换算都实时读 boardRef 的 rect（不用 state），避免与 pointer 事件基准不一致
  useEffect(() => {
    if (!drag) return;
    const toBase = (clientX: number, clientY: number) => {
      const rect = boardRef.current?.getBoundingClientRect();
      if (!rect || !rect.width || !rect.height) return { x: 0, y: 0 };
      return {
        x: ((clientX - rect.left) / rect.width) * HOTAREA_BASE,
        // 纵轴同样按宽度归一（500px 宽基准画布），与存储契约/前台渲染一致
        y: ((clientY - rect.top) / rect.width) * HOTAREA_BASE,
      };
    };
    /** 纵轴上限（基准坐标），每次事件实时取画板矩形，避免用 state 的过期值。 */
    const maxYBase = () => {
      const rect = boardRef.current?.getBoundingClientRect();
      return rect && rect.width ? (rect.height / rect.width) * HOTAREA_BASE : HOTAREA_BASE;
    };
    const onMove = (ev: PointerEvent) => {
      const cur = toBase(ev.clientX, ev.clientY);
      const maxY = maxYBase();
      const o = drag.origin;
      if (drag.kind === 'create') {
        const x1 = clamp(Math.round(drag.startX), 0, HOTAREA_BASE);
        const y1 = clamp(Math.round(drag.startY), 0, maxY);
        const x2 = clamp(Math.round(cur.x), 0, HOTAREA_BASE);
        const y2 = clamp(Math.round(cur.y), 0, maxY);
        const next: HotareaRect = {
          left: Math.min(x1, x2),
          top: Math.min(y1, y2),
          width: Math.max(HOTAREA_MIN_SIZE, Math.abs(x2 - x1)),
          height: Math.max(HOTAREA_MIN_SIZE, Math.abs(y2 - y1)),
        };
        setDrag((d) => (d ? { ...d, origin: { ...d.origin, ...next } } : d));
        return;
      }
      const dx = cur.x - drag.startX;
      const dy = cur.y - drag.startY;
      if (drag.kind === 'move') {
        updateHotarea(selected ?? 0, {
          left: clamp(Math.round((o.left || 0) + dx), 0, HOTAREA_BASE - (o.width || 0)),
          top: clamp(Math.round((o.top || 0) + dy), 0, Math.max(0, maxY - (o.height || 0))),
        });
        return;
      }
      // resize：以起始矩形为基准，按拖动的角调整对应边
      let left = o.left || 0;
      let top = o.top || 0;
      let width = o.width || 0;
      let height = o.height || 0;
      const corner = drag.corner || 'se';
      if (corner.includes('w')) {
        const nl = clamp(Math.round(left + dx), 0, left + width - HOTAREA_MIN_SIZE);
        width = width + (left - nl);
        left = nl;
      }
      if (corner.includes('n')) {
        const nt = clamp(Math.round(top + dy), 0, top + height - HOTAREA_MIN_SIZE);
        height = height + (top - nt);
        top = nt;
      }
      if (corner.includes('e')) width = clamp(Math.round(width + dx), HOTAREA_MIN_SIZE, HOTAREA_BASE - left);
      if (corner.includes('s')) height = clamp(Math.round(height + dy), HOTAREA_MIN_SIZE, Math.max(HOTAREA_MIN_SIZE, maxY - top));
      updateHotarea(selected ?? 0, { left, top, width, height });
    };
    const onUp = () => {
      if (drag.kind === 'create') {
        const o = drag.origin;
        const w = o.width || 0;
        const h = o.height || 0;
        // 单击未拖动：落一个默认大小的热区，避免"点了没反应"
        const nextRect: HotareaRect =
          w < HOTAREA_MIN_SIZE + 2 || h < HOTAREA_MIN_SIZE + 2
            ? {
                left: clamp(Math.round(drag.startX), 0, HOTAREA_BASE - HOTAREA_DEFAULT_SIZE),
                top: clamp(Math.round(drag.startY), 0, Math.max(0, maxYBase() - HOTAREA_DEFAULT_SIZE)),
                width: HOTAREA_DEFAULT_SIZE,
                height: HOTAREA_DEFAULT_SIZE,
              }
            : { ...o };
        let addedIndex = 0;
        mutatePic((list, pi) => {
          addedIndex = ((list[pi]?.hotarea || []) as HotareaRect[]).length;
          return list.map((item, i) =>
            i === pi
              ? { ...item, hotarea: [...(((item.hotarea || []) as HotareaRect[])), { ...nextRect, link: { link: '', name: '' } }] }
              : item,
          );
        });
        setSelected(addedIndex);
      }
      setDrag(null);
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [drag, selected, hotareas, activePic]);

  if (!picList.length) {
    return <Text type="tertiary" size="small">{t('请先在上方添加热图')}</Text>;
  }

  const pct = (v?: number) => `${((Number(v) || 0) / HOTAREA_BASE) * 100}%`;
  /** 纵向渲染值：基准坐标 × 画板宽度/500（与横轴同刻度），不能按画板高度取百分比。 */
  const vy = (v?: number) =>
    boardSize.w ? `${((((Number(v) || 0) / HOTAREA_BASE) * boardSize.w)).toFixed(2)}px` : pct(v);
  const sel = selected !== null ? hotareas[selected] : null;

  return (
    <div className="hotarea-painter">
      {picList.length > 1 && (
        <div className="hotarea-painter__tabs">
          {picList.map((item, i) => (
            <button
              key={i}
              type="button"
              className={`hotarea-painter__tab${i === activePic ? ' is-active' : ''}`}
              onClick={() => { setActivePic(i); setSelected(null); setMenu(null); }}
            >
              {t('图 {index}', { index: i + 1 })}
              <em>{(item.hotarea || []).length}</em>
            </button>
          ))}
        </div>
      )}

      <div
        className={`hotarea-painter__board${drag?.kind === 'create' ? ' is-drawing' : ''}`}
        ref={boardRef}
        onPointerDown={(e) => {
          if (e.button !== 0) return;
          if ((e.target as HTMLElement).closest('.hotarea-painter__box')) return;
          setMenu(null);
          setSelected(null);
          const rect = boardRef.current?.getBoundingClientRect();
          if (!rect || !rect.width) return;
          const startX = ((e.clientX - rect.left) / rect.width) * HOTAREA_BASE;
          const startY = ((e.clientY - rect.top) / rect.width) * HOTAREA_BASE;
          setDrag({ kind: 'create', startX, startY, origin: {} });
        }}
        onPointerLeave={() => setMenu(null)}
      >
        {picUrl ? (
          <img src={picUrl} alt="" className="hotarea-painter__img" draggable={false} />
        ) : (
          <div className="hotarea-painter__placeholder">{t('该图暂无图片，请先在「热图」中上传')}</div>
        )}

        {hotareas.map((h, i) => {
          const rect = h;
          const isSel = selected === i;
          return (
            <div
              key={i}
              className={`hotarea-painter__box${isSel ? ' is-selected' : ''}`}
              style={{ left: pct(rect.left), top: vy(rect.top), width: pct(rect.width), height: vy(rect.height) }}
              onPointerDown={(e) => {
                if ((e.target as HTMLElement).dataset.corner) return;
                e.stopPropagation();
                if (e.button !== 0) return;
                setSelected(i);
                setMenu(null);
                const host = boardRef.current?.getBoundingClientRect();
                if (!host || !host.width || !host.height) return;
                setDrag({
                  kind: 'move',
                  startX: ((e.clientX - host.left) / host.width) * HOTAREA_BASE,
                  startY: ((e.clientY - host.top) / host.width) * HOTAREA_BASE,
                  origin: { ...rect },
                });
              }}
              onClick={(e) => e.stopPropagation()}
              onContextMenu={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setSelected(i);
                const host = boardRef.current?.getBoundingClientRect();
                setMenu({ index: i, x: e.clientX - (host?.left || 0), y: e.clientY - (host?.top || 0) });
              }}
            >
              <span className="hotarea-painter__label">{rect.link?.name || rect.link?.label || t('热区 {index}', { index: i + 1 })}</span>
              {isSel &&
                ['nw', 'ne', 'sw', 'se'].map((corner) => (
                  <i
                    key={corner}
                    data-corner={corner}
                    className={`hotarea-painter__handle hotarea-painter__handle--${corner}`}
                    onPointerDown={(e) => {
                      e.stopPropagation();
                      const host = boardRef.current?.getBoundingClientRect();
                      if (!host) return;
                      setDrag({
                        kind: 'resize',
                        corner,
                        startX: ((e.clientX - host.left) / host.width) * HOTAREA_BASE,
                        startY: ((e.clientY - host.top) / host.width) * HOTAREA_BASE,
                        origin: { ...rect },
                      });
                    }}
                  />
                ))}
            </div>
          );
        })}

        {drag?.kind === 'create' && (
          <div
            className="hotarea-painter__box is-draft"
            style={{ left: pct(drag.origin.left), top: vy(drag.origin.top), width: pct(drag.origin.width), height: vy(drag.origin.height) }}
          />
        )}

        {!hotareas.length && !drag && (
          <div className="hotarea-painter__hint">{t('在图片上按住并拖动即可绘制热区')}</div>
        )}

        {menu && (
          <div className="hotarea-painter__menu" style={{ left: menu.x, top: menu.y }} onPointerDown={(e) => e.stopPropagation()}>
            <button type="button" onClick={() => { setLinkTarget({ index: menu.index, link: hotareas[menu.index]?.link || {} }); setMenu(null); }}>
              {t('编辑链接')}
            </button>
            <button type="button" className="is-danger" onClick={() => removeHotarea(menu.index)}>
              {t('删除热区')}
            </button>
          </div>
        )}
      </div>

      <div className="hotarea-painter__toolbar">
        <span className="hotarea-painter__count">{t('共 {n} 个热区', { n: hotareas.length })}</span>
        <Button
          size="small"
          type="primary"
          theme="light"
          icon={<IconPlus />}
          onClick={() => {
            updatePic({
              hotarea: [
                ...hotareas,
                { left: 20, top: 20 + hotareas.length * 20, width: HOTAREA_DEFAULT_SIZE, height: HOTAREA_DEFAULT_SIZE, link: { link: '', name: '' } },
              ],
            });
            setSelected(hotareas.length);
          }}
        >
          {t('添加热区')}
        </Button>
        {hotareas.length > 0 && (
          <Button size="small" theme="borderless" type="danger" onClick={() => { updatePic({ hotarea: [] }); setSelected(null); }}>
            {t('清空热区')}
          </Button>
        )}
      </div>

      {sel && selected !== null && (
        <div className="hotarea-painter__inspector">
          <div className="hotarea-painter__inspector-head">
            <span>{t('热区 {index}', { index: selected + 1 })}</span>
            <Button size="small" theme="borderless" type="danger" icon={<IconDelete />} onClick={() => removeHotarea(selected)} />
          </div>
          <div className="hotarea-painter__fields">
            <label>
              <span>{t('宽')}</span>
              <InputNumber min={HOTAREA_MIN_SIZE} max={HOTAREA_BASE} value={sel.width} onChange={(v) => updateHotarea(selected, { width: Number(v) || HOTAREA_MIN_SIZE })} />
            </label>
            <label>
              <span>{t('高')}</span>
              <InputNumber min={HOTAREA_MIN_SIZE} max={HOTAREA_BASE} value={sel.height} onChange={(v) => updateHotarea(selected, { height: Number(v) || HOTAREA_MIN_SIZE })} />
            </label>
            <label>
              <span>{t('左')}</span>
              <InputNumber min={0} max={HOTAREA_BASE} value={sel.left} onChange={(v) => updateHotarea(selected, { left: Number(v) || 0 })} />
            </label>
            <label>
              <span>{t('上')}</span>
              <InputNumber min={0} max={Math.round(baseHeight)} value={sel.top} onChange={(v) => updateHotarea(selected, { top: Number(v) || 0 })} />
            </label>
          </div>
          <div className="hotarea-painter__link">
            <span>{t('跳转链接')}</span>
            <LinkField value={sel.link} onChange={(link) => updateHotarea(selected, { link })} />
          </div>
        </div>
      )}

      <LinkPickerModal
        visible={!!linkTarget}
        value={linkTarget?.link}
        onCancel={() => setLinkTarget(null)}
        onOk={(link) => {
          if (linkTarget) updateHotarea(linkTarget.index, { link });
          setLinkTarget(null);
        }}
      />
    </div>
  );
}
