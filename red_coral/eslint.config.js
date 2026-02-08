import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist/', 'src-tauri/', 'node_modules/', 'escpos-rs/', 'i18n-tools/'] },
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  {
    linterOptions: {
      reportUnusedDisableDirectives: 'off',
    },
    rules: {
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-unused-vars': 'off',
      '@typescript-eslint/no-unused-expressions': 'off',
      '@typescript-eslint/no-empty-object-type': 'off',
      'no-useless-assignment': 'off',
      'no-useless-catch': 'off',
      'no-useless-escape': 'off',
      'no-empty': 'off',
      'no-shadow-restricted-names': 'off',
      'prefer-const': 'off',
      'preserve-caught-error': 'off',
    },
  },
);
