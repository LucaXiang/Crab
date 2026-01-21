import React, { useEffect, useState } from 'react';
import { X, Save } from 'lucide-react';
import { DndContext, closestCenter, KeyboardSensor, PointerSensor, useSensor, useSensors, DragEndEvent, DragStartEvent, DragOverlay, defaultDropAnimation } from '@dnd-kit/core';
import { arrayMove, SortableContext, sortableKeyboardCoordinates, rectSortingStrategy, useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';

const api = createTauriClient();
import { toast } from '@/presentation/components/Toast';
import { Product } from '@/core/domain/types';
import DefaultImage from '@/assets/reshot.svg';
import { useImageUrl } from '@/core/hooks/useImageUrl';
import { useSettingsStore } from '@/core/stores/settings';

interface SortableProductItemProps {
  id: string;
  product: Product;
}

const SortableProductItem: React.FC<SortableProductItemProps> = ({ id, product }) => {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id,
    animateLayoutChanges: (args) => !args.isSorting && !args.wasDragging,
  });

  const [imageUrl] = useImageUrl(product.image);
  const imageSrc = imageUrl || DefaultImage;

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    zIndex: isDragging ? 1 : 0,
    opacity: isDragging ? 0.3 : 1, // Dragging item becomes semi-transparent
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...listeners}
      className={`group relative flex flex-col items-start gap-0 p-0 rounded-xl overflow-hidden transition-all duration-200 cursor-grab active:cursor-grabbing select-none h-full bg-white border
        ${isDragging
          ? 'border-dashed border-gray-300 shadow-none opacity-40 grayscale'
          : 'border-gray-100 shadow-sm hover:shadow-md hover:border-teal-200'
        }`}
    >
      <div className="w-full aspect-square bg-gray-50 relative overflow-hidden">
        <img
          src={imageSrc}
          alt={product.name}
          className="w-full h-full object-cover pointer-events-none group-hover:scale-105 transition-transform duration-500 ease-out"
          onError={(e) => { (e.target as HTMLImageElement).src = DefaultImage; }}
        />
        {/* Sort Order Tag - Bottom Left Black/White */}
        <div className="absolute bottom-1 left-1 z-10">
           <div className="bg-black/80 backdrop-blur-[1px] px-1.5 py-0.5 rounded shadow-sm min-w-[20px] flex items-center justify-center">
             <span className="text-[10px] text-white font-medium font-mono leading-none">
               {product.sort_order}
             </span>
           </div>
        </div>
      </div>
      
      <div className="flex-1 flex items-center w-full p-2.5 bg-white">
        <span className="font-medium text-gray-700 text-xs text-left line-clamp-2 leading-tight w-full">
          {product.name}
        </span>
      </div>
    </div>
  );
};

// Drag overlay item component (separate to use hooks)
const DragOverlayProductItem: React.FC<{ product: Product }> = ({ product }) => {
  const [imageUrl] = useImageUrl(product.image);
  const imageSrc = imageUrl || DefaultImage;

  return (
    <div className="group relative flex flex-col items-start gap-0 p-0 rounded-xl overflow-hidden cursor-grabbing select-none h-full bg-white border border-teal-500 shadow-xl scale-[1.02]">
      <div className="w-full aspect-square bg-gray-50 relative overflow-hidden">
        <img
          src={imageSrc}
          alt={product.name}
          className="w-full h-full object-cover pointer-events-none"
          onError={(e) => { (e.target as HTMLImageElement).src = DefaultImage; }}
        />
        <div className="absolute bottom-1 left-1 z-10">
          <div className="bg-black/80 backdrop-blur-[1px] px-1.5 py-0.5 rounded shadow-sm min-w-[20px] flex items-center justify-center">
            <span className="text-[10px] text-white font-medium font-mono leading-none">
              {product.sort_order}
            </span>
          </div>
        </div>
      </div>
      <div className="flex-1 flex items-center w-full p-2.5 bg-white">
        <span className="font-medium text-gray-700 text-xs text-left line-clamp-2 leading-tight w-full">
          {product.name}
        </span>
      </div>
    </div>
  );
};

interface ProductOrderModalProps {
  isOpen: boolean;
  onClose: () => void;
  category: string;
}

export const ProductOrderModal: React.FC<ProductOrderModalProps> = ({ isOpen, category, onClose }) => {
  const { t } = useI18n();
  const [products, setProducts] = useState<Product[]>([]);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [activeId, setActiveId] = useState<string | null>(null);
  const refreshData = useSettingsStore((s) => s.refreshData);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8, // Instant drag, just enough to prevent accidental clicks
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  useEffect(() => {
    if (isOpen && category) {
      loadProducts();
    }
  }, [isOpen, category]);

  const loadProducts = async () => {
    setLoading(true);
    try {
      const resp = await api.listProducts();
      const allProducts = resp.data?.products || [];
      // Filter by category locally
      const filteredProducts = allProducts.filter(p => p.category === category);
      setProducts(filteredProducts);
    } catch (e) {
      console.error(e);
      toast.error(t('common.message.loadFailed'));
    } finally {
      setLoading(false);
    }
  };

  const handleDragStart = (event: DragStartEvent) => {
    const { active } = event;
    setActiveId(String(active.id));
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;

    if (over && active.id !== over.id) {
      setProducts((items) => {
        const oldIndex = items.findIndex((item) => item.id === active.id);
        const newIndex = items.findIndex((item) => item.id === over.id);
        return arrayMove(items, oldIndex, newIndex);
      });
    }
    setActiveId(null);
  };

  const handleDragCancel = () => {
    setActiveId(null);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      // API doesn't support batch reorder, skipping
      refreshData();
      toast.success(t('common.message.saveSuccess'));
      onClose();
    } catch (e) {
      console.error(e);
      toast.error(t('common.message.saveFailed'));
    } finally {
      setSaving(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-gray-900/60 backdrop-blur-md transition-opacity duration-300">
      <div className="bg-white rounded-2xl w-full max-w-5xl max-h-[85vh] flex flex-col shadow-2xl overflow-hidden ring-1 ring-black/5">
        <div className="px-8 py-6 border-b border-gray-100 flex items-center justify-between bg-white/80 backdrop-blur-sm sticky top-0 z-10">
          <div>
            <h3 className="text-2xl font-bold text-gray-900 tracking-tight">
              {t('settings.productOrder')}
            </h3>
            <div className="flex items-center gap-3 mt-2">
               <span className="px-2.5 py-0.5 rounded-full bg-teal-50 text-teal-700 text-xs font-semibold border border-teal-100">
                 {category}
               </span>
               <p className="text-xs text-gray-400">
                {t('settings.dragToReorder')}
               </p>
            </div>
          </div>
          <button 
            onClick={onClose} 
            className="p-2.5 hover:bg-gray-100 rounded-full text-gray-400 hover:text-gray-600 transition-all hover:rotate-90 active:scale-95"
          >
            <X size={24} />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-8 bg-gray-50/50 scrollbar-thin scrollbar-thumb-gray-200 scrollbar-track-transparent">
          {loading ? (
            <div className="flex items-center justify-center h-40">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-teal-600" />
            </div>
          ) : products.length === 0 ? (
            <div className="text-center py-12 text-gray-400">
              {t("settings.product.noProducts")}
            </div>
          ) : (
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragStart={handleDragStart}
              onDragEnd={handleDragEnd}
              onDragCancel={handleDragCancel}
            >
              <SortableContext items={products.map(p => p.id)} strategy={rectSortingStrategy}>
                <div className="grid grid-cols-4 sm:grid-cols-5 md:grid-cols-6 gap-4 p-2">
                  {products.map((product) => (
                    <SortableProductItem key={product.id} id={product.id} product={product} />
                  ))}
                </div>
              </SortableContext>
              <DragOverlay dropAnimation={{ ...defaultDropAnimation, duration: 200, easing: 'ease-out' }}>
                {activeId ? (
                  (() => {
                    const p = products.find((x) => x.id === activeId);
                    if (!p) return null;
                    return <DragOverlayProductItem product={p} />;
                  })()
                ) : null}
              </DragOverlay>
            </DndContext>
          )}
        </div>

        <div className="px-8 py-5 border-t border-gray-100 flex justify-end gap-3 bg-white sticky bottom-0 z-10">
          <button
            onClick={onClose}
            className="px-5 py-2.5 text-gray-600 hover:bg-gray-100 rounded-xl text-sm font-medium transition-colors border border-transparent hover:border-gray-200"
          >
            {t('common.action.cancel')}
          </button>
          <button
            onClick={handleSave}
            disabled={saving || products.length === 0}
            className="flex items-center gap-2 px-6 py-2.5 bg-teal-600 text-white rounded-xl text-sm font-bold shadow-lg shadow-teal-600/20 hover:bg-teal-700 hover:shadow-teal-600/30 hover:-translate-y-0.5 active:translate-y-0 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:translate-y-0 disabled:hover:shadow-none transition-all"
          >
            {saving ? (
              <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
            ) : (
              <Save size={18} />
            )}
            <span>{t('common.action.save')}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
