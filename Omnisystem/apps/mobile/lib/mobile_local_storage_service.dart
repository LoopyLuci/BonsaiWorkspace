// PATHFINDER Mobile - Local Storage Service
// Hive-based local data persistence with encryption

import 'package:hive/hive.dart';

class LocalStorageService {

  LocalStorageService() {
    _userBox = Hive.box('user_data');
    _exercisesBox = Hive.box('exercises');
    _progressBox = Hive.box('progress');
    _syncQueueBox = Hive.box('sync_queue');
  }
  late Box _userBox;
  late Box _exercisesBox;
  late Box _progressBox;
  late Box _syncQueueBox;

  // ============================================================================
  // USER DATA
  // ============================================================================

  Future<void> saveUser({
    required String userId,
    required String email,
    required String firstName,
    required String lastName,
  }) async {
    await _userBox.putAll({
      'userId': userId,
      'email': email,
      'firstName': firstName,
      'lastName': lastName,
      'lastUpdated': DateTime.now().millisecondsSinceEpoch,
    });
  }

  Map? getUser() => {
      'userId': _userBox.get('userId'),
      'email': _userBox.get('email'),
      'firstName': _userBox.get('firstName'),
      'lastName': _userBox.get('lastName'),
    };

  // Auth token management
  Future<void> saveAuthToken(String token) async {
    await _userBox.put('authToken', token);
  }

  String? getAuthToken() => _userBox.get('authToken');

  Future<void> clearAuthToken() async {
    await _userBox.delete('authToken');
  }

  // ============================================================================
  // EXERCISES & LEARNING DATA
  // ============================================================================

  Future<void> saveExercise({
    required String exerciseId,
    required String skillId,
    required String skillName,
    required String question,
    required List<String> options,
    required int correctIndex,
    required String explanation,
    required int difficulty,
  }) async {
    await _exercisesBox.put('exercise_$exerciseId', {
      'exerciseId': exerciseId,
      'skillId': skillId,
      'skillName': skillName,
      'question': question,
      'options': options,
      'correctIndex': correctIndex,
      'explanation': explanation,
      'difficulty': difficulty,
      'savedAt': DateTime.now().millisecondsSinceEpoch,
    });
  }

  Map? getExercise(String exerciseId) => _exercisesBox.get('exercise_$exerciseId');

  List<Map> getAllExercises() => _exercisesBox.values.cast<Map>().toList();

  // ============================================================================
  // EXERCISE ATTEMPTS (Responses)
  // ============================================================================

  Future<void> saveAttempt({
    required String attemptId,
    required String exerciseId,
    required String skillId,
    required int selectedIndex,
    required bool isCorrect,
    required int timeTakenSeconds,
  }) async {
    await _exercisesBox.put('attempt_$attemptId', {
      'attemptId': attemptId,
      'exerciseId': exerciseId,
      'skillId': skillId,
      'selectedIndex': selectedIndex,
      'isCorrect': isCorrect,
      'timeTakenSeconds': timeTakenSeconds,
      'createdAt': DateTime.now().millisecondsSinceEpoch,
      'synced': false,
    });
  }

  List<Map> getPendingAttempts() => _exercisesBox.values
        .cast<Map>()
        .where((e) => e.containsKey('attemptId') && !(e['synced'] as bool? ?? false))
        .toList();

  Future<void> markAttemptSynced(String attemptId) async {
    final attempt = _exercisesBox.get('attempt_$attemptId');
    if (attempt != null) {
      attempt['synced'] = true;
      await _exercisesBox.put('attempt_$attemptId', attempt);
    }
  }

  // ============================================================================
  // PROGRESS DATA
  // ============================================================================

  Future<void> saveProgress({
    required String skillId,
    required String skillName,
    required double masteryPercent,
    required int exercisesAttempted,
    required int exercisesCorrect,
  }) async {
    await _progressBox.put('skill_$skillId', {
      'skillId': skillId,
      'skillName': skillName,
      'masteryPercent': masteryPercent,
      'exercisesAttempted': exercisesAttempted,
      'exercisesCorrect': exercisesCorrect,
      'lastUpdated': DateTime.now().millisecondsSinceEpoch,
    });
  }

  Map? getProgress(String skillId) => _progressBox.get('skill_$skillId');

  List<Map> getAllProgress() => _progressBox.values.cast<Map>().toList();

  double getOverallMastery() {
    final allProgress = getAllProgress();
    if (allProgress.isEmpty) return 0;
    final avgMastery = allProgress.fold<double>(
      0,
      (sum, p) => sum + (p['masteryPercent'] as double? ?? 0),
    );
    return avgMastery / allProgress.length;
  }

  // ============================================================================
  // ACHIEVEMENTS & GAMIFICATION
  // ============================================================================

  Future<void> saveAchievement({
    required String badgeId,
    required String badgeName,
    required String category,
    required int points,
  }) async {
    await _userBox.put('achievement_$badgeId', {
      'badgeId': badgeId,
      'badgeName': badgeName,
      'category': category,
      'points': points,
      'unlockedAt': DateTime.now().millisecondsSinceEpoch,
    });
  }

  List<Map> getAchievements() {
    final achievements = [];
    for (var i = 0; i < _userBox.length; i++) {
      final key = _userBox.keyAt(i);
      if (key.toString().startsWith('achievement_')) {
        achievements.add(_userBox.getAt(i));
      }
    }
    return achievements.cast<Map>();
  }

  Future<void> saveGamificationStats({
    required int totalPoints,
    required int level,
    required int rank,
  }) async {
    await _userBox.putAll({
      'totalPoints': totalPoints,
      'level': level,
      'leaderboardRank': rank,
      'statsUpdated': DateTime.now().millisecondsSinceEpoch,
    });
  }

  int getTotalPoints() => _userBox.get('totalPoints') ?? 0;
  int getLevel() => _userBox.get('level') ?? 1;
  int getLeaderboardRank() => _userBox.get('leaderboardRank') ?? 0;

  // ============================================================================
  // PREFERENCES & SETTINGS
  // ============================================================================

  Future<void> savePreferences({
    required bool notifyMastery,
    required bool notifyAlerts,
    required String emailFrequency,
    required bool quietHoursEnabled,
    required String quietHoursStart,
    required String quietHoursEnd,
    required String timezone,
  }) async {
    await _userBox.putAll({
      'notifyMastery': notifyMastery,
      'notifyAlerts': notifyAlerts,
      'emailFrequency': emailFrequency,
      'quietHoursEnabled': quietHoursEnabled,
      'quietHoursStart': quietHoursStart,
      'quietHoursEnd': quietHoursEnd,
      'timezone': timezone,
      'preferencesUpdated': DateTime.now().millisecondsSinceEpoch,
    });
  }

  Map getPreferences() => {
      'notifyMastery': _userBox.get('notifyMastery') ?? true,
      'notifyAlerts': _userBox.get('notifyAlerts') ?? true,
      'emailFrequency': _userBox.get('emailFrequency') ?? 'daily',
      'quietHoursEnabled': _userBox.get('quietHoursEnabled') ?? false,
      'quietHoursStart': _userBox.get('quietHoursStart') ?? '22:00',
      'quietHoursEnd': _userBox.get('quietHoursEnd') ?? '08:00',
      'timezone': _userBox.get('timezone') ?? 'UTC',
    };

  // ============================================================================
  // CACHE MANAGEMENT
  // ============================================================================

  Future<void> clearAll() async {
    await _userBox.clear();
    await _exercisesBox.clear();
    await _progressBox.clear();
    await _syncQueueBox.clear();
  }

  Future<void> clearExerciseCache() async {
    await _exercisesBox.clear();
  }

  Future<void> clearProgressCache() async {
    await _progressBox.clear();
  }

  int getStorageSize() => _userBox.length + _exercisesBox.length + _progressBox.length + _syncQueueBox.length;

  Future<void> deleteOldData(int daysOld) async {
    final cutoff = DateTime.now().subtract(Duration(days: daysOld)).millisecondsSinceEpoch;

    // Delete old exercises
    for (var i = _exercisesBox.length - 1; i >= 0; i--) {
      final exercise = _exercisesBox.getAt(i) as Map?;
      if (exercise != null && (exercise['savedAt'] as int? ?? 0) < cutoff) {
        await _exercisesBox.deleteAt(i);
      }
    }

    // Delete old progress
    for (var i = _progressBox.length - 1; i >= 0; i--) {
      final progress = _progressBox.getAt(i) as Map?;
      if (progress != null && (progress['lastUpdated'] as int? ?? 0) < cutoff) {
        await _progressBox.deleteAt(i);
      }
    }
  }
}
