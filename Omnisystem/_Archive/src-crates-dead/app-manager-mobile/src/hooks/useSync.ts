import { useContext } from 'react';
import { SyncContext } from '../context/SyncContext';

/**
 * Custom hook for accessing sync context
 */
export function useSync() {
  const context = useContext(SyncContext);

  if (!context) {
    throw new Error('useSync must be used within SyncProvider');
  }

  return context;
}

/**
 * Hook for sync status
 */
export function useSyncStatus() {
  const { syncState } = useSync();

  return {
    isSyncing: syncState.isSyncing,
    status: syncState.status,
    lastSync: syncState.lastSync,
    nextSync: syncState.nextSync,
    pendingChanges: syncState.pendingChanges,
  };
}

/**
 * Hook for sync conflicts
 */
export function useSyncConflicts() {
  const { syncState, resolveConflict, getConflicts, clearConflicts } = useSync();

  return {
    conflicts: getConflicts(),
    hasConflicts: syncState.conflicts.length > 0,
    resolve: resolveConflict,
    clear: clearConflicts,
  };
}

/**
 * Hook for device management
 */
export function useDevices() {
  const { devices, registerDevice, removeDevice, getDevices } = useSync();

  return {
    devices,
    count: devices.length,
    register: registerDevice,
    remove: removeDevice,
    refresh: getDevices,
    currentDevice: devices.find(d => d.isCurrentDevice),
  };
}

/**
 * Hook for manual sync
 */
export function useManualSync() {
  const { syncState, triggerSync } = useSync();

  return {
    isSyncing: syncState.isSyncing,
    sync: triggerSync,
  };
}

/**
 * Hook for pause/resume sync
 */
export function useSyncControl() {
  const { syncState, pauseSync, resumeSync } = useSync();

  return {
    status: syncState.status,
    isPaused: syncState.status === 'paused',
    pause: pauseSync,
    resume: resumeSync,
  };
}
