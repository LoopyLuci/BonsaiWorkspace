// PATHFINDER Frontend - Offline Sync Utility
// Manages offline exercise queue, CRDT synchronization, and conflict resolution

import apiClient from '../api-client';

interface PendingExercise {
  id: string;
  userId: string;
  exerciseId: string;
  skillId: string;
  wasCorrect: boolean;
  response: string;
  responseTime: number;
  timestamp: number;
  vectorClock: Record<string, number>; // For CRDT conflict resolution
  synced: boolean;
}

interface VectorClock {
  [key: string]: number;
}

// In-memory vector clock for conflict detection
let localVectorClock: VectorClock = {};

/**
 * Initialize offline sync on app startup
 */
export async function initializeOfflineSync() {
  // Register Service Worker
  if ('serviceWorker' in navigator) {
    try {
      const registration = await navigator.serviceWorker.register('/sw.js');
      console.log('[OfflineSync] Service Worker registered:', registration);

      // Request background sync permission
      if ('sync' in registration) {
        await registration.sync.register('sync-exercises');
      }

      // Listen for messages from Service Worker
      navigator.serviceWorker.addEventListener('message', handleSWMessage);
    } catch (error) {
      console.error('[OfflineSync] Service Worker registration failed:', error);
    }
  }

  // Load pending exercises from IndexedDB
  await loadPendingExercises();

  // Set up online/offline listeners
  window.addEventListener('online', handleOnline);
  window.addEventListener('offline', handleOffline);

  console.log('[OfflineSync] Initialized');
}

/**
 * Queue exercise attempt when offline or for reliability
 */
export async function queueExerciseAttempt(
  userId: string,
  exerciseId: string,
  skillId: string,
  wasCorrect: boolean,
  response: string,
  responseTime: number
): Promise<void> {
  const pending: PendingExercise = {
    id: generateId(),
    userId,
    exerciseId,
    skillId,
    wasCorrect,
    response,
    responseTime,
    timestamp: Date.now(),
    vectorClock: { ...localVectorClock },
    synced: false,
  };

  // Try to submit immediately if online
  if (navigator.onLine) {
    try {
      const result = await apiClient.recordExerciseAttempt(
        userId,
        exerciseId,
        skillId,
        wasCorrect,
        response,
        responseTime
      );

      // Update vector clock on successful sync
      updateVectorClock('server');

      console.log('[OfflineSync] Exercise synced immediately:', pending.id);
      return;
    } catch (error) {
      console.warn('[OfflineSync] Failed to sync, queueing for later:', error);
    }
  }

  // Store in IndexedDB for offline handling
  await saveToIndexedDB(pending);
  console.log('[OfflineSync] Exercise queued:', pending.id);

  // Show notification
  notifyOfflineQueue(1);
}

/**
 * Sync all pending exercises when back online
 */
export async function syncPendingExercises(): Promise<void> {
  if (!navigator.onLine) {
    console.log('[OfflineSync] Still offline, skipping sync');
    return;
  }

  console.log('[OfflineSync] Starting sync...');

  try {
    const db = await openIndexedDB();
    const pendingExercises = await getAllFromIndexedDB(db, 'pendingExercises');

    let syncedCount = 0;
    const conflicts: PendingExercise[] = [];

    for (const exercise of pendingExercises) {
      if (exercise.synced) continue; // Skip already synced

      try {
        // Check for conflicts using vector clocks
        const hasConflict = detectConflict(exercise.vectorClock);

        if (hasConflict) {
          conflicts.push(exercise);
          console.warn('[OfflineSync] Conflict detected, will resolve:', exercise.id);
          continue;
        }

        // Submit exercise
        const result = await apiClient.recordExerciseAttempt(
          exercise.userId,
          exercise.exerciseId,
          exercise.skillId,
          exercise.wasCorrect,
          exercise.response,
          exercise.responseTime
        );

        // Mark as synced
        exercise.synced = true;
        await updateInIndexedDB(db, 'pendingExercises', exercise);

        // Update vector clock
        updateVectorClock('local');

        syncedCount++;
        console.log('[OfflineSync] Synced exercise:', exercise.id);
      } catch (error) {
        console.error('[OfflineSync] Failed to sync exercise:', exercise.id, error);
      }
    }

    console.log(`[OfflineSync] Sync complete: ${syncedCount} synced, ${conflicts.length} conflicts`);

    // Handle conflicts (simple conflict resolution: server wins)
    if (conflicts.length > 0) {
      await resolveConflicts(db, conflicts);
    }

    // Remove successfully synced items
    await removeInIndexedDB(db, 'pendingExercises', { synced: true });

    // Notify app of sync completion
    window.dispatchEvent(
      new CustomEvent('offline-sync-complete', {
        detail: { syncedCount, conflictCount: conflicts.length },
      })
    );
  } catch (error) {
    console.error('[OfflineSync] Sync failed:', error);
  }
}

/**
 * Detect conflicts using vector clocks (CRDT approach)
 */
function detectConflict(clientVectorClock: VectorClock): boolean {
  // If server vector clock is ahead in any dimension, there's a conflict
  // This is a simplified check; full CRDT would use more sophisticated comparison

  const clientKeys = Object.keys(clientVectorClock);
  const localKeys = Object.keys(localVectorClock);

  // Check if client was out of sync before queuing
  for (const key of clientKeys) {
    if ((localVectorClock[key] || 0) > (clientVectorClock[key] || 0)) {
      return true; // Local clock is ahead, potential conflict
    }
  }

  return false;
}

/**
 * Resolve conflicts between offline and online data
 * Simple strategy: Server wins, but user is notified
 */
async function resolveConflicts(
  db: IDBDatabase,
  conflicts: PendingExercise[]
): Promise<void> {
  console.log('[OfflineSync] Resolving conflicts:', conflicts.length);

  for (const exercise of conflicts) {
    // In a real app, you might:
    // 1. Ask user to re-submit
    // 2. Merge changes (CRDT)
    // 3. Show conflict notification

    // For now, we'll just discard the conflicted item and notify user
    await removeFromIndexedDB(db, 'pendingExercises', exercise.id);

    console.log('[OfflineSync] Discarded conflicted exercise:', exercise.id);

    // You could dispatch an action here to show a conflict notification
    window.dispatchEvent(
      new CustomEvent('offline-conflict', {
        detail: { exerciseId: exercise.exerciseId },
      })
    );
  }
}

/**
 * Update vector clock for ordering causality
 */
function updateVectorClock(source: 'local' | 'server'): void {
  const clientId = getClientId();
  localVectorClock[clientId] = (localVectorClock[clientId] || 0) + 1;
  console.log('[OfflineSync] Vector clock updated:', localVectorClock);
}

/**
 * Get unique client ID (stored in localStorage)
 */
function getClientId(): string {
  let clientId = localStorage.getItem('clientId');
  if (!clientId) {
    clientId = `client-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    localStorage.setItem('clientId', clientId);
  }
  return clientId;
}

/**
 * Event handlers
 */

function handleOnline() {
  console.log('[OfflineSync] Back online!');
  syncPendingExercises();
}

function handleOffline() {
  console.log('[OfflineSync] Went offline');
  notifyOfflineMode(true);
}

function handleSWMessage(event: ExtendableMessageEvent) {
  console.log('[OfflineSync] Message from Service Worker:', event.data);

  if (event.data.type === 'SYNC_COMPLETE') {
    console.log('[OfflineSync] Service Worker sync complete');
    syncPendingExercises();
  }
}

/**
 * UI Notifications
 */

function notifyOfflineMode(offline: boolean) {
  window.dispatchEvent(
    new CustomEvent('offline-mode-changed', {
      detail: { offline },
    })
  );
}

function notifyOfflineQueue(count: number) {
  window.dispatchEvent(
    new CustomEvent('offline-queue-updated', {
      detail: { pendingCount: count },
    })
  );
}

/**
 * IndexedDB Operations
 */

async function openIndexedDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open('pathfinder', 1);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);

    request.onupgradeneeded = (event) => {
      const db = event.target.result;
      if (!db.objectStoreNames.contains('pendingExercises')) {
        db.createObjectStore('pendingExercises', { keyPath: 'id' });
      }
    };
  });
}

async function saveToIndexedDB(exercise: PendingExercise): Promise<void> {
  const db = await openIndexedDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('pendingExercises', 'readwrite');
    const store = tx.objectStore('pendingExercises');
    const request = store.add(exercise);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve();
  });
}

async function loadPendingExercises(): Promise<void> {
  try {
    const db = await openIndexedDB();
    const exercises = await getAllFromIndexedDB(db, 'pendingExercises');
    console.log('[OfflineSync] Loaded pending exercises:', exercises.length);

    if (exercises.length > 0 && navigator.onLine) {
      await syncPendingExercises();
    }
  } catch (error) {
    console.warn('[OfflineSync] Failed to load pending exercises:', error);
  }
}

async function getAllFromIndexedDB(
  db: IDBDatabase,
  storeName: string
): Promise<any[]> {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, 'readonly');
    const store = tx.objectStore(storeName);
    const request = store.getAll();

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);
  });
}

async function updateInIndexedDB(
  db: IDBDatabase,
  storeName: string,
  data: any
): Promise<void> {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, 'readwrite');
    const store = tx.objectStore(storeName);
    const request = store.put(data);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve();
  });
}

async function removeFromIndexedDB(
  db: IDBDatabase,
  storeName: string,
  id: string
): Promise<void> {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, 'readwrite');
    const store = tx.objectStore(storeName);
    const request = store.delete(id);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve();
  });
}

async function removeInIndexedDB(
  db: IDBDatabase,
  storeName: string,
  criteria: any
): Promise<void> {
  const all = await getAllFromIndexedDB(db, storeName);
  const toDelete = all.filter((item) =>
    Object.entries(criteria).every(([key, value]) => item[key] === value)
  );

  for (const item of toDelete) {
    await removeFromIndexedDB(db, storeName, item.id);
  }
}

/**
 * Utility: Generate unique ID
 */
function generateId(): string {
  return `exercise-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

console.log('[OfflineSync] Module loaded');
