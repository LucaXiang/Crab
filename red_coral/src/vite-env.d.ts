/// <reference types="vite/client" />

declare module '*.svg' {
  import * as React from 'react';
  export const ReactComponent: React.FunctionComponent<React.SVGProps<SVGSVGElement> & { title?: string }>;
  const src: string;
  export default src;
}

declare module 'pinyin-engine' {
  class PinyinEngine {
    constructor(data: string[], indexs?: string | string[], begin?: boolean);
    constructor(data: object[], indexs: string | string[], begin?: boolean);
    query(keyword: string): string[];
    static participle(keyword: string): string;
  }
  export default PinyinEngine;
}
