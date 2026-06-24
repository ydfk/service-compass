import type { GlobalThemeOverrides } from 'naive-ui'

export const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#5da9ff',
    primaryColorHover: '#82bdff',
    primaryColorPressed: '#398de9',
    borderRadius: '10px',
    bodyColor: '#080d17',
    cardColor: '#101827',
    modalColor: '#101827',
  },
  Card: {
    borderColor: 'rgba(148, 163, 184, 0.14)',
  },
  DataTable: {
    thColor: '#0c1320',
    tdColor: '#101827',
    tdColorHover: '#141f31',
  },
}
