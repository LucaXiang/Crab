import React, { useRef, useCallback, useEffect } from 'react';
import Keyboard from 'react-simple-keyboard';
import type { SimpleKeyboard } from 'react-simple-keyboard';
import 'react-simple-keyboard/build/css/index.css';
import spanishLayout from 'simple-keyboard-layouts/build/layouts/spanish';
import { useVirtualKeyboardStore, useVirtualKeyboardVisible, useVirtualKeyboardLayout } from '@/core/stores/ui/useVirtualKeyboardStore';
import { handleKeyboardChange, getCurrentInputValue, scrollActiveElementIntoView } from './useKeyboardInput';
import { Z_INDEX } from '@/shared/constants/zIndex';

const KEYBOARD_HEIGHT = 280;

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

/** Display labels for Spanish text layout */
const textDisplay: Record<string, string> = {
  '{bksp}': '⌫',
  '{enter}': '↵',
  '{shift}': '⇧',
  '{lock}': '⇪',
  '{tab}': '⇥',
  '{space}': ' ',
  '{symbols}': '?123',
  '{abc}': 'ABC',
};

/** Symbols layout */
const symbolsLayout = {
  default: [
    '1 2 3 4 5 6 7 8 9 0',
    '@ # € % & - + ( )',
    '{abc} ! ? / \' " : ; {bksp}',
    '{space}',
  ],
};

export const VirtualKeyboard: React.FC = () => {
  const visible = useVirtualKeyboardVisible();
  const layout = useVirtualKeyboardLayout();
  const keyboardRef = useRef<SimpleKeyboard | null>(null);
  const layoutNameRef = useRef<string>('default');
  const [layoutName, setLayoutName] = React.useState('default');
  const [useSymbols, setUseSymbols] = React.useState(false);

  // Sync input value from activeElement to simple-keyboard
  useEffect(() => {
    if (visible && keyboardRef.current) {
      const val = getCurrentInputValue();
      keyboardRef.current.setInput(val);
      // Scroll into view when keyboard appears
      scrollActiveElementIntoView();
    }
  }, [visible]);

  // Set body padding when keyboard is visible
  useEffect(() => {
    if (visible) {
      document.body.style.paddingBottom = `${KEYBOARD_HEIGHT}px`;
      document.body.classList.add('vkb-visible');
    } else {
      document.body.style.paddingBottom = '';
      document.body.classList.remove('vkb-visible');
    }
    return () => {
      document.body.style.paddingBottom = '';
      document.body.classList.remove('vkb-visible');
    };
  }, [visible]);

  // Reset symbols state when layout changes
  useEffect(() => {
    setUseSymbols(false);
    setLayoutName('default');
    layoutNameRef.current = 'default';
  }, [layout]);

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
  const currentLayout = isNumber
    ? numberLayout
    : useSymbols
      ? symbolsLayout
      : spanishLayout.layout;
  const currentDisplay = isNumber
    ? numberDisplay
    : useSymbols
      ? { ...textDisplay }
      : { ...textDisplay };

  return (
    <div
      className="fixed bottom-0 left-0 right-0 bg-gray-100 border-t border-gray-300 shadow-2xl transition-transform duration-200 ease-out"
      style={{
        zIndex: Z_INDEX.VIRTUAL_KEYBOARD,
        height: KEYBOARD_HEIGHT,
        transform: visible ? 'translateY(0)' : 'translateY(100%)',
      }}
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
      />
    </div>
  );
};
