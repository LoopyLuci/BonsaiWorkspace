export const DEFAULT_API_HOST = '127.0.0.1';
export const DEFAULT_API_PORT = 11369;
export const DEFAULT_WS_PATH = '/ws';

export function buildDefaultWsUrl(host: string = DEFAULT_API_HOST, port: number = DEFAULT_API_PORT): string {
  return `ws://${host}:${port}${DEFAULT_WS_PATH}`;
}
