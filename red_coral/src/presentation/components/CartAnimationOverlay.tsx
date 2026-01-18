import React, { useEffect, useState } from 'react';

export interface AnimationItem {
  id: string;
  type: string;
  data?: any;
  image?: string;
  startRect?: DOMRect;
  targetX?: number;
  targetY?: number;
}

interface Props {
  items: AnimationItem[];
  onComplete: (id: string) => void;
}

const FlyingItem: React.FC<{ item: AnimationItem; onComplete: (id: string) => void }> = ({ item, onComplete }) => {
  const startRect = item.startRect || { left: 0, top: 0, width: 0, height: 0, right: 0, bottom: 0 };
  const targetX = item.targetX ?? 0;
  const targetY = item.targetY ?? 0;

  const [style, setStyle] = useState<React.CSSProperties>({
    position: 'fixed',
    left: startRect.left,
    top: startRect.top,
    width: startRect.width,
    height: startRect.height,
    opacity: 0.8,
    zIndex: 9999,
    pointerEvents: 'none',
    borderRadius: '4px',
    objectFit: 'cover',
    transformOrigin: 'center center',
    transition: 'all 0.6s cubic-bezier(0.19, 1, 0.22, 1)', // Ease-out expo for "throw" feel
    boxShadow: '0 10px 25px rgba(0,0,0,0.3)',
  });

  useEffect(() => {
    // Trigger animation in next frame to ensure browser renders initial position first
    const frameId = requestAnimationFrame(() => {
        setStyle(prev => ({
            ...prev,
            left: targetX,
            top: targetY,
            width: 20, // Shrink to dot size
            height: 20,
            opacity: 0, // Fade out at end
            borderRadius: '50%', // Turn into a dot
            transform: 'scale(0.5)'
        }));
    });

    return () => cancelAnimationFrame(frameId);
  }, [targetX, targetY]);

  return (
    <img 
        src={item.image || undefined} 
        style={style}
        onTransitionEnd={() => onComplete(item.id)}
        alt=""
    />
  );
};

export const CartAnimationOverlay: React.FC<Props> = React.memo(({ items, onComplete }) => {
  return (
    <div className="fixed inset-0 pointer-events-none z-[100]">
      {items.map(item => (
        <FlyingItem key={item.id} item={item} onComplete={onComplete} />
      ))}
    </div>
  );
});
