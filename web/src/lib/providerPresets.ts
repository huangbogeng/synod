export interface ProviderPreset {
  id: 'deepseek' | 'minimax';
  name: string;
  description: string;
  baseUrl: string;
  environmentName: string;
  modelName: string;
  accent: 'deepseek' | 'minimax';
}

export const providerPresets: ProviderPreset[] = [
  {
    id: 'deepseek',
    name: 'DeepSeek',
    description: 'Official Chat API',
    baseUrl: 'https://api.deepseek.com',
    environmentName: 'DEEPSEEK_API_KEY',
    modelName: 'deepseek-v4-pro',
    accent: 'deepseek'
  },
  {
    id: 'minimax',
    name: 'MiniMax',
    description: 'Official Chat API',
    baseUrl: 'https://api.minimaxi.com/v1',
    environmentName: 'MINIMAX_API_KEY',
    modelName: 'MiniMax-M2.7',
    accent: 'minimax'
  }
];
