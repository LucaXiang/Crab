import React, { useRef } from 'react';

interface JsonEditorProps {
  value: string;
  onChange: (value: string) => void;
  className?: string;
  placeholder?: string;
}

export const JsonEditor: React.FC<JsonEditorProps> = ({ value, onChange, className = '', placeholder }) => {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const preRef = useRef<HTMLPreElement>(null);

  const handleScroll = () => {
    if (textareaRef.current && preRef.current) {
      preRef.current.scrollTop = textareaRef.current.scrollTop;
      preRef.current.scrollLeft = textareaRef.current.scrollLeft;
    }
  };

  const handleBlur = () => {
    try {
      if (!value.trim()) return;
      const parsed = JSON.parse(value);
      const formatted = JSON.stringify(parsed, null, 2);
      onChange(formatted);
    } catch (e) {
      // Invalid JSON - keep as is, maybe could add error state visually later
    }
  };

  const highlight = (code: string) => {
    if (!code) return '';
    // Escape HTML entities to prevent XSS and rendering issues
    const escaped = code
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');

    return escaped.replace(
      /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
      (match) => {
        let cls = 'text-blue-600'; // number
        if (/^"/.test(match)) {
          if (/:$/.test(match)) {
            cls = 'text-purple-700 font-medium'; // key
          } else {
            cls = 'text-green-600'; // string
          }
        } else if (/true|false/.test(match)) {
          cls = 'text-orange-600'; // boolean
        } else if (/null/.test(match)) {
          cls = 'text-gray-500'; // null
        }
        return `<span class="${cls}">${match}</span>`;
      }
    );
  };

  // Base styles shared between pre and textarea to ensure perfect alignment
  const baseStyles = "font-mono text-xs leading-normal p-3";

  return (
    <div className={`relative group bg-white rounded-lg ${className}`}>
      {/* Background Highlighter */}
      <pre
        ref={preRef}
        aria-hidden="true"
        className={`absolute inset-0 pointer-events-none whitespace-pre overflow-hidden rounded-lg ${baseStyles}`}
        style={{ margin: 0, border: '1px solid transparent' }} // border transparent to match textarea border width
        dangerouslySetInnerHTML={{ __html: highlight(value) + '<br/>' }} 
      />
      
      {/* Foreground Input */}
      <textarea
        ref={textareaRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onBlur={handleBlur}
        onScroll={handleScroll}
        className={`relative w-full h-full bg-transparent text-transparent caret-gray-900 resize-none border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent whitespace-pre overflow-auto ${baseStyles}`}
        placeholder={placeholder}
        spellCheck={false}
        autoCapitalize="off"
        autoComplete="off"
        autoCorrect="off"
      />
    </div>
  );
};
