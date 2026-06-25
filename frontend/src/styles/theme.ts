import type { GlobalThemeOverrides } from 'naive-ui'

export const darkThemeOverrides: GlobalThemeOverrides = {
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

export const lightThemeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#2563eb',
    primaryColorHover: '#3b82f6',
    primaryColorPressed: '#1d4ed8',
    borderRadius: '10px',
    bodyColor: '#f4f7fb',
    cardColor: '#ffffff',
    modalColor: '#ffffff',
    textColorBase: '#172033',
  },
  Card: {
    borderColor: 'rgba(30, 41, 59, 0.1)',
  },
  DataTable: {
    thColor: '#eef3f9',
    tdColor: '#ffffff',
    tdColorHover: '#f6f9fd',
  },
}
