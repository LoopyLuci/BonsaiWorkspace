import 'package:dio/dio.dart';
import '../models/user.dart';
import '../models/skill.dart';

class ApiService {

  ApiService() {
    _dio = Dio(BaseOptions(
      baseUrl: baseUrl,
      connectTimeout: const Duration(seconds: 10),
      receiveTimeout: const Duration(seconds: 10),
    ));

    _dio.interceptors.add(
      InterceptorsWrapper(
        onRequest: (options, handler) {
          if (_token != null) {
            options.headers['Authorization'] = 'Bearer $_token';
          }
          return handler.next(options);
        },
      ),
    );
  }
  static const String baseUrl = 'http://localhost:8000/api/v1';
  late Dio _dio;
  String? _token;

  Future<AuthResponse> login(String email, String password) async {
    final response = await _dio.post('/auth/login', data: {
      'email': email,
      'password': password,
    });
    final authResponse = AuthResponse.fromJson(response.data);
    _token = authResponse.token;
    return authResponse;
  }

  Future<List<Skill>> fetchSkills() async {
    final response = await _dio.get('/skills');
    return (response.data as List)
        .map((e) => Skill.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<Skill> fetchSkill(String id) async {
    final response = await _dio.get('/skills/$id');
    return Skill.fromJson(response.data);
  }

  Future<List<Exercise>> fetchExercises() async {
    final response = await _dio.get('/exercises');
    return (response.data as List)
        .map((e) => Exercise.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<Exercise> fetchExercise(String id) async {
    final response = await _dio.get('/exercises/$id');
    return Exercise.fromJson(response.data);
  }

  Future<Map<String, dynamic>> submitAttempt(
    String exerciseId,
    String answer,
  ) async {
    final response = await _dio.post('/exercises/attempts', data: {
      'exercise_id': exerciseId,
      'answer': answer,
    });
    return response.data;
  }

  Future<List<SkillProgress>> fetchProgress(String userId) async {
    final response = await _dio.get('/progress/user/$userId');
    return (response.data as List)
        .map((e) => SkillProgress.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<Map<String, dynamic>> search(String query) async {
    final response = await _dio.get('/search', queryParameters: {
      'query': query,
    });
    return response.data;
  }

  Future<Map<String, dynamic>> fetchMetrics(String userId) async {
    final response = await _dio.get('/analytics/user/$userId/metrics');
    return response.data;
  }

  Future<Map<String, dynamic>> fetchRecommendations(String userId) async {
    final response = await _dio.get('/personalization/user/$userId/recommendations');
    return response.data;
  }

  Future<Map<String, dynamic>> getClassroomStats(String classroomId) async {
    final response = await _dio.get('/analytics/classroom/$classroomId/stats');
    return response.data;
  }

  void setToken(String token) {
    _token = token;
  }
}
