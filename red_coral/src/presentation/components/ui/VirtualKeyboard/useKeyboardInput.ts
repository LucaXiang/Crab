import { useVirtualKeyboardStore } from '@/core/stores/ui/useVirtualKeyboardStore';

const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
  HTMLInputElement.prototype, 'value'
)!.set!;

const nativeTextareaValueSetter = Object.getOwnPropertyDescriptor(
  HTMLTextAreaElement.prototype, 'value'
)!.set!;

/**
 * Writes a new value to the currently focused input element,
 * bypassing React's synthetic event system via the native setter.
 */
function writeToActiveElement(newValue: string) {
  const el = useVirtualKeyboardStore.getState().activeElement;
  if (!el) return;

  if (el instanceof HTMLInputElement) {
    nativeInputValueSetter.call(el, newValue);
  } else if (el instanceof HTMLTextAreaElement) {
    nativeTextareaValueSetter.call(el, newValue);
  } else {
    return;
  }

  el.dispatchEvent(new Event('input', { bubbles: true }));
}

export function getCurrentInputValue(): string {
  const el = useVirtualKeyboardStore.getState().activeElement;
  if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
    return el.value;
  }
  return '';
}

export function handleKeyboardChange(input: string) {
  writeToActiveElement(input);

  // Restore caret to end (simple-keyboard tracks caret internally)
  const el = useVirtualKeyboardStore.getState().activeElement;
  if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
    // Let the React re-render settle, then try to restore caret
    requestAnimationFrame(() => {
      try {
        // Place caret at the position simple-keyboard expects
        el.focus({ preventScroll: true });
      } catch { /* some input types don't support setSelectionRange */ }
    });
  }
}

export function scrollActiveElementIntoView() {
  const el = useVirtualKeyboardStore.getState().activeElement;
  if (el) {
    // 'nearest' respects scroll-padding-bottom on the scroll container,
    // ensuring the element is scrolled above the virtual keyboard area.
    el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }
}
