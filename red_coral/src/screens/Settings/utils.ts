
// Helper for attribute binding synchronization
export async function syncAttributeBindings(
  selectedAttributeIds: string[],
  attributeDefaultOptions: Record<string, string | string[]>,
  existingBindings: { attributeId: string; id: string; defaultOptionIds?: string[]; defaultOptionId?: string }[],
  unbindFn: (bindingId: string) => Promise<unknown>,
  bindFn: (attributeId: string, defaultOptionIds: string[], displayOrder: number) => Promise<unknown>
) {
  const existingAttributeIds = existingBindings.map(b => b.attributeId);

  // Unbind removed attributes - use binding ID, not attribute ID
  const toUnbind = existingBindings.filter(b => !selectedAttributeIds.includes(b.attributeId));
  for (const binding of toUnbind) {
    try {
      await unbindFn(binding.id);  // Pass binding ID
    } catch (error) {
      throw error;
    }
  }

  // Bind new or updated attributes
  for (let i = 0; i < selectedAttributeIds.length; i++) {
    const attributeId = selectedAttributeIds[i];
    const existingBinding = existingBindings.find(b => b.attributeId === attributeId);

    // Normalize default options to array
    const rawNewDefaults = attributeDefaultOptions?.[attributeId];
    const newDefaultOptionIds = Array.isArray(rawNewDefaults)
      ? rawNewDefaults
      : (rawNewDefaults ? [rawNewDefaults] : []);

    let shouldBind = false;

    if (!existingBinding) {
      // New binding
      shouldBind = true;
    } else {
      // Existing binding, check if default option changed
      const oldDefaultOptionIds = existingBinding.defaultOptionIds ||
                                 (existingBinding.defaultOptionId ? [existingBinding.defaultOptionId] : []);

      const oldStr = [...oldDefaultOptionIds].sort().join(',');
      const newStr = [...newDefaultOptionIds].sort().join(',');

      if (oldStr !== newStr) {
        // Changed! Need to update. Unbind first using binding ID.
        try {
          await unbindFn(existingBinding.id);  // Pass binding ID
        } catch(e) {
          // Ignore BINDING_NOT_FOUND, log others
          const msg = String(e);
          if (!msg.includes('BINDING_NOT_FOUND')) {
             console.error('Failed to unbind for update:', attributeId, e);
          }
        }
        shouldBind = true;
      }
    }

    if (shouldBind) {
      try {
        await bindFn(attributeId, newDefaultOptionIds, i);
      } catch (error) {
        const msg = String(error);
        if (msg.includes('BINDING_ALREADY_EXISTS')) {
             // If it exists but we thought we needed to bind (maybe unbind failed silently or wasn't needed?),
             // we should try to unbind explicitly and retry bind, OR just log warning.
             // If we are here, it means we wanted to UPDATE the binding (change defaults).
             // If ALREADY_EXISTS, it means unbind didn't work.
             console.warn('Binding already exists during sync, attempting force update:', attributeId);
             try {
                // Find binding ID for this attribute
                const binding = existingBindings.find(b => b.attributeId === attributeId);
                if (binding) {
                  await unbindFn(binding.id);  // Pass binding ID
                }
                await bindFn(attributeId, newDefaultOptionIds, i);
             } catch (retryError) {
                throw retryError;
             }
        } else {
             console.error('Failed to bind attribute:', attributeId, error);
        }
      }
    }
  }
}
