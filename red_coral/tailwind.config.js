/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#fff1f1',
          100: '#ffe4e4',
          200: '#ffcccc',
          300: '#ffa8a8',
          400: '#ff7a7a',
          500: '#FF5E5E',  // 主色调
          600: '#e63946',  // hover 状态
          700: '#c92a36',
          800: '#a82530',
          900: '#8c222b',
        },
      },
      fontFamily: {
        sans: ['Roboto', 'LXGW WenKai GB', 'sans-serif'],
      },
    },
  },
  plugins: [],
}
