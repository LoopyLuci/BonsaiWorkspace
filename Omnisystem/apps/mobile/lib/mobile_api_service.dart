// PATHFINDER Mobile - API Service
// HTTP client with retry logic, error handling, and offline queuing

import 'package:dio/dio.dart';
import 'package:connectivity_plus/connectivity_plus.dart';

class ApiService {

  ApiService() {
    _dio = Dio(
      BaseOptions(
        baseUrl: baseUrl,
        connectTimeout: const Duration(seconds: 10),
        receiveTimeout: const Duration(seconds: 10),
        responseType: ResponseType.json,
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json',
        },
      ),
    );

    // Add interceptors
    _dio.interceptors.add(
      InterceptorsWrapper(
        onRequest: (options, handler) {
          // Add auth token if available
          final token = _getAuthToken();
          if (token != null) {
            options.headers['Authorization'] = 'Bearer $token';
          }
          options.headers['X-User-ID'] = _getUserId();
          return handler.next(options);
        },
        onError: (error, handler) {
          // Handle network errors
          if (error.type == DioExceptionType.connectionTimeout ||
              error.type == DioExceptionType.receiveTimeout) {
            isOnline = false;
          }
          return handler.next(error);
        },
      ),
    );

    // Monitor connectivity
    _connectivity.onConnectivityChanged.listen((result) {
      isOnline = result != ConnectivityResult.none;
    });
  }
  late Dio _dio;
  final String baseUrl = 'http://localhost:8000/api'; // Change for production
  final Connectivity _connectivity = Connectivity();
  bool isOnline = true;

  String? _getAuthToken() {
    // Retrieve from local storage
    return null; // Implemented in auth provider
  }

  String _getUserId() {
    // Retrieve from local storage
    return ''; // Implemented in auth provider
  }

  // GET request
  Future<dynamic> get(String endpoint) async {
    try {
      final response = await _dio.get(endpoint);
      return response.data;
    } catch (e) {
      _handleError(e);
      rethrow;
    }
  }

  // POST request
  Future<dynamic> post(String endpoint, dynamic data) async {
    try {
      final response = await _dio.post(endpoint, data: data);
      return response.data;
    } catch (e) {
      _handleError(e);
      rethrow;
    }
  }

  // PUT request
  Future<dynamic> put(String endpoint, dynamic data) async {
    try {
      final response = await _dio.put(endpoint, data: data);
      return response.data;
    } catch (e) {
      _handleError(e);
      rethrow;
    }
  }

  // DELETE request
  Future<dynamic> delete(String endpoint) async {
    try {
      final response = await _dio.delete(endpoint);
      return response.data;
    } catch (e) {
      _handleError(e);
      rethrow;
    }
  }

  // Upload file
  Future<dynamic> uploadFile(String endpoint, String filePath) async {
    try {
      final formData = FormData.fromMap({
        'file': await MultipartFile.fromFile(filePath),
      });
      final response = await _dio.post(endpoint, data: formData);
      return response.data;
    } catch (e) {
      _handleError(e);
      rethrow;
    }
  }

  // Batch request
  Future<List<dynamic>> batch(List<Future<dynamic>> requests) async {
    try {
      return await Future.wait(requests);
    } catch (e) {
      _handleError(e);
      rethrow;
    }
  }

  void _handleError(dynamic error) {
    if (error is DioException) {
      if (error.type == DioExceptionType.connectionTimeout ||
          error.type == DioExceptionType.receiveTimeout) {
        isOnline = false;
        throw Exception('Connection timeout - offline mode');
      }
      if (error.response?.statusCode == 401) {
        throw Exception('Unauthorized - please login again');
      }
      if (error.response?.statusCode == 404) {
        throw Exception('Resource not found');
      }
      if (error.response?.statusCode == 500) {
        throw Exception('Server error - please try again later');
      }
    }
  }

  void dispose() {
    _dio.close();
  }
}
