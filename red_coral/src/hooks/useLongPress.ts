import React, { useCallback, useRef, useState } from 'react';

interface LongPressOptions {
  isPreventDefault?: boolean;
  delay?: number;
}

export const useLongPress = (
  onLongPress: (e: React.MouseEvent | React.TouchEvent) => void,
  onClick: (e: React.MouseEvent | React.TouchEvent) => void,
  { isPreventDefault = true, delay = 500 }: LongPressOptions = {}
) => {
  const [longPressTriggered, setLongPressTriggered] = useState(false);
  const timeout = useRef<NodeJS.Timeout>(undefined);
  const target = useRef<EventTarget>(undefined);
  const startPos = useRef<{ x: number; y: number } | null>(null);
  const isStarted = useRef(false);
  const isMoved = useRef(false);
  const lastTouchTime = useRef<number>(0);

  const start = useCallback(
    (e: React.MouseEvent | React.TouchEvent) => {
      // Prevent mouse events that occur immediately after touch events (ghost clicks)
      if ('button' in e && Date.now() - lastTouchTime.current < 1000) {
        return;
      }

      isStarted.current = true;
      isMoved.current = false;
      if (isPreventDefault && e.target) {
        target.current = e.target;
      }
      
      // Record start position for touch events to allow small movements
      if ('touches' in e) {
        lastTouchTime.current = Date.now();
        startPos.current = { x: e.touches[0].clientX, y: e.touches[0].clientY };
      } else {
        startPos.current = null;
      }

      setLongPressTriggered(false);
      timeout.current = setTimeout(() => {
        onLongPress(e);
        setLongPressTriggered(true);
      }, delay);
    },
    [onLongPress, delay, isPreventDefault]
  );

  const clear = useCallback(
    (e: React.MouseEvent | React.TouchEvent, shouldTriggerClick = true) => {
      timeout.current && clearTimeout(timeout.current);
      
      const wasStarted = isStarted.current;
      const wasMoved = isMoved.current;
      isStarted.current = false;
      isMoved.current = false;

      // If NOT long press triggered, and we should trigger click, do it
      // Only trigger if we actually started the press AND didn't move too much (prevents ghost clicks and scroll-clicks)
      if (shouldTriggerClick && !longPressTriggered && wasStarted && !wasMoved) {
        onClick(e);
      }
      
      setLongPressTriggered(false);
      startPos.current = null;
    },
    [onClick, longPressTriggered]
  );

  return {
    onMouseDown: (e: React.MouseEvent) => start(e),
    onTouchStart: (e: React.TouchEvent) => start(e),
    onMouseUp: (e: React.MouseEvent) => clear(e),
    onMouseLeave: (e: React.MouseEvent) => clear(e, false), // Mouse leave cancels everything
    onTouchEnd: (e: React.TouchEvent) => clear(e),
    onTouchMove: (e: React.TouchEvent) => {
      // Allow small movement (jitter) for touch
      if (startPos.current) {
        const moveX = e.touches[0].clientX;
        const moveY = e.touches[0].clientY;
        const diffX = Math.abs(moveX - startPos.current.x);
        const diffY = Math.abs(moveY - startPos.current.y);
        
        // If movement is within threshold (15px to allow for some jitter/fat fingers), don't cancel
        if (diffX < 15 && diffY < 15) return;
        
        // If moved significantly, mark as moved so we don't trigger click on release
        isMoved.current = true;
      }

      timeout.current && clearTimeout(timeout.current);
      setLongPressTriggered(false);
    },
  };
};
