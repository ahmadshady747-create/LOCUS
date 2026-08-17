/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        locus: {
          bg: "#08090D",
          panel: "#0F1117",
          card: "#161822",
          border: "#27272A",
          accent: "#10B981",
          accentDim: "#059669",
          violet: "#8B5CF6",
          violetDim: "#7C3AED",
          text: "#E4E4E7",
          muted: "#71717A",
          code: "#08090D",
        },
      },
      fontFamily: {
        mono: ["JetBrains Mono", "Fira Code", "Consolas", "monospace"],
        sans: ["IBM Plex Sans", "Inter", "system-ui", "sans-serif"],
      },
      boxShadow: {
        'glow-violet': '0 0 20px rgba(139, 92, 246, 0.15)',
        'glow-emerald': '0 0 20px rgba(16, 185, 129, 0.15)',
        'inner-glow': 'inset 0 0 20px rgba(139, 92, 246, 0.05)',
      },
      transitionDuration: {
        '200': '200ms',
      },
    },
  },
  plugins: [],
};