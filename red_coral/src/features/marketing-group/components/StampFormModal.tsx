import React, { useState } from 'react';
import { X, Plus, Trash2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useCategoryStore } from '@/features/category/store';
import { useProductStore } from '@/core/stores/resources';
import type {
  StampActivityDetail,
  StampActivityCreate,
  StampTargetInput,
  RewardStrategy,
  StampTargetType,
} from '@/core/domain/types/api';

interface StampFormModalProps {
  activity: StampActivityDetail | null;
  onSave: (data: StampActivityCreate) => void;
  onClose: () => void;
}

export const StampFormModal: React.FC<StampFormModalProps> = ({ activity, onSave, onClose }) => {
  const { t } = useI18n();
  const categories = useCategoryStore((s) => s.items);
  const products = useProductStore((s) => s.items);

  const [name, setName] = useState(activity?.name || '');
  const [displayName, setDisplayName] = useState(activity?.display_name || '');
  const [stampsRequired, setStampsRequired] = useState(activity?.stamps_required || 10);
  const [rewardQuantity, setRewardQuantity] = useState(activity?.reward_quantity || 1);
  const [rewardStrategy, setRewardStrategy] = useState<RewardStrategy>(
    activity?.reward_strategy || 'ECONOMIZADOR'
  );
  const [designatedProductId, setDesignatedProductId] = useState<number | null>(
    activity?.designated_product_id || null
  );
  const [isCyclic, setIsCyclic] = useState(activity?.is_cyclic ?? true);
  const [stampTargets, setStampTargets] = useState<StampTargetInput[]>(
    activity?.stamp_targets.map((st) => ({ target_type: st.target_type, target_id: st.target_id })) || []
  );
  const [rewardTargets, setRewardTargets] = useState<StampTargetInput[]>(
    activity?.reward_targets.map((rt) => ({ target_type: rt.target_type, target_id: rt.target_id })) || []
  );

  const canSave = name.trim() && displayName.trim() && stampsRequired > 0 && stampTargets.length > 0;

  const handleSubmit = () => {
    onSave({
      name,
      display_name: displayName,
      stamps_required: stampsRequired,
      reward_quantity: rewardQuantity,
      reward_strategy: rewardStrategy,
      designated_product_id: rewardStrategy === 'DESIGNATED' ? designatedProductId : null,
      is_cyclic: isCyclic,
      stamp_targets: stampTargets,
      reward_targets: rewardTargets,
    });
  };

  const addTarget = (list: StampTargetInput[], setList: React.Dispatch<React.SetStateAction<StampTargetInput[]>>) => {
    const firstCat = categories[0];
    if (firstCat) {
      setList([...list, { target_type: 'CATEGORY', target_id: firstCat.id }]);
    }
  };

  const removeTarget = (list: StampTargetInput[], setList: React.Dispatch<React.SetStateAction<StampTargetInput[]>>, idx: number) => {
    setList(list.filter((_, i) => i !== idx));
  };

  const updateTarget = (
    list: StampTargetInput[],
    setList: React.Dispatch<React.SetStateAction<StampTargetInput[]>>,
    idx: number,
    updates: Partial<StampTargetInput>
  ) => {
    setList(list.map((item, i) => (i === idx ? { ...item, ...updates } : item)));
  };

  const renderTargetRows = (
    list: StampTargetInput[],
    setList: React.Dispatch<React.SetStateAction<StampTargetInput[]>>,
    label: string
  ) => (
    <div>
      <div className="flex items-center justify-between mb-2">
        <label className="text-sm font-medium text-gray-700">{label}</label>
        <button
          type="button"
          onClick={() => addTarget(list, setList)}
          className="text-xs text-violet-600 hover:text-violet-700 flex items-center gap-1"
        >
          <Plus size={12} />
          {t('settings.marketing_group.stamp.add_target')}
        </button>
      </div>
      {list.length === 0 ? (
        <p className="text-xs text-gray-400">{t('settings.marketing_group.stamp.no_targets')}</p>
      ) : (
        <div className="space-y-2">
          {list.map((target, idx) => (
            <div key={idx} className="flex items-center gap-2">
              <select
                value={target.target_type}
                onChange={(e) => {
                  const newType = e.target.value as StampTargetType;
                  const firstEntity = newType === 'CATEGORY' ? categories[0] : products[0];
                  updateTarget(list, setList, idx, {
                    target_type: newType,
                    target_id: firstEntity?.id || 0,
                  });
                }}
                className="px-2 py-1.5 border border-gray-300 rounded-lg text-sm w-24"
              >
                <option value="CATEGORY">{t('settings.marketing_group.stamp.target_type.category')}</option>
                <option value="PRODUCT">{t('settings.marketing_group.stamp.target_type.product')}</option>
              </select>
              <select
                value={target.target_id}
                onChange={(e) =>
                  updateTarget(list, setList, idx, { target_id: Number(e.target.value) })
                }
                className="flex-1 px-2 py-1.5 border border-gray-300 rounded-lg text-sm"
              >
                {(target.target_type === 'CATEGORY' ? categories : products).map((entity) => (
                  <option key={entity.id} value={entity.id}>
                    {entity.name}
                  </option>
                ))}
              </select>
              <button
                type="button"
                onClick={() => removeTarget(list, setList, idx)}
                className="p-1 text-gray-400 hover:text-red-500 transition-colors"
              >
                <Trash2 size={14} />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );

  return (
    <div className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl max-w-lg w-full max-h-[90vh] flex flex-col animate-in zoom-in-95">
        {/* Header */}
        <div className="p-5 border-b border-gray-100 flex items-center justify-between shrink-0">
          <h3 className="text-xl font-bold text-gray-800">
            {activity ? t('settings.marketing_group.edit_stamp') : t('settings.marketing_group.add_stamp')}
          </h3>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-lg transition-colors">
            <X size={20} className="text-gray-400" />
          </button>
        </div>

        {/* Form */}
        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {/* Name fields */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.marketing_group.field.name')}
              </label>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500"
                placeholder="coffee_stamp"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.marketing_group.field.display_name')}
              </label>
              <input
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500"
                placeholder={t('settings.marketing_group.stamp.display_name_placeholder')}
              />
            </div>
          </div>

          {/* Stamps & Reward count */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.marketing_group.stamp.stamps_required')}
              </label>
              <input
                type="number"
                value={stampsRequired}
                onChange={(e) => setStampsRequired(Number(e.target.value) || 0)}
                min={1}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.marketing_group.stamp.reward_quantity')}
              </label>
              <input
                type="number"
                value={rewardQuantity}
                onChange={(e) => setRewardQuantity(Number(e.target.value) || 0)}
                min={1}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500"
              />
            </div>
          </div>

          {/* Reward Strategy */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              {t('settings.marketing_group.stamp.reward_strategy')}
            </label>
            <div className="grid grid-cols-3 gap-2">
              {(['ECONOMIZADOR', 'GENEROSO', 'DESIGNATED'] as RewardStrategy[]).map((strategy) => (
                <button
                  key={strategy}
                  type="button"
                  onClick={() => setRewardStrategy(strategy)}
                  className={`px-3 py-2 rounded-xl text-xs font-medium transition-colors ${
                    rewardStrategy === strategy
                      ? 'bg-violet-50 text-violet-700 ring-2 ring-violet-400'
                      : 'bg-gray-50 text-gray-600 hover:bg-gray-100'
                  }`}
                >
                  {t(`settings.marketing_group.stamp.strategy.${strategy.toLowerCase()}`)}
                </button>
              ))}
            </div>
          </div>

          {/* Designated Product (only if DESIGNATED) */}
          {rewardStrategy === 'DESIGNATED' && (
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.marketing_group.stamp.designated_product')}
              </label>
              <select
                value={designatedProductId || ''}
                onChange={(e) => setDesignatedProductId(e.target.value ? Number(e.target.value) : null)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg"
              >
                <option value="">{t('common.hint.select')}</option>
                {products.map((p) => (
                  <option key={p.id} value={p.id}>{p.name}</option>
                ))}
              </select>
            </div>
          )}

          {/* Cyclic toggle */}
          <div className="flex items-center justify-between bg-gray-50 rounded-xl p-3">
            <div>
              <span className="text-sm font-medium text-gray-700">
                {t('settings.marketing_group.stamp.is_cyclic')}
              </span>
              <p className="text-xs text-gray-400 mt-0.5">
                {t('settings.marketing_group.stamp.cyclic_hint')}
              </p>
            </div>
            <button
              type="button"
              onClick={() => setIsCyclic(!isCyclic)}
              className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                isCyclic ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-500'
              }`}
            >
              {isCyclic ? t('common.status.enabled') : t('common.status.disabled')}
            </button>
          </div>

          {/* Stamp targets */}
          {renderTargetRows(stampTargets, setStampTargets, t('settings.marketing_group.stamp.stamp_targets'))}

          {/* Reward targets */}
          {renderTargetRows(rewardTargets, setRewardTargets, t('settings.marketing_group.stamp.reward_targets'))}
        </div>

        {/* Footer */}
        <div className="p-5 border-t border-gray-100 flex gap-3 shrink-0">
          <button
            onClick={onClose}
            className="flex-1 px-4 py-3 bg-gray-200 text-gray-700 rounded-xl font-bold hover:bg-gray-300 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
          <button
            onClick={handleSubmit}
            disabled={!canSave}
            className="flex-1 px-4 py-3 bg-violet-500 text-white rounded-xl font-bold hover:bg-violet-600 disabled:opacity-50 transition-colors"
          >
            {t('common.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
