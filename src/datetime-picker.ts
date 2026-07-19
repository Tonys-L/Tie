/**
 * 日期时间分段选择器
 *
 * 单个输入框展示 `yyyy-MM-dd HH:mm` 格式，年/月/日/时/分 5 段。
 * 点击某段高亮选中，滚轮或上下箭头调整当前段数值（自动 clamp），
 * 左右箭头切换段，数字键直接输入到当前段。不自动跳段，不需失焦校验。
 *
 * 实现思路：
 * - 用 contenteditable=false 的容器，每段是一个 <span contenteditable=false>
 * - 高亮段加 .active 类
 * - 监听容器的 click/wheel/keydown 事件
 * - 段值变更后重新渲染文本
 */

export interface DateTimeSegmentPickerOptions {
  /** 初始值 */
  initialValue?: Date;
  /** 值变化回调 */
  onChange?: (date: Date) => void;
}

const SEGMENT_DEFS = [
  { key: 'year', min: 1970, max: 2099, pad: 4 },
  { key: 'month', min: 1, max: 12, pad: 2 },
  { key: 'day', min: 1, max: 31, pad: 2 },
  { key: 'hour', min: 0, max: 23, pad: 2 },
  { key: 'minute', min: 0, max: 59, pad: 2 },
] as const;

type SegmentKey = typeof SEGMENT_DEFS[number]['key'];

export class DateTimeSegmentPicker {
  private container: HTMLElement;
  private options: DateTimeSegmentPickerOptions;
  private segmentEls: Record<SegmentKey, HTMLElement> = {} as any;
  private activeSegment: SegmentKey = 'year';
  private value: Date;
  private inputBuffer = '';
  private inputBufferTimer: number | null = null;

  constructor(container: HTMLElement, options: DateTimeSegmentPickerOptions = {}) {
    this.container = container;
    this.options = options;
    this.value = options.initialValue ?? new Date();
    this.render();
    this.setActiveSegment('year');
  }

  private render() {
    this.container.classList.add('dts-root');
    this.container.innerHTML = '';
    this.container.setAttribute('tabindex', '0');

    const segDefs = SEGMENT_DEFS;
    const separators = ['-', '-', ' ', ':'];

    segDefs.forEach((def, idx) => {
      const span = document.createElement('span');
      span.className = 'dts-seg';
      span.dataset.seg = def.key;
      span.textContent = this.formatSegment(def.key);
      this.container.appendChild(span);
      this.segmentEls[def.key] = span;

      if (idx < segDefs.length - 1) {
        const sep = document.createElement('span');
        sep.className = 'dts-sep';
        sep.textContent = separators[idx];
        this.container.appendChild(sep);
      }
    });

    this.bindEvents();
  }

  private formatSegment(key: SegmentKey): string {
    const def = SEGMENT_DEFS.find(d => d.key === key)!;
    let v: number;
    switch (key) {
      case 'year': v = this.value.getFullYear(); break;
      case 'month': v = this.value.getMonth() + 1; break;
      case 'day': v = this.value.getDate(); break;
      case 'hour': v = this.value.getHours(); break;
      case 'minute': v = this.value.getMinutes(); break;
    }
    return String(v).padStart(def.pad, '0');
  }

  private getSegmentMax(key: SegmentKey): number {
    if (key === 'day') {
      // 根据当前年月计算天数上限
      const year = this.value.getFullYear();
      const month = this.value.getMonth() + 1;
      return new Date(year, month, 0).getDate();
    }
    return SEGMENT_DEFS.find(d => d.key === key)!.max;
  }

  private getSegmentMin(key: SegmentKey): number {
    return SEGMENT_DEFS.find(d => d.key === key)!.min;
  }

  private getSegmentValue(key: SegmentKey): number {
    switch (key) {
      case 'year': return this.value.getFullYear();
      case 'month': return this.value.getMonth() + 1;
      case 'day': return this.value.getDate();
      case 'hour': return this.value.getHours();
      case 'minute': return this.value.getMinutes();
    }
  }

  private setSegmentValue(key: SegmentKey, v: number) {
    const min = this.getSegmentMin(key);
    const max = this.getSegmentMax(key);
    const clamped = Math.max(min, Math.min(max, v));
    switch (key) {
      case 'year': this.value.setFullYear(clamped); break;
      case 'month': this.value.setMonth(clamped - 1); break;
      case 'day': this.value.setDate(clamped); break;
      case 'hour': this.value.setHours(clamped); break;
      case 'minute': this.value.setMinutes(clamped); break;
    }
    // 修改年月后，日可能溢出（如 1月31日 → 2月，日变 3月3日），重新 clamp
    if (key === 'year' || key === 'month') {
      const dayMax = this.getSegmentMax('day');
      if (this.value.getDate() > dayMax) {
        this.value.setDate(dayMax);
      }
    }
    this.rerenderSegments();
    this.options.onChange?.(new Date(this.value));
  }

  private rerenderSegments() {
    (Object.keys(this.segmentEls) as SegmentKey[]).forEach(key => {
      this.segmentEls[key].textContent = this.formatSegment(key);
    });
  }

  private setActiveSegment(key: SegmentKey) {
    this.activeSegment = key;
    (Object.keys(this.segmentEls) as SegmentKey[]).forEach(k => {
      this.segmentEls[k].classList.toggle('active', k === key);
    });
    this.container.focus();
  }

  private nextSegment() {
    const idx = SEGMENT_DEFS.findIndex(d => d.key === this.activeSegment);
    const nextIdx = (idx + 1) % SEGMENT_DEFS.length;
    this.setActiveSegment(SEGMENT_DEFS[nextIdx].key);
  }

  private prevSegment() {
    const idx = SEGMENT_DEFS.findIndex(d => d.key === this.activeSegment);
    const prevIdx = (idx - 1 + SEGMENT_DEFS.length) % SEGMENT_DEFS.length;
    this.setActiveSegment(SEGMENT_DEFS[prevIdx].key);
  }

  private adjustCurrent(delta: number) {
    const cur = this.getSegmentValue(this.activeSegment);
    this.setSegmentValue(this.activeSegment, cur + delta);
  }

  private handleDigitInput(digit: number) {
    // 数字键输入：累积到 inputBuffer，超过段位数则重新开始
    const def = SEGMENT_DEFS.find(d => d.key === this.activeSegment)!;
    if (this.inputBufferTimer) clearTimeout(this.inputBufferTimer);
    // 若 buffer 长度已达段位数，重新开始
    if (this.inputBuffer.length >= def.pad) {
      this.inputBuffer = '';
    }
    this.inputBuffer += String(digit);
    const parsed = parseInt(this.inputBuffer, 10);
    // 若输入值超过段上限，重新以当前数字开始
    if (parsed > this.getSegmentMax(this.activeSegment) && this.inputBuffer.length < def.pad) {
      this.inputBuffer = String(digit);
    }
    const newVal = parseInt(this.inputBuffer, 10);
    if (!isNaN(newVal)) {
      this.setSegmentValue(this.activeSegment, newVal);
    }
    // 1.5 秒无输入清空 buffer
    this.inputBufferTimer = window.setTimeout(() => {
      this.inputBuffer = '';
    }, 1500);
  }

  private bindEvents() {
    // 点击段：选中对应段
    (Object.keys(this.segmentEls) as SegmentKey[]).forEach(key => {
      const el = this.segmentEls[key];
      el.addEventListener('mousedown', (e) => {
        e.preventDefault();
        e.stopPropagation();
        this.setActiveSegment(key);
      });
    });

    // 点击容器空白处：选中第一段
    this.container.addEventListener('mousedown', (e) => {
      if (e.target === this.container) {
        e.preventDefault();
        this.setActiveSegment('year');
      }
    });

    // 滚轮调整当前段
    this.container.addEventListener('wheel', (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.container.focus();
      const delta = e.deltaY > 0 ? -1 : 1;
      this.adjustCurrent(delta);
    }, { passive: false });

    // 键盘
    this.container.addEventListener('keydown', (e) => {
      switch (e.key) {
        case 'ArrowLeft':
          e.preventDefault();
          this.prevSegment();
          break;
        case 'ArrowRight':
          e.preventDefault();
          this.nextSegment();
          break;
        case 'ArrowUp':
          e.preventDefault();
          this.adjustCurrent(1);
          break;
        case 'ArrowDown':
          e.preventDefault();
          this.adjustCurrent(-1);
          break;
        case '0': case '1': case '2': case '3': case '4':
        case '5': case '6': case '7': case '8': case '9':
          e.preventDefault();
          this.handleDigitInput(parseInt(e.key, 10));
          break;
        case 'Tab':
          e.preventDefault();
          if (e.shiftKey) this.prevSegment();
          else this.nextSegment();
          break;
        default:
          // 阻止其他字符输入
          if (e.key.length === 1) e.preventDefault();
          break;
      }
    });

    // 失焦时清除高亮
    this.container.addEventListener('blur', (e) => {
      // 仅在焦点离开容器整体时才清除
      const related = e.relatedTarget as Node | null;
      if (!this.container.contains(related)) {
        (Object.keys(this.segmentEls) as SegmentKey[]).forEach(k => {
          this.segmentEls[k].classList.remove('active');
        });
      }
    });

    // 阻止默认粘贴等行为
    this.container.addEventListener('paste', (e) => e.preventDefault());
    this.container.addEventListener('copy', (e) => e.preventDefault());
  }

  /** 外部设置值 */
  setValue(date: Date) {
    this.value = new Date(date);
    this.rerenderSegments();
    this.options.onChange?.(new Date(this.value));
  }

  /** 获取当前值 */
  getValue(): Date {
    return new Date(this.value);
  }

  destroy() {
    if (this.inputBufferTimer) clearTimeout(this.inputBufferTimer);
    this.container.innerHTML = '';
    this.container.classList.remove('dts-root');
    this.container.removeAttribute('tabindex');
  }
}
