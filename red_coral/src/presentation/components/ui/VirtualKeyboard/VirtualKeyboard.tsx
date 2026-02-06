import React, { useRef, useCallback, useEffect } from 'react';
import Keyboard from 'react-simple-keyboard';
import type { SimpleKeyboard } from 'react-simple-keyboard';
import 'react-simple-keyboard/build/css/index.css';
import spanishLayout from 'simple-keyboard-layouts/build/layouts/spanish';
import englishLayout from 'simple-keyboard-layouts/build/layouts/english';
import chineseLayout from 'simple-keyboard-layouts/build/layouts/chinese';
import { useVirtualKeyboardStore, useVirtualKeyboardVisible, useVirtualKeyboardLayout, useVirtualKeyboardLanguage } from '@/core/stores/ui/useVirtualKeyboardStore';
import type { KeyboardLanguage } from '@/core/stores/ui/useVirtualKeyboardStore';
import { handleKeyboardChange, getCurrentInputValue, scrollActiveElementIntoView } from './useKeyboardInput';
import { Z_INDEX } from '@/shared/constants/zIndex';

import { pinyinWords } from './pinyinWords';

// The chinese layout exports layoutCandidates at runtime but the TS type doesn't include it
const charCandidates = (chineseLayout as unknown as { layoutCandidates: Record<string, string> }).layoutCandidates;

/**
 * Expand word entries into all prefix entries and merge with single-char candidates.
 *
 * For word "nihao" → "你好", generates prefix entries:
 *   "nih"   → "你好"
 *   "niha"  → "你好"
 *   "nihao" → "你好"
 *
 * Prefixes that already have single-char candidates (like "ni") are skipped
 * so they don't pollute single-character selection.
 */
function buildCandidates(
  chars: Record<string, string>,
  words: Record<string, string>,
): Record<string, string> {
  const merged = { ...chars };

  // Collect all word candidates per prefix
  const prefixMap: Record<string, string[]> = {};
  for (const [pinyin, word] of Object.entries(words)) {
    for (let len = 2; len <= pinyin.length; len++) {
      const prefix = pinyin.substring(0, len);
      // Skip prefixes that have single-char candidates (e.g. "ni", "hao")
      if (chars[prefix]) continue;
      if (!prefixMap[prefix]) prefixMap[prefix] = [];
      if (!prefixMap[prefix].includes(word)) {
        prefixMap[prefix].push(word);
      }
    }
  }

  for (const [prefix, wordList] of Object.entries(prefixMap)) {
    merged[prefix] = wordList.join(' ');
  }

  return merged;
}

const chineseCandidates = buildCandidates(charCandidates, pinyinWords);

/** Number-only layout for numeric inputs */
const numberLayout = {
  default: [
    '1 2 3',
    '4 5 6',
    '7 8 9',
    '. 0 {bksp}',
  ],
};

const numberDisplay: Record<string, string> = {
  '{bksp}': '⌫',
};

/** Display labels for text layout (space is set dynamically per language) */
const textDisplay: Record<string, string> = {
  '{bksp}': '⌫',
  '{enter}': '↵',
  '{shift}': '⇧',
  '{lock}': '⇪',
  '{tab}': '⇥',
  '{symbols}': '?123',
  '{abc}': 'ABC',
  '{lang}': '🌐',
};

const spaceLabel: Record<KeyboardLanguage, string> = {
  spanish: 'español',
  english: 'English',
  chinese: '中文',
};

/** Symbols layout */
const symbolsLayout = {
  default: [
    '1 2 3 4 5 6 7 8 9 0',
    '@ # € % & - + ( )',
    '{abc} ! ? / \' " : ; {bksp}',
    '{lang} {abc} {space}',
  ],
};

/** Bottom row for text layouts */
const TEXT_BOTTOM_ROW = '{lang} {symbols} {space} @ .';

/** Replace the original bottom row (.com @ {space}) with our balanced layout */
function patchBottomRow(layout: Record<string, string[]>): Record<string, string[]> {
  const patched: Record<string, string[]> = {};
  for (const key of Object.keys(layout)) {
    const rows = [...layout[key]];
    rows[rows.length - 1] = TEXT_BOTTOM_ROW;
    patched[key] = rows;
  }
  return patched;
}

// Module-level cached patched layouts (computed once)
const patchedSpanish = patchBottomRow(spanishLayout.layout);
const patchedEnglish = patchBottomRow(englishLayout.layout);
const patchedChinese = patchBottomRow(chineseLayout.layout);

const layoutsByLanguage: Record<KeyboardLanguage, Record<string, string[]>> = {
  spanish: patchedSpanish,
  english: patchedEnglish,
  chinese: patchedChinese,
};

/** Set the --vkb-height CSS variable on :root */
function setVkbHeight(px: number) {
  document.documentElement.style.setProperty('--vkb-height', `${px}px`);
}

export const VirtualKeyboard: React.FC = () => {
  const visible = useVirtualKeyboardVisible();
  const layout = useVirtualKeyboardLayout();
  const language = useVirtualKeyboardLanguage();
  const activeElement = useVirtualKeyboardStore((s) => s.activeElement);
  const keyboardRef = useRef<SimpleKeyboard | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const layoutNameRef = useRef<string>('default');
  const [layoutName, setLayoutName] = React.useState('default');
  const [useSymbols, setUseSymbols] = React.useState(false);

  // Sync input value from activeElement to simple-keyboard
  useEffect(() => {
    if (visible && keyboardRef.current) {
      const val = getCurrentInputValue();
      keyboardRef.current.setInput(val);
    }
  }, [visible, activeElement]);

  // Set CSS variable + body class based on keyboard visibility
  useEffect(() => {
    if (!visible) {
      setVkbHeight(0);
      document.body.classList.remove('vkb-visible');
      return;
    }

    document.body.classList.add('vkb-visible');

    const updateHeight = () => {
      if (containerRef.current) {
        setVkbHeight(containerRef.current.offsetHeight);
      }
    };

    // Measure after first paint
    requestAnimationFrame(updateHeight);

    // Watch for size changes (layout switch, font load, etc.)
    const observer = new ResizeObserver(updateHeight);
    if (containerRef.current) {
      observer.observe(containerRef.current);
    }

    return () => {
      observer.disconnect();
      setVkbHeight(0);
      document.body.classList.remove('vkb-visible');
    };
  }, [visible]);

  // Scroll active element into view when keyboard appears or focus changes
  useEffect(() => {
    if (!visible || !activeElement) return;

    // Double rAF ensures CSS variable is applied before scroll calculation
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        scrollActiveElementIntoView();
      });
    });
  }, [visible, activeElement]);

  // Reset symbols state when layout or language changes
  useEffect(() => {
    setUseSymbols(false);
    setLayoutName('default');
    layoutNameRef.current = 'default';
    if (keyboardRef.current) {
      keyboardRef.current.setInput(getCurrentInputValue());
    }
  }, [layout, language]);

  // Sync activeElement value whenever it changes externally
  useEffect(() => {
    if (!visible) return;

    const el = useVirtualKeyboardStore.getState().activeElement;
    if (!el || !(el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement)) return;

    // Poll-sync: When React re-renders the input, sync back to keyboard
    const sync = () => {
      if (keyboardRef.current && el) {
        const current = keyboardRef.current.getInput();
        if (el.value !== current) {
          keyboardRef.current.setInput(el.value);
        }
      }
    };

    const observer = new MutationObserver(sync);
    observer.observe(el, { attributes: true, attributeFilter: ['value'] });

    // Also listen for external value changes
    el.addEventListener('change', sync);
    return () => {
      observer.disconnect();
      el.removeEventListener('change', sync);
    };
  }, [visible]);

  const onChange = useCallback((input: string) => {
    handleKeyboardChange(input);
  }, []);

  const onKeyPress = useCallback((button: string) => {
    if (button === '{lang}') {
      useVirtualKeyboardStore.getState().cycleLanguage();
      return;
    }
    if (button === '{shift}' || button === '{lock}') {
      const next = layoutNameRef.current === 'default' ? 'shift' : 'default';
      layoutNameRef.current = next;
      setLayoutName(next);
      return;
    }
    if (button === '{symbols}') {
      setUseSymbols(true);
      return;
    }
    if (button === '{abc}') {
      setUseSymbols(false);
      return;
    }
    if (button === '{enter}') {
      // Submit: blur the input (triggers form submission or closes keyboard)
      const el = useVirtualKeyboardStore.getState().activeElement;
      if (el instanceof HTMLInputElement) {
        // Dispatch Enter keypress for forms
        el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      }
      return;
    }

    // Auto-revert shift after typing a letter (single shift behavior)
    if (layoutNameRef.current === 'shift' && button.length === 1 && button !== button.toLowerCase()) {
      layoutNameRef.current = 'default';
      setLayoutName('default');
    }
  }, []);

  if (!visible) return null;

  const isNumber = layout === 'number';
  const isChinese = language === 'chinese';
  const currentLayout = isNumber
    ? numberLayout
    : useSymbols
      ? symbolsLayout
      : layoutsByLanguage[language];
  const currentDisplay = isNumber
    ? numberDisplay
    : { ...textDisplay, '{space}': spaceLabel[language] };

  return (
    <div
      ref={containerRef}
      className="fixed bottom-0 left-0 right-0 bg-gray-100 border-t border-gray-300 shadow-2xl"
      style={{ zIndex: Z_INDEX.VIRTUAL_KEYBOARD }}
      onPointerDown={(e) => {
        // Prevent the keyboard container from stealing focus
        e.preventDefault();
      }}
    >
      <Keyboard
        keyboardRef={(r) => { keyboardRef.current = r; }}
        layout={currentLayout}
        layoutName={layoutName}
        display={currentDisplay}
        onChange={onChange}
        onKeyPress={onKeyPress}
        preventMouseDownDefault
        preventMouseUpDefault
        physicalKeyboardHighlight={false}
        mergeDisplay
        theme={`hg-theme-default hg-layout-default vkb-theme ${isNumber ? 'vkb-number' : 'vkb-text'}`}
        {...(!isNumber && isChinese && !useSymbols ? {
          layoutCandidates: chineseCandidates,
          layoutCandidatesPageSize: 5,
        } : {})}
      />
    </div>
  );
};
