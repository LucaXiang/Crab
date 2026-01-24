import React from 'react';
import { CartAnimationOverlay } from '@/presentation/components/CartAnimationOverlay';
import { useAnimations, useUIActions } from '@/core/stores/ui';

export const CartAnimationLayer: React.FC = React.memo(() => {
	  const animations = useAnimations();
	  const { removeAnimation } = useUIActions();

	  if (!animations.length) return null;

	  return (
	    <CartAnimationOverlay items={animations} onComplete={removeAnimation} />
	  );
});

CartAnimationLayer.displayName = 'CartAnimationLayer';
