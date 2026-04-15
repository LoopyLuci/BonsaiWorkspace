import { writable } from 'svelte/store';

export type ToastType = 'success' | 'error' | 'info';

export interface ToastMessage {
  id: string;
  text: string;
  type: ToastType;
}

export const toasts = writable<ToastMessage[]>([]);

export function addToast(text: string, type: ToastType = 'success', duration = 4000) {
  const id = crypto.randomUUID();
  toasts.update((items) => [...items, { id, text, type }]);
  setTimeout(() => removeToast(id), duration);
}

export function removeToast(id: string) {
  toasts.update((items) => items.filter((toast) => toast.id !== id));
}
