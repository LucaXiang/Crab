import React from 'react';
import { CartAnimationOverlay } from '@/presentation/components/CartAnimationOverlay';
import { useAnimations } from '@/core/stores/ui/useAnimations';

export const CartAnimationLayer: React.FC = React.memo(() => {
	  const { animationQueue, removeAnimation } = useAnimations();

	  if (!animationQueue.length) return null;

	  return (
	    <CartAnimationOverlay items={animationQueue} onComplete={removeAnimation} />
	  );
});

CartAnimationLayer.displayName = 'CartAnimationLayer';
