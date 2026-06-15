// PATHFINDER Service Worker
// Enables offline support with smart caching strategy

const CACHE_NAME = 'pathfinder-v1';
const OFFLINE_URL = '/offline.html';

// Assets to cache on install (critical app resources)
const CRITICAL_ASSETS = [
  '/',
  '/index.html',
  '/manifest.json',
  '/offline.html',
];

// Cache strategies:
// 1. Network-first for API calls (always try fresh, fall back to cache)
// 2. Cache-first for static assets (use cache, update in background)
// 3. Stale-while-revalidate for exercises (serve cached, update background)

// ============================================================================
// INSTALL EVENT - Cache critical assets
// ============================================================================

self.addEventListener('install', (event) => {
  console.log('[ServiceWorker] Installing...');

  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      console.log('[ServiceWorker] Caching critical assets');
      return cache.addAll(CRITICAL_ASSETS);
    })
  );

  // Activate immediately
  self.skipWaiting();
});

// ============================================================================
// ACTIVATE EVENT - Clean up old caches
// ============================================================================

self.addEventListener('activate', (event) => {
  console.log('[ServiceWorker] Activating...');

  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames.map((cacheName) => {
          if (cacheName !== CACHE_NAME) {
            console.log('[ServiceWorker] Deleting old cache:', cacheName);
            return caches.delete(cacheName);
          }
        })
      );
    })
  );

  self.clients.claim();
});

// ============================================================================
// FETCH EVENT - Implement caching strategies
// ============================================================================

self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Skip non-GET requests
  if (request.method !== 'GET') {
    return;
  }

  // Skip external requests
  if (!url.origin.includes(location.origin)) {
    return;
  }

  // API calls - Network-first strategy
  if (url.pathname.includes('/v1/')) {
    return event.respondWith(networkFirst(request));
  }

  // Static assets (JS, CSS, images) - Cache-first strategy
  if (
    request.destination === 'script' ||
    request.destination === 'style' ||
    request.destination === 'image'
  ) {
    return event.respondWith(cacheFirst(request));
  }

  // HTML pages - Network-first with offline fallback
  if (request.destination === 'document') {
    return event.respondWith(networkFirstWithFallback(request));
  }

  // Default - Network-first
  event.respondWith(networkFirst(request));
});

// ============================================================================
// CACHING STRATEGIES
// ============================================================================

/**
 * Network-first: Try network, fall back to cache
 * Used for: API calls, dynamic content
 */
async function networkFirst(request) {
  try {
    const networkResponse = await fetch(request);

    // Cache successful responses
    if (networkResponse.ok) {
      const cache = await caches.open(CACHE_NAME);
      cache.put(request, networkResponse.clone());
    }

    return networkResponse;
  } catch (error) {
    console.log('[ServiceWorker] Network failed, trying cache:', request.url);

    const cachedResponse = await caches.match(request);
    if (cachedResponse) {
      return cachedResponse;
    }

    // Return offline page for document requests
    if (request.destination === 'document') {
      return caches.match(OFFLINE_URL);
    }

    // Return error response
    return new Response('Offline - Resource unavailable', {
      status: 503,
      statusText: 'Service Unavailable',
    });
  }
}

/**
 * Cache-first: Use cache, update in background
 * Used for: Static assets (JS, CSS, images)
 */
async function cacheFirst(request) {
  const cachedResponse = await caches.match(request);

  if (cachedResponse) {
    // Update cache in background (no await)
    fetch(request).then((networkResponse) => {
      if (networkResponse.ok) {
        const cache = caches.open(CACHE_NAME).then((c) => {
          c.put(request, networkResponse);
        });
      }
    });

    return cachedResponse;
  }

  // Not in cache, fetch from network
  try {
    const networkResponse = await fetch(request);

    if (networkResponse.ok) {
      const cache = await caches.open(CACHE_NAME);
      cache.put(request, networkResponse.clone());
    }

    return networkResponse;
  } catch (error) {
    console.log('[ServiceWorker] Offline:', request.url);
    return new Response('Offline', { status: 503 });
  }
}

/**
 * Network-first with offline fallback
 * Used for: HTML pages
 */
async function networkFirstWithFallback(request) {
  try {
    const networkResponse = await fetch(request);

    if (networkResponse.ok) {
      const cache = await caches.open(CACHE_NAME);
      cache.put(request, networkResponse.clone());
    }

    return networkResponse;
  } catch (error) {
    const cachedResponse = await caches.match(request);
    if (cachedResponse) {
      return cachedResponse;
    }

    return caches.match(OFFLINE_URL);
  }
}

// ============================================================================
// BACKGROUND SYNC - Sync offline actions when back online
// ============================================================================

self.addEventListener('sync', (event) => {
  if (event.tag === 'sync-exercises') {
    console.log('[ServiceWorker] Syncing offline exercises...');

    event.waitUntil(
      (async () => {
        try {
          // Get pending exercises from IndexedDB
          const db = await openIndexedDB();
          const pendingExercises = await getPendingExercises(db);

          // Submit each pending exercise
          for (const exercise of pendingExercises) {
            try {
              await fetch('/v1/learners/' + exercise.userId + '/exercises/' + exercise.exerciseId + '/attempt', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(exercise.data),
              });

              // Remove from pending after successful sync
              await removePendingExercise(db, exercise.id);
            } catch (error) {
              console.error('[ServiceWorker] Failed to sync exercise:', error);
              // Will retry on next sync
            }
          }

          console.log('[ServiceWorker] Sync complete');
          notifyClients({ type: 'SYNC_COMPLETE' });
        } catch (error) {
          console.error('[ServiceWorker] Sync failed:', error);
        }
      })()
    );
  }
});

// ============================================================================
// MESSAGE HANDLING - Communicate with app
// ============================================================================

self.addEventListener('message', (event) => {
  if (event.data.type === 'SKIP_WAITING') {
    self.skipWaiting();
  }

  if (event.data.type === 'CLEAR_CACHE') {
    caches.delete(CACHE_NAME);
  }

  if (event.data.type === 'SYNC_OFFLINE') {
    if (self.registration.sync) {
      self.registration.sync.register('sync-exercises');
    }
  }
});

// ============================================================================
// HELPERS
// ============================================================================

function notifyClients(message) {
  self.clients.matchAll().then((clients) => {
    clients.forEach((client) => {
      client.postMessage(message);
    });
  });
}

// Simplified IndexedDB helpers (would be imported from main app in production)
async function openIndexedDB() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open('pathfinder', 1);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);

    request.onupgradeneeded = (event) => {
      const db = event.target.result;
      if (!db.objectStoreNames.contains('pendingExercises')) {
        db.createObjectStore('pendingExercises', { keyPath: 'id', autoIncrement: true });
      }
    };
  });
}

async function getPendingExercises(db) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction('pendingExercises', 'readonly');
    const store = tx.objectStore('pendingExercises');
    const request = store.getAll();

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);
  });
}

async function removePendingExercise(db, id) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction('pendingExercises', 'readwrite');
    const store = tx.objectStore('pendingExercises');
    const request = store.delete(id);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve();
  });
}

console.log('[ServiceWorker] Loaded and ready');
