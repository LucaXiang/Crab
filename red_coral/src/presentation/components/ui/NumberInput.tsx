import React, { useState, useEffect } from 'react';

export interface NumberInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  value: number;
  onValueChange: (value: number) => void;
}

export const NumberInput: React.FC<NumberInputProps> = ({ value, onValueChange, className, ...props }) => {
  const [inputValue, setInputValue] = useState(String(value));
  const [isFocused, setIsFocused] = useState(false);

  useEffect(() => {
    // Only sync from external value if not focused to avoid interrupting typing
    if (!isFocused) {
      setInputValue(String(value));
    }
  }, [value, isFocused]);

  const handleBlur = () => {
    setIsFocused(false);
    let newVal = parseFloat(inputValue);
    
    // Default to 0 if empty or invalid
    if (isNaN(newVal)) {
      newVal = 0;
    }

    onValueChange(newVal);
    // Force format to number string (removes leading zeros, etc)
    setInputValue(String(newVal));
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.currentTarget.blur();
    }
  };

  return (
    <input
      type="number"
      className={className}
      value={inputValue}
      onChange={(e) => setInputValue(e.target.value)}
      onFocus={() => setIsFocused(true)}
      onBlur={handleBlur}
      onKeyDown={handleKeyDown}
      {...props}
    />
  );
};
