import React, { createContext, useCallback, useState, useEffect } from 'react';
import { SyncState, ChangeLog, SyncConflict, Device } from '../types';
import * as SyncService from '../services/sync';

interface SyncContextType {
  syncState: SyncState;
  devices: Device[];
  triggerSync: () => Promise<void>;
  pauseSync: () => void;
  resumeSync: () => void;
  resolveConflict: (conflictId: string, resolution: 'local' | 'remote' | 'merged') => Promise<void>;
  getPendingChanges: () => ChangeLog[];
  getConflicts: () => SyncConflict[];
  clearConflicts: () => void;
  registerDevice: (name: string) => Promise<Device>;
  removeDevice: (deviceId: string) => Promise<void>;
  getDevices: () => Promise<Device[]>;
}

export const SyncContext = createContext<SyncContextType | undefined>(undefined);

interface SyncProviderProps {
  children: React.ReactNode;
  syncInterval?: number; // ms, default 3600000 (1 hour)
}

export const SyncProvider: React.FC<SyncProviderProps> = ({
  children,
  syncInterval = 3600000,
}) => {
  const [syncState, setSyncState] = useState<SyncState>({
    isSyncing: false,
    lastSync: null,
    nextSync: null,
    pendingChanges: 0,
    conflicts: [],
    status: 'idle',
  });

  const [devices, setDevices] = useState<Device[]>([]);
  const [pendingChanges, setPendingChanges] = useState<ChangeLog[]>([]);
  const [autoSyncTimer, setAutoSyncTimer] = useState<NodeJS.Timer | null>(null);

  // Load initial state
  useEffect(() => {
    const loadInitialState = async () => {
      try {
        const changes = await SyncService.getPendingChanges();
        setPendingChanges(changes);

        setSyncState(prev => ({
          ...prev,
          pendingChanges: changes.length,
        }));

        const deviceList = await SyncService.getDevices();
        setDevices(deviceList);

        const lastSyncTime = await SyncService.getLastSyncTime();
        if (lastSyncTime) {
          setSyncState(prev => ({
            ...prev,
            lastSync: new Date(lastSyncTime).toISOString(),
          }));
        }
      } catch (err) {
        console.error('Failed to load sync state:', err);
      }
    };

    loadInitialState();
  }, []);

  // Setup auto-sync timer
  useEffect(() => {
    const timer = setInterval(() => {
      setSyncState(prev => ({
        ...prev,
        nextSync: new Date(Date.now() + syncInterval).toISOString(),
      }));
    }, syncInterval);

    setAutoSyncTimer(timer);

    return () => {
      if (timer) clearInterval(timer);
    };
  }, [syncInterval]);

  const triggerSync = useCallback(async () => {
    if (syncState.isSyncing || syncState.status === 'paused') {
      return;
    }

    setSyncState(prev => ({
      ...prev,
      isSyncing: true,
      status: 'syncing',
    }));

    try {
      // Get local changes
      const changes = await SyncService.getPendingChanges();

      // Push changes to cloud
      const { conflicts } = await SyncService.pushChanges(changes);

      // Pull remote changes
      const remoteChanges = await SyncService.pullChanges();

      // Merge changes
      await SyncService.mergeChanges(remoteChanges);

      // Update state
      const remainingChanges = await SyncService.getPendingChanges();
      const lastSync = new Date().toISOString();
      const nextSync = new Date(Date.now() + syncInterval).toISOString();

      setSyncState(prev => ({
        ...prev,
        isSyncing: false,
        status: 'idle',
        lastSync,
        nextSync,
        pendingChanges: remainingChanges.length,
        conflicts: conflicts || [],
      }));

      setPendingChanges(remainingChanges);

      // Save sync time
      await SyncService.setLastSyncTime(lastSync);
    } catch (err) {
      setSyncState(prev => ({
        ...prev,
        isSyncing: false,
        status: 'error',
      }));

      console.error('Sync failed:', err);
      throw err;
    }
  }, [syncState.isSyncing, syncState.status, syncInterval]);

  const pauseSync = useCallback(() => {
    setSyncState(prev => ({
      ...prev,
      status: 'paused',
    }));

    if (autoSyncTimer) {
      clearInterval(autoSyncTimer);
      setAutoSyncTimer(null);
    }
  }, [autoSyncTimer]);

  const resumeSync = useCallback(() => {
    setSyncState(prev => ({
      ...prev,
      status: 'idle',
    }));

    const timer = setInterval(() => {
      triggerSync().catch(err => console.error('Auto-sync error:', err));
    }, syncInterval);

    setAutoSyncTimer(timer);
  }, [syncInterval, triggerSync]);

  const resolveConflict = useCallback(
    async (conflictId: string, resolution: 'local' | 'remote' | 'merged') => {
      try {
        await SyncService.resolveConflict(conflictId, resolution);

        setSyncState(prev => ({
          ...prev,
          conflicts: prev.conflicts.filter(c => c.id !== conflictId),
        }));
      } catch (err) {
        console.error('Failed to resolve conflict:', err);
        throw err;
      }
    },
    []
  );

  const getPendingChanges = useCallback(() => pendingChanges, [pendingChanges]);

  const getConflicts = useCallback(
    () => syncState.conflicts,
    [syncState.conflicts]
  );

  const clearConflicts = useCallback(async () => {
    try {
      for (const conflict of syncState.conflicts) {
        await SyncService.resolveConflict(conflict.id, 'local');
      }

      setSyncState(prev => ({
        ...prev,
        conflicts: [],
      }));
    } catch (err) {
      console.error('Failed to clear conflicts:', err);
      throw err;
    }
  }, [syncState.conflicts]);

  const registerDevice = useCallback(
    async (name: string) => {
      try {
        const device = await SyncService.registerDevice(name);
        setDevices(prev => [...prev, device]);
        return device;
      } catch (err) {
        console.error('Failed to register device:', err);
        throw err;
      }
    },
    []
  );

  const removeDevice = useCallback(
    async (deviceId: string) => {
      try {
        await SyncService.removeDevice(deviceId);
        setDevices(prev => prev.filter(d => d.id !== deviceId));
      } catch (err) {
        console.error('Failed to remove device:', err);
        throw err;
      }
    },
    []
  );

  const getDevices = useCallback(async () => {
    try {
      const deviceList = await SyncService.getDevices();
      setDevices(deviceList);
      return deviceList;
    } catch (err) {
      console.error('Failed to get devices:', err);
      throw err;
    }
  }, []);

  const value: SyncContextType = {
    syncState,
    devices,
    triggerSync,
    pauseSync,
    resumeSync,
    resolveConflict,
    getPendingChanges,
    getConflicts,
    clearConflicts,
    registerDevice,
    removeDevice,
    getDevices,
  };

  return (
    <SyncContext.Provider value={value}>
      {children}
    </SyncContext.Provider>
  );
};
