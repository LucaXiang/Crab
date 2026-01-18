import { toast } from '@/presentation/components/Toast';
import { useI18n } from './useI18n';

export interface FormSubmitOptions<T> {
  validationRules?: (data: T) => string | null;
  onCreate: (data: T) => Promise<any>;
  onUpdate: (data: T) => Promise<any>;
  onSuccess?: () => void;
  successMessage?: { create?: string; update?: string };
}

export interface UseFormSubmitReturn {
  handleSubmit: () => Promise<void>;
  isSubmitting: boolean;
}

/**
 * Custom hook for handling form submission with validation and error handling
 * Automatically differentiates between create and update operations
 *
 * @param editingItem - The item being edited (null/undefined for create mode)
 * @param formData - Current form data
 * @param options - Submit configuration options
 * @returns { handleSubmit, isSubmitting }
 */
export function useFormSubmit<T extends Record<string, any>>(
  editingItem: any,
  formData: T,
  options: FormSubmitOptions<T>
): UseFormSubmitReturn {
  const { t } = useI18n();
  const { validationRules, onCreate, onUpdate, onSuccess, successMessage } = options;

  const handleSubmit = async () => {
    // Run validation if provided
    if (validationRules) {
      const validationError = validationRules(formData);
      if (validationError) {
        toast.error(validationError);
        return;
      }
    }

    try {
      if (editingItem) {
        // Update mode
        await onUpdate(formData);
        toast.success(
          successMessage?.update || t('settings.user.message.updateSuccess')
        );
      } else {
        // Create mode
        await onCreate(formData);
        toast.success(
          successMessage?.create || t('settings.user.message.updateSuccess')
        );
      }

      // Trigger success callback
      onSuccess?.();
    } catch (error: any) {
      console.error('Form submission error:', error);
      const errorMessage = error.message || t('settings.user.message.updateFailed');
      toast.error(errorMessage);
    }
  };

  return {
    handleSubmit,
    isSubmitting: false // Can be extended to track submission state
  };
}
