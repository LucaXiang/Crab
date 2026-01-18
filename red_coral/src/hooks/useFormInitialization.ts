import { useState, useEffect } from 'react';

/**
 * Custom hook for initializing form state based on edit mode
 * Automatically resets form to default values when item is null (create mode)
 * Populates form with item data when item is provided (edit mode)
 *
 * @param item - The item to edit (null for create mode)
 * @param defaultValues - Default values for create mode (can omit fields like id)
 * @param dependencies - Additional dependencies to trigger reinitialization
 * @returns [formData, setFormData] tuple
 */
export function useFormInitialization<T extends Record<string, any>>(
  item: T | null,
  defaultValues: Partial<T> & Record<string, any>,
  dependencies: any[] = []
): [T, React.Dispatch<React.SetStateAction<T>>] {
  const [formData, setFormData] = useState<T>(defaultValues as T);

  useEffect(() => {
    if (item) {
      // Edit mode: populate with item data, merging with defaults for missing fields
      setFormData({ ...defaultValues, ...item });
    } else {
      // Create mode: use default values
      setFormData(defaultValues as T);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [item, ...dependencies]);

  return [formData, setFormData];
}
