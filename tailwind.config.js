/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{svelte,ts}'],
  theme: {
    extend: {
      colors: {
        // Design tokens from the PRD (Glassmorphism + Minimalism)
        base: '#0F0F0F',
        surface: '#16161A',
        'surface-2': '#1D1D23',
        stroke: 'rgba(255,255,255,0.08)',
        ink: '#FFFFFF',
        muted: '#A0A0A0',
        faint: '#6B6B6F',
        accent: '#6366F1',
        'accent-2': '#8B5CF6',
        success: '#10B981',
        danger: '#EF4444',
        warning: '#F59E0B'
      },
      fontFamily: {
        // Native-feel stack: Segoe UI Variable on Windows, SF on macOS, Inter elsewhere
        sans: [
          'Inter',
          'Segoe UI Variable Text',
          'Segoe UI',
          'system-ui',
          '-apple-system',
          'sans-serif'
        ],
        mono: ['JetBrains Mono', 'Cascadia Code', 'Consolas', 'monospace']
      },
      fontSize: {
        title: ['18px', '24px'],
        body: ['14px', '20px'],
        small: ['12px', '16px']
      },
      animation: {
        'palette-in': 'palette-in 200ms ease-out',
        'fade-in': 'fade-in 100ms ease-out',
        'spring-in': 'spring-in 150ms cubic-bezier(0.34, 1.56, 0.64, 1)'
      },
      keyframes: {
        'palette-in': {
          from: { opacity: '0', transform: 'translateY(-8px) scale(0.98)' },
          to: { opacity: '1', transform: 'translateY(0) scale(1)' }
        },
        'fade-in': { from: { opacity: '0' }, to: { opacity: '1' } },
        'spring-in': {
          from: { opacity: '0', transform: 'scale(0.96)' },
          to: { opacity: '1', transform: 'scale(1)' }
        }
      }
    }
  },
  plugins: []
};
