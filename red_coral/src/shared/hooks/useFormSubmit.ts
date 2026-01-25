import { toast } from '@/presentation/components/Toast';
import { useI18n } from '@/hooks/useI18n';

export interface FormSubmitOptions<T> {
  validationRules?: (data: T) => string | null;
  onCreate: (data: T) => Promise<unknown>;
  onUpdate: (data: T) => Promise<unknown>;
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
export function useFormSubmit<T>(
  editingItem: unknown,
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
          successMessage?.update || t('settings.user.message.update_success')
        );
      } else {
        // Create mode
        await onCreate(formData);
        toast.success(
          successMessage?.create || t('settings.user.message.update_success')
        );
      }

      // Trigger success callback
      onSuccess?.();
    } catch (error: unknown) {
      const errorMessage = error instanceof Error ? error.message : t('settings.user.message.update_failed');
      toast.error(errorMessage);
    }
  };

  return {
    handleSubmit,
    isSubmitting: false // Can be extended to track submission state
  };
}
