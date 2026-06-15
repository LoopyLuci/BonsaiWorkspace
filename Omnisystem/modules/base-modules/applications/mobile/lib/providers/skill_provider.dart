import 'package:flutter/foundation.dart';
import '../models/skill.dart';
import '../services/api_service.dart';

class SkillProvider extends ChangeNotifier {
  List<Skill> _skills = [];
  List<SkillProgress> _progress = [];
  bool _isLoading = false;
  String? _error;

  List<Skill> get skills => _skills;
  List<SkillProgress> get progress => _progress;
  bool get isLoading => _isLoading;
  String? get error => _error;

  final ApiService _apiService = ApiService();

  Future<void> fetchSkills() async {
    _isLoading = true;
    _error = null;
    notifyListeners();

    try {
      _skills = await _apiService.fetchSkills();
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    } finally {
      _isLoading = false;
      notifyListeners();
    }
  }

  Future<void> fetchProgress(String userId) async {
    _isLoading = true;
    _error = null;
    notifyListeners();

    try {
      _progress = await _apiService.fetchProgress(userId);
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    } finally {
      _isLoading = false;
      notifyListeners();
    }
  }
}
