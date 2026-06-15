// PATHFINDER Mobile - State Management Providers
// Using Provider package for reactive state management

import 'package:flutter/material.dart';
import 'services/api_service.dart';
import 'services/local_storage_service.dart';
import 'services/offline_sync_service.dart';

// ============================================================================
// AUTH PROVIDER
// ============================================================================

class AuthProvider extends ChangeNotifier {

  AuthProvider(this._storage, this._api) {
    _initialize();
  }
  final LocalStorageService _storage;
  final ApiService _api;

  String? _userId;
  String? _email;
  String? _firstName;
  String? _lastName;
  String? _authToken;
  bool _isLoading = true;
  String? _error;

  String? get userId => _userId;
  String? get email => _email;
  String? get firstName => _firstName;
  String? get lastName => _lastName;
  bool get isAuthenticated => _authToken != null;
  bool get isLoading => _isLoading;
  String? get error => _error;

  void _initialize() async {
    _isLoading = true;
    notifyListeners();

    final token = _storage.getAuthToken();
    if (token != null) {
      _authToken = token;
      final user = _storage.getUser();
      if (user != null) {
        _userId = user['userId'];
        _email = user['email'];
        _firstName = user['firstName'];
        _lastName = user['lastName'];
      }
    }

    _isLoading = false;
    notifyListeners();
  }

  Future<bool> login(String email, String password) async {
    try {
      _isLoading = true;
      _error = null;
      notifyListeners();

      final response = await _api.post('/v1/auth/login', {
        'email': email,
        'password': password,
      });

      _authToken = response['token'];
      _userId = response['user']['id'];
      _email = response['user']['email'];
      _firstName = response['user']['first_name'];
      _lastName = response['user']['last_name'];

      // Save locally
      await _storage.saveAuthToken(_authToken!);
      await _storage.saveUser(
        userId: _userId!,
        email: _email!,
        firstName: _firstName!,
        lastName: _lastName!,
      );

      _isLoading = false;
      notifyListeners();
      return true;
    } catch (e) {
      _error = e.toString();
      _isLoading = false;
      notifyListeners();
      return false;
    }
  }

  Future<bool> register(String email, String password, String firstName, String lastName) async {
    try {
      _isLoading = true;
      _error = null;
      notifyListeners();

      final response = await _api.post('/v1/auth/register', {
        'email': email,
        'password': password,
        'first_name': firstName,
        'last_name': lastName,
      });

      _authToken = response['token'];
      _userId = response['user']['id'];
      _email = response['user']['email'];
      _firstName = response['user']['first_name'];
      _lastName = response['user']['last_name'];

      await _storage.saveAuthToken(_authToken!);
      await _storage.saveUser(
        userId: _userId!,
        email: _email!,
        firstName: _firstName!,
        lastName: _lastName!,
      );

      _isLoading = false;
      notifyListeners();
      return true;
    } catch (e) {
      _error = e.toString();
      _isLoading = false;
      notifyListeners();
      return false;
    }
  }

  Future<void> logout() async {
    _authToken = null;
    _userId = null;
    _email = null;
    _firstName = null;
    _lastName = null;
    await _storage.clearAuthToken();
    notifyListeners();
  }
}

// ============================================================================
// LEARNING PROVIDER
// ============================================================================

class LearningProvider extends ChangeNotifier {

  LearningProvider(this._api, this._storage) {
    _loadLocalData();
  }
  final ApiService _api;
  final LocalStorageService _storage;

  List<Map> _exercises = [];
  List<Map> _progress = [];
  double _overallMastery = 0;
  int _totalPoints = 0;
  int _level = 1;
  bool _isLoading = false;
  String? _error;

  List<Map> get exercises => _exercises;
  List<Map> get progress => _progress;
  double get overallMastery => _overallMastery;
  int get totalPoints => _totalPoints;
  int get level => _level;
  bool get isLoading => _isLoading;
  String? get error => _error;

  void _loadLocalData() {
    _progress = _storage.getAllProgress();
    _overallMastery = _storage.getOverallMastery();
    _totalPoints = _storage.getTotalPoints();
    _level = _storage.getLevel();
    notifyListeners();
  }

  Future<void> fetchExercises({String? skillId}) async {
    try {
      _isLoading = true;
      _error = null;
      notifyListeners();

      final endpoint = skillId != null
          ? '/v1/exercises?skill_id=$skillId'
          : '/v1/exercises?limit=10';

      final response = await _api.get(endpoint);
      _exercises = List<Map>.from(response['exercises'] ?? []);

      // Cache locally
      for (final exercise in _exercises) {
        await _storage.saveExercise(
          exerciseId: exercise['id'],
          skillId: exercise['skill_id'],
          skillName: exercise['skill_name'],
          question: exercise['question'],
          options: List<String>.from(exercise['options'] ?? []),
          correctIndex: exercise['correct_index'],
          explanation: exercise['explanation'] ?? '',
          difficulty: exercise['difficulty'] ?? 1,
        );
      }

      _isLoading = false;
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      // Fall back to local cache
      _exercises = _storage.getAllExercises();
      _isLoading = false;
      notifyListeners();
    }
  }

  Future<void> submitAttempt({
    required String exerciseId,
    required String skillId,
    required int selectedIndex,
    required int timeTakenSeconds,
    required OfflineSyncService syncService,
  }) async {
    try {
      _isLoading = true;
      _error = null;
      notifyListeners();

      final exercise = _exercises.firstWhere((e) => e['id'] == exerciseId, orElse: () => {});
      final isCorrect = exercise['correct_index'] == selectedIndex;

      final attemptId = DateTime.now().millisecondsSinceEpoch.toString();

      // Save locally immediately for offline support
      await _storage.saveAttempt(
        attemptId: attemptId,
        exerciseId: exerciseId,
        skillId: skillId,
        selectedIndex: selectedIndex,
        isCorrect: isCorrect,
        timeTakenSeconds: timeTakenSeconds,
      );

      // Queue for sync
      await syncService.queueOperation(
        type: 'exercise_attempt',
        endpoint: '/v1/exercises/$exerciseId/attempt',
        payload: {
          'attempt_id': attemptId,
          'exercise_id': exerciseId,
          'skill_id': skillId,
          'selected_index': selectedIndex,
          'is_correct': isCorrect,
          'time_taken_seconds': timeTakenSeconds,
        },
        method: 'POST',
      );

      _isLoading = false;
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      _isLoading = false;
      notifyListeners();
    }
  }

  Future<void> fetchProgress() async {
    try {
      _isLoading = true;
      _error = null;
      notifyListeners();

      final response = await _api.get('/v1/progress');
      final progressList = response['progress'] as List? ?? [];

      for (final p in progressList) {
        await _storage.saveProgress(
          skillId: p['skill_id'],
          skillName: p['skill_name'],
          masteryPercent: (p['mastery_percent'] as num).toDouble(),
          exercisesAttempted: p['exercises_attempted'] ?? 0,
          exercisesCorrect: p['exercises_correct'] ?? 0,
        );
      }

      _loadLocalData();
      _isLoading = false;
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      _loadLocalData();
      _isLoading = false;
      notifyListeners();
    }
  }
}

// ============================================================================
// SYNC PROVIDER
// ============================================================================

class SyncProvider extends ChangeNotifier {

  SyncProvider(this._syncService);
  final OfflineSyncService _syncService;

  bool _isOnline = true;
  bool _isSyncing = false;
  int _pendingChanges = 0;

  bool get isOnline => _isOnline;
  bool get isSyncing => _isSyncing;
  int get pendingChanges => _pendingChanges;

  void startBackgroundSync() {
    // Sync immediately
    _sync();

    // Sync every 30 seconds
    Future.delayed(const Duration(seconds: 30), () {
      if (_isOnline && !_isSyncing) {
        _sync();
      }
      startBackgroundSync();
    });
  }

  Future<void> _sync() async {
    if (_isSyncing) return;

    _isSyncing = true;
    notifyListeners();

    await _syncService.syncAll();

    _pendingChanges = _syncService.getPendingChanges();
    _isSyncing = false;
    notifyListeners();
  }

  void setOnlineStatus(bool online) {
    _isOnline = online;
    if (online && !_isSyncing) {
      _sync();
    }
    notifyListeners();
  }

  Future<void> syncNow() async {
    await _sync();
  }
}
