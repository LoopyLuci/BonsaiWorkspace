import { writable } from "svelte/store";

// Authentication state
export const isAuthenticated = writable(false);

// Current user info
export const currentUser = writable(null);

// Notifications queue
export const notifications = writable([]);

// Add notification helper
export function addNotification(notification) {
  const id = Math.random().toString(36).substring(7);
  const notif = { ...notification, id };

  notifications.update((n) => [...n, notif]);

  // Auto-remove after 5 seconds
  setTimeout(() => {
    notifications.update((n) => n.filter((notif) => notif.id !== id));
  }, 5000);

  return id;
}

// Remove notification
export function removeNotification(id) {
  notifications.update((n) => n.filter((notif) => notif.id !== id));
}

// Clear all notifications
export function clearNotifications() {
  notifications.set([]);
}
