// PATHFINDER Mobile - Offline Sync Service
// Local-first sync with CRDT (Conflict-free Replicated Data Types)

import 'package:hive/hive.dart';
import 'api_service.dart';
import 'local_storage_service.dart';
import 'dart:convert';

class OfflineSyncService {

  OfflineSyncService(this.apiService, this.localStorageService);
  final ApiService apiService;
  final LocalStorageService localStorageService;
  final _syncBox = Hive.box('sync_queue');
  final _exercisesBox = Hive.box('exercises');
  final _progressBox = Hive.box('progress');

  bool isSyncing = false;
  int pendingChanges = 0;

  // Queue operation for later sync
  Future<void> queueOperation({
    required String type, // 'exercise_attempt', 'goal_update', 'preference_change'
    required String endpoint,
    required dynamic payload,
    required String method, // 'POST', 'PUT', 'DELETE'
  }) async {
    final operation = {
      'id': _generateId(),
      'timestamp': DateTime.now().millisecondsSinceEpoch,
      'type': type,
      'endpoint': endpoint,
      'payload': payload,
      'method': method,
      'synced': false,
      'retries': 0,
    };

    await _syncBox.add(operation);
    pendingChanges = _syncBox.values.where((op) => !(op['synced'] as bool)).length;
  }

  // Sync all pending operations
  Future<void> syncAll() async {
    if (isSyncing || !apiService.isOnline) return;

    isSyncing = true;
    final operations = List<Map>.from(_syncBox.values.where((op) => !(op['synced'] as bool)).toList());

    for (final operation in operations) {
      try {
        await _syncOperation(operation);
        await _markSynced(operation['id']);
      } catch (e) {
        await _incrementRetries(operation['id']);
        if (operation['retries'] >= 3) {
          // Give up after 3 retries
          await _markSynced(operation['id']);
        }
      }
    }

    pendingChanges = _syncBox.values.where((op) => !(op['synced'] as bool)).length;
    isSyncing = false;
  }

  Future<void> _syncOperation(Map operation) async {
    final method = operation['method'] as String;
    final endpoint = operation['endpoint'] as String;
    final payload = operation['payload'];

    switch (method) {
      case 'POST':
        await apiService.post(endpoint, payload);
        break;
      case 'PUT':
        await apiService.put(endpoint, payload);
        break;
      case 'DELETE':
        await apiService.delete(endpoint);
        break;
    }
  }

  Future<void> _markSynced(dynamic key) async {
    final operation = _syncBox.get(key);
    if (operation != null) {
      operation['synced'] = true;
      operation['syncedAt'] = DateTime.now().millisecondsSinceEpoch;
      await _syncBox.put(key, operation);
    }
  }

  Future<void> _incrementRetries(dynamic key) async {
    final operation = _syncBox.get(key);
    if (operation != null) {
      operation['retries'] = (operation['retries'] as int) + 1;
      await _syncBox.put(key, operation);
    }
  }

  // CRDT: Last-write-wins merge strategy
  Future<void> mergeExerciseAttempt({
    required String exerciseId,
    required Map attempt,
    required int clientTimestamp,
  }) async {
    final key = 'exercise_$exerciseId';
    final existing = _exercisesBox.get(key) as Map?;

    if (existing == null) {
      // First write
      await _exercisesBox.put(key, {
        ...attempt,
        'clientTimestamp': clientTimestamp,
        'serverId': null,
        'synced': false,
      });
    } else {
      final existingTimestamp = (existing['clientTimestamp'] as int?) ?? 0;
      if (clientTimestamp > existingTimestamp) {
        // Newer version wins
        existing['clientTimestamp'] = clientTimestamp;
        existing.addAll(attempt);
        existing['synced'] = false;
        await _exercisesBox.put(key, existing);
      }
      // Older version ignored (CRDT conflict resolution)
    }

    // Queue for sync
    await queueOperation(
      type: 'exercise_attempt',
      endpoint: '/v1/exercises/$exerciseId/attempt',
      payload: attempt,
      method: 'POST',
    );
  }

  // Merge progress updates
  Future<void> mergeProgress({
    required String skillId,
    required Map progress,
    required int clientTimestamp,
  }) async {
    final key = 'progress_$skillId';
    final existing = _progressBox.get(key) as Map?;

    if (existing == null) {
      await _progressBox.put(key, {
        ...progress,
        'clientTimestamp': clientTimestamp,
        'synced': false,
      });
    } else {
      final existingTimestamp = (existing['clientTimestamp'] as int?) ?? 0;
      if (clientTimestamp > existingTimestamp) {
        existing['clientTimestamp'] = clientTimestamp;
        existing.addAll(progress);
        existing['synced'] = false;
        await _progressBox.put(key, existing);
      }
    }

    await queueOperation(
      type: 'progress_update',
      endpoint: '/v1/progress/$skillId',
      payload: progress,
      method: 'PUT',
    );
  }

  // Get all pending changes count
  int getPendingChanges() => _syncBox.values.where((op) => !(op['synced'] as bool)).length;

  // Clear completed operations (keep for 7 days)
  Future<void> cleanup() async {
    final sevenDaysAgo = DateTime.now().subtract(const Duration(days: 7)).millisecondsSinceEpoch;
    final keysToDelete = [];

    for (var i = 0; i < _syncBox.length; i++) {
      final operation = _syncBox.getAt(i) as Map;
      if (operation['synced'] == true &&
          (operation['syncedAt'] as int?) ?? 0 < sevenDaysAgo) {
        keysToDelete.add(_syncBox.keyAt(i));
      }
    }

    for (final key in keysToDelete) {
      await _syncBox.delete(key);
    }
  }

  String _generateId() => '${DateTime.now().millisecondsSinceEpoch}_${(0...9999).random()}';
}
